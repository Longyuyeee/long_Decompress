use std::io::{self, Write};

#[derive(PartialEq)]
enum State { Active, Finished, Failed }

/// Filter bytes before the next encoder. History spans writes and solid entries.
pub(crate) struct DeltaEncoder<W: Write> {
    inner: W,
    history: [u8; 256],
    position: u8,
    distance: usize,
    state: State,
}
impl<W: Write> DeltaEncoder<W> {
    pub(crate) fn new(inner: W, distance: u32) -> io::Result<Self> {
        if !(1..=256).contains(&distance) { return Err(io::Error::new(io::ErrorKind::InvalidInput,"Delta distance must be 1..256")); }
        Ok(Self { inner, history: [0;256], position: 0, distance: distance as usize, state: State::Active })
    }
}
impl<W: Write> Write for DeltaEncoder<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.state == State::Failed || !bytes.is_empty() && self.state == State::Finished {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe,"Delta encoder finished or failed"));
        }
        if bytes.is_empty() {
            if self.state == State::Active {
                self.state = State::Failed;
                self.inner.write(&[])?;
                // The following coder finishes and flushes its own output on
                // this signal. LZMA/LZMA2 reject flush after they have ended.
                self.state = State::Finished;
            }
            return Ok(0);
        }
        let mut encoded = [0u8;8192];
        for chunk in bytes.chunks(encoded.len()) {
            for (i,&byte) in chunk.iter().enumerate() {
                let pos = self.position as usize;
                encoded[i] = byte.wrapping_sub(self.history[(pos+self.distance)&255]);
                self.history[pos] = byte;
                self.position = self.position.wrapping_sub(1);
            }
            if let Err(error) = self.inner.write_all(&encoded[..chunk.len()]) {
                self.state = State::Failed;
                return Err(error);
            }
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.state == State::Failed { return Err(io::Error::new(io::ErrorKind::BrokenPipe,"Delta encoder failed")); }
        if self.state == State::Finished { return Ok(()); }
        if let Err(error) = self.inner.flush() { self.state = State::Failed; return Err(error); }
        Ok(())
    }
}
