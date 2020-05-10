use std::io::{self, Write};
fn main() -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout_handle = stdout.lock();
    let stderr = io::stderr();
    let mut stderr_handle = stderr.lock();

    let mut i = 0;
    loop {
        if i % 1000 == 0 {
            stdout_handle.flush()?;
            stderr_handle.flush()?;
        }
        writeln!(&mut stdout_handle, "hello out {}", i)?;
        writeln!(&mut stderr_handle, "hello err {}", i)?;
        i += 1;
    }
}
