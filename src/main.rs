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

mod crypto;
mod db;
mod handler;
mod readline;
mod settings;
mod web;

const RUST_LOG: &'static str = "RUST_LOG";

fn main() -> Fallible<()> {
    if std::env::var(RUST_LOG).is_err() {
        std::env::set_var(
            RUST_LOG,
            "service_daemon=trace,actix_web=info,actix_server=info",
        );
    }
    env_logger::init();
    let settings = settings::Settings::new()?;
    trace!("{:#?}", settings);

    let sys = actix_rt::System::new("sc-web");

    // TODO: we can't catch anything except sighub for child processes, hint was to look into daemon(1)
    // let sigint = Signal::new(SIGINT).flatten_stream();
    // let sigterm = Signal::new(SIGTERM).flatten_stream();

    // // Use the `select` combinator to merge these two streams into one
    // let stream = sigint.select(sigterm);
    // let fut = stream
    //     .for_each(|signal| {
    //         println!("Received signal {}", signal);
    //         Ok(())
    //     })
    //     .map_err(|_| ());
    // actix::spawn(fut);
    let startup = ServiceController::from_registry()
        .send(messages::LoadServices {
            data: settings.services,
        })
        .map_err(|_| ());
    actix::spawn(startup);
    let crypto_setup = UserService::from_registry()
        .send(messages::SetPasswordCost {
            cost: settings.security.bcrypt_cost,
        })
        .and_then(|_| UserService::from_registry().send(messages::StartupCheck {}))
        .map(|_| ())
        .map_err(|e| error!("User-Service startup check failed! {}", e));
    actix::spawn(crypto_setup);
    let _ = web::start(settings.web.domain, settings.web.max_session_age_secs);
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
