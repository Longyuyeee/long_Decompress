use std::io::Read;

use crate::{
    internal::ppmd7::{PPMd7, RangeDecoder},
    Error, PPMD7_MAX_MEM_SIZE, PPMD7_MAX_ORDER, PPMD7_MIN_MEM_SIZE, PPMD7_MIN_ORDER, SYM_END,
};

/// A decoder to decompress data using PPMd7 (PPMdH) with the 7z range coder.
pub struct Ppmd7Decoder<R: Read> {
    ppmd: PPMd7<RangeDecoder<R>>,
    finished: bool,
}

unsafe impl<R: Read> Send for Ppmd7Decoder<R> {}
unsafe impl<R: Read> Sync for Ppmd7Decoder<R> {}

impl<R: Read> Ppmd7Decoder<R> {
    /// Creates a new [`Ppmd7Decoder`] which provides a reader over the uncompressed data.
    ///
    /// The given `order` must be between [`PPMD7_MIN_ORDER`] and [`PPMD7_MAX_ORDER`]
    /// The given `mem_size` must be between [`PPMD7_MIN_MEM_SIZE`] and [`PPMD7_MAX_MEM_SIZE`]
    pub fn new(reader: R, order: u32, mem_size: u32) -> crate::Result<Self> {
        if !(PPMD7_MIN_ORDER..=PPMD7_MAX_ORDER).contains(&order)
            || !(PPMD7_MIN_MEM_SIZE..=PPMD7_MAX_MEM_SIZE).contains(&mem_size)
        {
            return Err(Error::InvalidParameter);
        }

        let ppmd = PPMd7::new_decoder(reader, order, mem_size)?;

        Ok(Self {
            ppmd,
            finished: false,
        })
    }

    /// Gets a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.ppmd.get_ref()
    }

    /// Gets a mutable reference to the underlying reader.
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this decoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.ppmd.get_mut()
    }

    /// Returns the inner reader.
    pub fn into_inner(self) -> R {
        self.ppmd.into_inner()
    }

    /// Finish a size-delimited 7z stream without attempting another symbol.
    /// The caller must first read exactly the declared output and separately
    /// verify compressed coverage and CRCs. Ordinary Read behavior is unchanged.
    /// Mirrors 7-Zip's size-defined FinishStream range-code == 0 condition.
    pub fn finish_7z(self) -> std::io::Result<R> {
        if self.finished || self.ppmd.range_decoder_code() != 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"PPMd 7z range coder did not finish"));
        }
        Ok(self.ppmd.into_inner())
    }
}

#[cfg(test)]
mod sevenz1627_tests {
    use super::*;
    use std::io::{Read,Write};
    fn encoded()->(Vec<u8>,Vec<u8>){let text=b"7z PPMd complete text and range coder finish\n".repeat(100);let mut e=crate::Ppmd7Encoder::new(vec![],6,1024*1024).unwrap();e.write_all(&text).unwrap();(text,e.finish(false).unwrap())}
    #[test]
    fn sevenz1627_size_delimited_stream_needs_range_finish_not_an_extra_symbol(){
        let (text,data)=encoded();let mut d=Ppmd7Decoder::new(std::io::Cursor::new(data.clone()),6,1024*1024).unwrap();let mut decoded=vec![0;text.len()];d.read_exact(&mut decoded).unwrap();assert_eq!(decoded,text);assert_eq!(d.finish_7z().unwrap().position(),data.len() as u64);
        let mut d=Ppmd7Decoder::new(data.as_slice(),6,1024*1024).unwrap();d.read_exact(&mut [0;10]).unwrap();assert!(d.finish_7z().is_err());
    }
    #[test]
    fn sevenz1627_truncated_or_changed_flush_cannot_finish_as_complete(){
        let (text,data)=encoded();for cut in [1,2,4]{let mut d=Ppmd7Decoder::new(&data[..data.len()-cut],6,1024*1024).unwrap();let mut output=vec![0;text.len()];assert!(d.read_exact(&mut output).is_err() || d.finish_7z().is_err());}
        let mut changed=data;*changed.last_mut().unwrap()^=1;let mut d=Ppmd7Decoder::new(changed.as_slice(),6,1024*1024).unwrap();let mut out=vec![0;text.len()];assert!(d.read_exact(&mut out).is_err() || out!=text || d.finish_7z().is_err());
    }
}

impl<R: Read> Read for Ppmd7Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.finished {
            return Ok(0);
        }

        if buf.is_empty() {
            return Ok(0);
        }

        let mut sym = 0;
        let mut decoded = 0;

        for byte in buf.iter_mut() {
            match self.ppmd.decode_symbol() {
                Ok(symbol) => sym = symbol,
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::UnexpectedEof {
                        self.finished = true;
                        return Ok(decoded);
                    }
                    return Err(err);
                }
            }

            if sym < 0 {
                break;
            }

            *byte = sym as u8;
            decoded += 1;
        }

        let code = self.ppmd.range_decoder_code();

        if sym >= 0 {
            return Ok(decoded);
        }

        self.finished = true;

        if sym != SYM_END || code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Error during PPMd decoding",
            ));
        }

        // END_MARKER detected
        Ok(decoded)
    }
}
