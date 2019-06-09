//! Adopted from tokio-io io::lines.rs and read_until.rs
//! Allows read-line of [u8] for later String::from_utf8_lossy conversion

use std::io;
use std::mem;
use tokio_io::try_nb;

use futures::{Poll, Stream};

use tokio_io::AsyncRead;

/// Combinator created by the top-level `lines` method which is a stream over
/// the lines of text on an I/O object.
#[derive(Debug)]
pub struct Lines<A> {
    io: A,
    buf: Vec<u8>,
}

/// Creates a new stream from the I/O object given representing the lines of
/// input that are found on `A`.
///
/// This method takes an asynchronous I/O object, `a`, and returns a `Stream` of
/// lines that the object contains. The returned stream will reach its end once
/// `a` reaches EOF.
pub fn lines<A>(a: A) -> Lines<A>
where
    A: AsyncRead + io::BufRead,
{
    Lines {
        io: a,
        buf: Vec::new(),
    }
}

impl<A> Lines<A> {
    /// Returns the underlying I/O object.
    ///
    /// Note that this may lose data already read into internal buffers. It's
    /// recommended to only call this once the stream has reached its end.
    pub fn into_inner(self) -> A {
        self.io
    }
}

impl<A> Stream for Lines<A>
where
    A: AsyncRead + io::BufRead,
{
    type Item = Vec<u8>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Vec<u8>>, io::Error> {
        // let n = try_nb!(self.io.read_line(&mut self.line));
        let n = try_nb!(self.io.read_until(b'\n', &mut self.buf));
        if n == 0 && self.buf.len() == 0 {
            return Ok(None.into());
        }
        // let line = String::from_utf8_lossy(self.bug);
        if self.buf.ends_with(&[b'\n']) {
            self.buf.pop();
            if self.buf.ends_with(&[b'\r']) {
                self.buf.pop();
            }
        }
        Ok(Some(mem::replace(&mut self.buf, Vec::new())).into())
    }
}
