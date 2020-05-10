use std::env;

fn main() {
    println!("Envs:");
    for (key, value) in env::vars() {
        println!("{}: {}", key, value);
    }
}
