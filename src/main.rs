#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;

use crate::db::DBInterface;
use crate::handler::messages;
use crate::handler::service::ServiceController;
use crate::handler::user::UserService;
use crate::settings::Settings;

use actix;
use actix::prelude::*;
#[cfg(target_os = "linux")]
use actix_rt::signal::unix::signal;
#[cfg(target_os = "linux")]
use actix_rt::signal::unix::SignalKind;
use clap::{App, Arg};
use env_logger;
use failure::Fallible;

mod crypto;
mod db;
mod handler;
mod settings;
mod web;

fn main() -> Fallible<()> {
    if std::env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        std::env::set_var(
            env_logger::DEFAULT_FILTER_ENV,
            #[cfg(debug_assertions)]
            "service_daemon=trace,actix_web=info,actix_server=info",
            #[cfg(not(debug_assertions))]
            "service_daemon=info,actix_web=warn,actix_server=info",
        );
    }
    env_logger::init();

    let app = App::new("Service-Daemon")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Service management with remote control, permissions and logging.")
        .arg(
            Arg::with_name("configtest")
                .short("t")
                .long("test-config")
                .help("Test configuration"),
        )
        .arg(
            Arg::with_name("cleanup")
                .long("cleanup")
                .value_name("max age date 2020-01-01")
                .takes_value(true)
                .help("Cleanup database from outdated entries"),
        )
        .get_matches();

    let settings = match settings::Settings::new() {
        Err(e) => {
            error!("Error loading configuration {}", e);
            info!("Please check your config file. If upgrading from an earlier version be sure to check for new required fields in config/template.toml");
            return Err(e.into());
        }
        Ok(v) => v,
    };
    trace!("{:#?}", settings);

    if app.is_present("cleanup") {
        if let Some(v) = app.value_of("cleanup") {
            if let Ok(v) = chrono::NaiveDate::parse_from_str(v,"%Y-%m-%d") {
                let dt = v.and_hms_milli(0, 0, 0, 1);
                db::DB.cleanup(dt.timestamp_millis())?;
            } else {
                error!("Invalid date value {}!",v);
            }
        } else {
            error!("Missing max age for cleanup!")
        }
        
    }

    if !app.is_present("configtest") && !app.is_present("cleanup") {
        run_daemon(settings)?;
    }

    Ok(())
}

fn run_daemon(settings: Settings) -> Fallible<()> {
    let sys = actix_rt::System::new("sc-web");

    // TODO: we can't catch anything except sighub for child processes, hint was to look into daemon(1)
    
    #[cfg(target_os = "linux")]
    actix::spawn(async move {
        let kind = SignalKind::hangup();
        let mut sighub = signal(kind).unwrap();
        for _ in sighub.recv().await {
            info!("Received sighub, reloading..");
            //TODO: don't hang the executor
            let settings = match settings::Settings::new() {
                Err(e) => {
                    error!("Error loading configuration {}", e);
                    info!("Please check your config file. If upgrading from an earlier version be sure to check for new required fields in config/template.toml");
                    continue;
                }
                Ok(v) => v,
            };
            if let Err(e) = ServiceController::from_registry()
                .send(messages::unchecked::ReloadServices { data: settings.services })
                .await {
                error!("Unable to reload service, failed to send msg: {}",e);
            }
        }
    });
    let services = settings.services;

    let bcrypt_cost = settings.security.bcrypt_cost;
    let max_session_age_secs = settings.web.max_session_age_secs;
    let disable_totp = settings.security.disable_totp;
    if disable_totp {
        warn!("TOTP auth disabled!");
    }

    actix::spawn(async move {
        if let Err(e) = async move {
            UserService::from_registry()
                .send(messages::unchecked::SetConfig {
                    cost: bcrypt_cost,
                    max_session_age_secs,
                    disable_totp,
                })
                .await?;
            ServiceController::from_registry()
                .send(messages::unchecked::ReloadServices { data: services })
                .await?;
            Ok::<(), failure::Error>(())
        }
        .await
        {
            error!("Startup failure: {}", e);
        }
    });
    let _ = web::start(&settings.web, max_session_age_secs);
    sys.run()?;
    Ok(())
}

#[cfg(test)]
mod test {
    use arraydeque::{ArrayDeque, Wrapping};

    #[test]
    fn test_arraydequeue() {
        let mut deque: ArrayDeque<[_; 3], Wrapping> = ArrayDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);
        let val: Vec<_> = deque.iter().map(|s| s.clone()).collect();
        assert_eq!(vec![2, 3, 4], val);
        assert_eq!(3, deque.len());
        deque.iter().for_each(|s| print!("{} ", s));
        println!();
    }
}
