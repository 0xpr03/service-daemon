use std::io;

/// IO testing applicaton to print out input
fn main() {
    println!("IO test service, write something to stdin and receive an echo on stdout and amount of bytes received on stderr");
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(n) => {
            println!("Received {}", input);
            eprintln!("{} bytes", n);
        }
        Err(error) => eprintln!("error reading input: {}", error),
    }
}
