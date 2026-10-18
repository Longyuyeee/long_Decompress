pub mod bzip2;
pub mod deflate;
pub mod lzma;
pub mod store;

use std::io::{self, Write};

/// Writer wrapper that caps total output at a declared size, turning a
/// decompression bomb (unbounded output from a small input) into an error.
pub struct LimitWriter<W: Write> {
    inner: W,
    remaining: u64,
}

impl<W: Write> LimitWriter<W> {
    pub fn new(inner: W, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }
}

impl<W: Write> Write for LimitWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() as u64 > self.remaining {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "decompressed output exceeds declared uncompressed size",
            ));
        }
        let n = self.inner.write(buf)?;
        self.remaining -= n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
