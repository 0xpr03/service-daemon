use std::thread::sleep;
use std::time::Duration;
use std::time::SystemTime;
fn main() {
    loop {
        println!("Ping {:?}", SystemTime::now());
        sleep(Duration::from_millis(1_000));
    }
}
