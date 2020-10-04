use std::io::{Error, ErrorKind, Result};

fn main() -> Result<()> {
    Err(Error::new(ErrorKind::Other, "oh no!"))
}