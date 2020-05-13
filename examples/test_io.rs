use std::io;

/// IO testing applicaton to print out input
fn main() {
    println!("IO test service, write something to stdin and receive an echo on stdout and amount of bytes received on stderr");
    println!("Enter quit to end this service");
    loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                println!("Received {}", input);
                eprintln!("{} bytes", n);
                if input.trim() == "quit" {
                    break;
                }
            }
            Err(error) => eprintln!("error reading input: {}", error),
        }
    }
    println!("Exit normally or crash? (Y/n)");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    if input.trim() == "n" {
        panic!("Test panic crash!");
    }
}
