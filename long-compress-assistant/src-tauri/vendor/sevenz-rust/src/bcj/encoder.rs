use super::BCJFilter;
use std::io::{self, Write};

#[derive(PartialEq)]
enum State { Active, Finished, Failed }

/// Streaming x86 BCJ. Preserve the final <=4 bytes until the next write;
/// flush must not reset the instruction position or expose an incomplete call.
pub struct X86Encoder<W: Write> {
    inner: W,
    filter: BCJFilter,
    pending: [u8; 8192],
    len: usize,
    state: State,
}
impl<W: Write> X86Encoder<W> {
    pub(crate) fn new(inner: W) -> Self {
        Self { inner, filter: BCJFilter::new_x86(0, true), pending: [0; 8192], len: 0, state: State::Active }
    }
    fn failed() -> io::Error { io::Error::new(io::ErrorKind::BrokenPipe, "x86 BCJ encoder finished or failed") }
}
impl<W: Write> Write for X86Encoder<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.state == State::Failed || !bytes.is_empty() && self.state == State::Finished { return Err(Self::failed()); }
        if bytes.is_empty() {
            if self.state == State::Active {
                self.state = State::Failed;
                self.inner.write_all(&self.pending[..self.len])?;
                // The next coder owns its finish/flush. Never flush LZMA again
                // after this signal; it has already reached its terminal state.
                self.inner.write(&[])?;
                self.len = 0;
                self.state = State::Finished;
            }
            return Ok(0);
        }
        let mut offset = 0;
        while offset < bytes.len() {
            let count = (bytes.len() - offset).min(self.pending.len() - self.len);
            self.pending[self.len..self.len + count].copy_from_slice(&bytes[offset..offset + count]);
            self.len += count;
            offset += count;
            let processed = self.filter.code(&mut self.pending[..self.len]);
            if let Err(error) = self.inner.write_all(&self.pending[..processed]) {
                self.state = State::Failed;
                return Err(error);
            }
            self.pending.copy_within(processed..self.len, 0);
            self.len -= processed;
            debug_assert!(self.len <= 4);
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.state == State::Failed { return Err(Self::failed()); }
        if self.state == State::Finished { return Ok(()); }
        if let Err(error) = self.inner.flush() { self.state = State::Failed; return Err(error); }
        Ok(())
    }
}
