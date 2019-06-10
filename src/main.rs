#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;

use actix;
use actix::prelude::*;
use actix::System;
use failure::Fallible;
use crate::handler::service::ServiceController;
use crate::handler::messages;
use tokio_signal::unix::{Signal, SIGINT, SIGTERM};

mod readline;
mod handler;
mod settings;
mod db;
mod web;
fn main() -> Fallible<()> {
    env_logger::init();
    let settings = settings::Settings::new()?;
    trace!("{:#?}", settings);

    System::run(|| {
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
        let _ = web::start();
    })?;

    Ok(())
}

fn load_config() -> Fallible<()> {
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
