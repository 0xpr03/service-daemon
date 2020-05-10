use std::time::SystemTime;
use std::thread::sleep;
use std::time::Duration;
fn main() {
    loop {
        println!("Ping {:?}",SystemTime::now());
        sleep(Duration::from_millis(1_000));
    }
}