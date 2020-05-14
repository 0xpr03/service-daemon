#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;

use crate::handler::messages;
use crate::handler::service::ServiceController;
use crate::handler::user::UserService;

use actix;
use actix::prelude::*;
use failure::Fallible;
use env_logger;

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
    let settings = match settings::Settings::new() {
        Err(e) => {
            error!("Error loading configuration {}", e);
            info!("Please check your config file. If upgrading from an earlier version be sure to check for new required fields in config/template.toml");
            return Err(e.into());
        }
        Ok(v) => v,
    };
    trace!("{:#?}", settings);

    let sys = actix_rt::System::new("sc-web");

    // TODO: we can't catch anything except sighub for child processes, hint was to look into daemon(1)
    // let sigint = Signal::new(SIGINT).flatten_stream();
    // let sigterm = Signal::new(SIGTERM).flatten_stream();
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
                .send(messages::unchecked::LoadServices { data: services })
                .await?;
            UserService::from_registry()
                .send(messages::unchecked::StartupCheck {})
                .await??;
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
    use circular_queue::CircularQueue;
    #[test]
    fn test_circular_buffer() {
        let mut queue = CircularQueue::with_capacity(3);
        queue.push(1);
        queue.push(2);
        queue.push(3);
        queue.push(4);
        assert_eq!(queue.len(), 3);
        let val: Vec<_> = queue.iter().map(|s| s.clone()).collect();
        assert_eq!(vec![4, 3, 2], val);
        assert_eq!(queue.len(), 3);
        queue.iter().for_each(|s| print!("{} ", s));
        println!();
    }

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
