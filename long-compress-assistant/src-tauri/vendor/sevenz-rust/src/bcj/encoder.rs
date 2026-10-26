use super::BCJFilter;
use std::io::{self, Write};

#[derive(PartialEq)]
enum State { Active, Finished, Failed }

/// Streaming x86/ARM32/Thumb/ARM64/RISC-V BCJ. Preserve the final <=4/3/3/3/7 bytes until the next write;
/// flush must not reset the instruction position or expose an incomplete call.
pub struct BcjEncoder<W: Write> {
    inner: W,
    filter: BCJFilter,
    pending: [u8; 8192],
    len: usize,
    state: State,
}
pub type X86Encoder<W> = BcjEncoder<W>;
impl<W: Write> BcjEncoder<W> {
    pub(crate) fn new(inner: W) -> Self {
        Self::with_filter(inner, BCJFilter::new_x86(0, true))
    }
    pub(crate) fn new_arm(inner: W) -> Self {
        Self::with_filter(inner, BCJFilter::new_arm(0, true))
    }
    pub(crate) fn new_arm_thumb(inner: W) -> Self {
        Self::with_filter(inner, BCJFilter::new_arm_thumb(0, true))
    }
    pub(crate) fn new_arm64(inner: W) -> Self {
        Self::with_filter(inner, BCJFilter::new_arm64(0, true))
    }
    pub(crate) fn new_riscv(inner: W) -> Self {
        Self::with_filter(inner, BCJFilter::new_riscv(0, true))
    }
    fn with_filter(inner: W, filter: BCJFilter) -> Self {
        Self { inner, filter, pending: [0; 8192], len: 0, state: State::Active }
    }
    fn failed() -> io::Error { io::Error::new(io::ErrorKind::BrokenPipe, "BCJ encoder finished or failed") }
}
impl<W: Write> Write for BcjEncoder<W> {
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
            debug_assert!(self.len <= 7);
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
