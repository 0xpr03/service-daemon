use std::env;

fn main() {
    println!("Environment test service printing all ENVs");
    println!("Envs:");
    for (key, value) in env::vars() {
        println!("{}: {}", key, value);
    }
}
