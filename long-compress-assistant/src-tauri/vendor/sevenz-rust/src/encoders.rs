use std::io::Write;

#[cfg(feature = "aes256")]
use crate::aes256sha256::Aes256Sha256Encoder;
use crate::{
    archive::{SevenZMethod, SevenZMethodConfiguration},
    lzma::CountingWriter,
    lzma::{LZMA2Options, LZMA2Writer, LZMAWriter},
    method_options::MethodOptions,
    Error,
};

pub enum Encoder<W: Write> {
    DELTA(crate::delta_encoder::DeltaEncoder<CountingWriter<W>>),
    COPY(CopyEncoder<CountingWriter<W>>),
    LZMA(LZMAWriter<W>),
    LZMA2(LZMA2Writer<W>),
    #[cfg(feature = "bzip2")]
    BZIP2(Bzip2Encoder<CountingWriter<W>>),
    #[cfg(feature = "additional-codecs")]
    DEFLATE(DeflateEncoder<CountingWriter<W>>),
    #[cfg(feature = "additional-codecs")]
    PPMD(PpmdEncoder<CountingWriter<W>>),
    #[cfg(feature = "aes256")]
    AES(Aes256Sha256Encoder<W>),
}

impl<W: Write> Write for Encoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Encoder::DELTA(w) => w.write(buf),
            Encoder::COPY(w) => w.write(buf),
            Encoder::LZMA(w) => w.write(buf),
            Encoder::LZMA2(w) => w.write(buf),
            #[cfg(feature = "bzip2")]
            Encoder::BZIP2(w) => w.write(buf),
            #[cfg(feature = "additional-codecs")]
            Encoder::DEFLATE(w) => w.write(buf),
            #[cfg(feature = "additional-codecs")]
            Encoder::PPMD(w) => w.write(buf),
            #[cfg(feature = "aes256")]
            Encoder::AES(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Encoder::DELTA(w) => w.flush(),
            Encoder::COPY(w) => w.flush(),
            Encoder::LZMA(w) => w.flush(),
            Encoder::LZMA2(w) => w.flush(),
            #[cfg(feature = "bzip2")]
            Encoder::BZIP2(w) => w.flush(),
            #[cfg(feature = "additional-codecs")]
            Encoder::DEFLATE(w) => w.flush(),
            #[cfg(feature = "additional-codecs")]
            Encoder::PPMD(w) => w.flush(),
            #[cfg(feature = "aes256")]
            Encoder::AES(w) => w.flush(),
        }
    }
}

pub fn add_encoder<W: Write>(
    input: CountingWriter<W>,
    method_config: &SevenZMethodConfiguration,
) -> Result<Encoder<W>, Error> {
    let method = method_config.method;

    match method.id() {
        SevenZMethod::ID_DELTA => {
            let distance = delta_distance(method_config.options.as_ref())?;
            Ok(Encoder::DELTA(crate::delta_encoder::DeltaEncoder::new(input, distance).map_err(Error::io)?))
        }
        SevenZMethod::ID_COPY => {
            if method_config.options.is_some() { return Err(Error::other("Copy does not accept compression options")); }
            Ok(Encoder::COPY(CopyEncoder { inner: input, state: CopyState::Active }))
        }
        #[cfg(feature = "additional-codecs")]
        SevenZMethod::ID_PPMD => {
            let (order, memory) = ppmd_encoder_settings(method_config.options.as_ref())?;
            let inner = ppmd_rust::Ppmd7Encoder::new(input, order, memory)
                .map_err(|e| Error::other(format!("PPMd encoder: {e}")))?;
            Ok(Encoder::PPMD(PpmdEncoder { active: Some(inner), finished: None }))
        }
        #[cfg(feature = "additional-codecs")]
        SevenZMethod::ID_DEFLATE => {
            let level = match method_config.options.as_ref() {
                Some(MethodOptions::Num(level)) => (*level).clamp(1, 9),
                None => 6,
                _ => return Err(Error::other("Invalid Deflate encoder options")),
            };
            Ok(Encoder::DEFLATE(DeflateEncoder {
                inner: flate2::write::DeflateEncoder::new(input, flate2::Compression::new(level)),
                done: false,
            }))
        }
        #[cfg(feature = "bzip2")]
        SevenZMethod::ID_BZIP2 => {
            let level = match method_config.options.as_ref() {
                Some(MethodOptions::Num(level)) => (*level).clamp(1, 9),
                None => 6,
                _ => return Err(Error::other("Invalid Bzip2 encoder options")),
            };
            Ok(Encoder::BZIP2(Bzip2Encoder { inner: bzip2::write::BzEncoder::new(input, bzip2::Compression::new(level)), done: false }))
        }
        SevenZMethod::ID_LZMA => {
            let mut def_opts = LZMA2Options::default();
            let options = get_lzma2_options(method_config.options.as_ref(), &mut def_opts);
            let lz = LZMAWriter::new_no_header(input, options, false).map_err(Error::io)?;
            Ok(Encoder::LZMA(lz))
        }
        SevenZMethod::ID_LZMA2 => {
            let mut def_opts = LZMA2Options::default();
            let options = get_lzma2_options(method_config.options.as_ref(), &mut def_opts);

            let lz = LZMA2Writer::new(input, options);
            Ok(Encoder::LZMA2(lz))
        }
        #[cfg(feature = "aes256")]
        SevenZMethod::ID_AES256SHA256 => {
            let options = match method_config.options.as_ref() {
                Some(MethodOptions::Aes(p)) => p,
                _ => return Err(Error::PasswordRequired),
            };

            Ok(Encoder::AES(Aes256Sha256Encoder::new(input, options)?))
        }
        _ => Err(Error::UnsupportedCompressionMethod(
            method.name().to_string(),
        )),
    }
}

fn delta_distance(options: Option<&MethodOptions>) -> Result<u32, Error> {
    match options { None => Ok(1), Some(MethodOptions::Num(n)) if (1..=256).contains(n) => Ok(*n), _ => Err(Error::other("Delta distance must be 1..256")) }
}

#[derive(PartialEq)]
enum CopyState { Active, Finished, Failed }

/// Copy still participates in coder termination: AES must pad only after
/// all source bytes, and any downstream terminal failure remains a failure.
pub struct CopyEncoder<W: Write> { inner: W, state: CopyState }
impl<W: Write> Write for CopyEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.state == CopyState::Failed || !buf.is_empty() && self.state == CopyState::Finished {
            return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Copy coder already finished or failed"));
        }
        if !buf.is_empty() { return self.inner.write(buf); }
        if self.state == CopyState::Active {
            self.state = CopyState::Failed;
            self.inner.write(&[])?;
            self.inner.flush()?;
            self.state = CopyState::Finished;
        }
        Ok(0)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if self.state == CopyState::Failed {
            return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Copy coder failed to finish"));
        }
        self.inner.flush()
    }
}

/// SevenZWriter ends a coder with an empty write after flush. Explicitly
/// finish Bzip2 first, then forward that signal through counting/AES writers;
/// neither the Bzip2 trailer nor AES padding may depend on a fallible Drop.
#[cfg(feature = "bzip2")]
pub struct Bzip2Encoder<W: Write> { inner: bzip2::write::BzEncoder<W>, done: bool }
#[cfg(feature = "bzip2")]
impl<W: Write> Write for Bzip2Encoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            if !self.done {
                self.inner.try_finish()?;
                self.inner.get_mut().write(&[])?;
                self.inner.get_mut().flush()?;
                self.done = true;
            }
            return Ok(0);
        }
        if self.done { return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe,"Bzip2 coder already finished")); }
        self.inner.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if self.done { self.inner.get_mut().flush() } else { self.inner.flush() }
    }
}

/// A raw Deflate stream must reach BFINAL before ending the following AES
/// coder. Return every terminal error to SevenZWriter, before Drop can run.
#[cfg(feature = "additional-codecs")]
pub struct DeflateEncoder<W: Write> {
    inner: flate2::write::DeflateEncoder<W>,
    done: bool,
}

#[cfg(feature = "additional-codecs")]
impl<W: Write> Write for DeflateEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            if !self.done {
                self.inner.try_finish()?;
                self.inner.get_mut().write(&[])?;
                self.inner.get_mut().flush()?;
                self.done = true;
            }
            return Ok(0);
        }
        if self.done {
            return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Deflate coder already finished"));
        }
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.done { self.inner.get_mut().flush() } else { self.inner.flush() }
    }
}

#[cfg(feature = "additional-codecs")]
fn ppmd_encoder_settings(options: Option<&MethodOptions>) -> Result<(u32, u32), Error> {
    let level = match options {
        Some(MethodOptions::Num(level)) => (*level).clamp(1, 9),
        None => 6,
        _ => return Err(Error::other("Invalid PPMd encoder options")),
    };
    let (order, mib) = [(2,4),(3,4),(4,8),(5,8),(6,16),(7,16),(8,32),(10,32),(12,64)][level as usize - 1];
    Ok((order, mib * 1024 * 1024))
}

/// Ppmd7Encoder::flush finalizes its range coder. Only the terminal empty
/// write may do that; ordinary flush must allow further source bytes.
#[cfg(feature = "additional-codecs")]
pub struct PpmdEncoder<W: Write> {
    active: Option<ppmd_rust::Ppmd7Encoder<W>>,
    finished: Option<W>,
}

#[cfg(feature = "additional-codecs")]
impl<W: Write> Write for PpmdEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if !buf.is_empty() {
            return match &mut self.active {
                Some(encoder) => encoder.write(buf),
                None => Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "PPMd coder already finished or failed")),
            };
        }
        if let Some(encoder) = self.active.take() {
            // 7z supplies the unpack size. Do not add a ZIP/PPMd8 end marker.
            let mut output = encoder.finish(false)?;
            output.write(&[])?;
            output.flush()?;
            self.finished = Some(output);
        } else if self.finished.is_none() {
            return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "PPMd coder failed to finish"));
        }
        Ok(0)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(encoder) = &mut self.active { return encoder.get_mut().flush(); }
        if let Some(output) = &mut self.finished { return output.flush(); }
        Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "PPMd coder failed to finish"))
    }
}

pub(crate) fn get_options_as_properties<'a>(
    method: SevenZMethod,
    options: Option<&MethodOptions>,
    out: &'a mut [u8],
) -> &'a [u8] {
    match method.id() {
        SevenZMethod::ID_DELTA => {
            let Ok(distance) = delta_distance(options) else { return &[]; };
            out[0] = (distance - 1) as u8;
            &out[..1]
        }
        #[cfg(feature = "additional-codecs")]
        SevenZMethod::ID_PPMD => {
            let Ok((order, memory)) = ppmd_encoder_settings(options) else { return &[]; };
            out[0] = order as u8;
            out[1..5].copy_from_slice(&memory.to_le_bytes());
            &out[..5]
        }
        SevenZMethod::ID_LZMA2 => {
            let dict_size = options
                .map(|o| o.get_lzma2_dict_size())
                .unwrap_or(LZMA2Options::DICT_SIZE_DEFAULT);
            let lead = dict_size.leading_zeros();
            let second_bit = (dict_size >> (30u32.wrapping_sub(lead))).wrapping_sub(2);
            let prop = (19u32.wrapping_sub(lead) * 2 + second_bit) as u8;
            out[0] = prop;
            &out[0..1]
        }
        SevenZMethod::ID_LZMA => {
            let mut def_opts = LZMA2Options::default();
            let options = get_lzma2_options(options, &mut def_opts);
            let dict_size = options.dict_size;
            out[0] = options.get_props();
            out[1..5].copy_from_slice(dict_size.to_le_bytes().as_ref());
            &out[0..5]
        }
        #[cfg(feature = "aes256")]
        SevenZMethod::ID_AES256SHA256 => {
            let options = match options.as_ref() {
                Some(MethodOptions::Aes(p)) => p,
                _ => return &[],
            };
            options.write_properties(out);
            &out[..34]
        }
        _ => &[],
    }
}

#[inline]
pub(crate) fn get_lzma2_options<'a>(
    options: Option<&'a MethodOptions>,
    def_opt: &'a mut LZMA2Options,
) -> &'a LZMA2Options {
    let options = match options.as_ref() {
        Some(MethodOptions::LZMA2(opts)) => opts,
        Some(MethodOptions::Num(n)) => {
            def_opt.dict_size = *n;
            def_opt
        }
        _ => {
            def_opt.dict_size = LZMA2Options::DICT_SIZE_DEFAULT;
            def_opt
        }
    };
    options
}


#[cfg(all(test,feature="bzip2"))]
mod sevenz1646_tests {
 use super::*;use std::{rc::Rc,cell::RefCell,io::{self,Read}};
 #[derive(Default)]struct State{bytes:Vec<u8>,signal:usize,fail:u8}
 #[derive(Clone)]struct Sink(Rc<RefCell<State>>);
 impl Write for Sink{fn write(&mut self,b:&[u8])->io::Result<usize>{let mut s=self.0.borrow_mut();if s.fail==1||s.fail==2&&b.is_empty(){return Err(io::Error::other("injected terminal write"));}if b.is_empty(){s.signal+=1;}s.bytes.extend_from_slice(b);Ok(b.len())}fn flush(&mut self)->io::Result<()>{if self.0.borrow().fail==3{Err(io::Error::other("injected terminal flush"))}else{Ok(())}}}
 #[test]fn sevenz1646_bzip2_explicit_eos_before_drop_and_idempotent(){
  for input in [Vec::new(),b"abc terminal tail".repeat(1000)]{let state=Rc::new(RefCell::new(State::default()));let mut encoder=add_encoder(CountingWriter::new(Sink(state.clone())),&SevenZMethodConfiguration::new(SevenZMethod::BZIP2).with_options(MethodOptions::Num(9))).unwrap();encoder.write_all(&input).unwrap();encoder.flush().unwrap();encoder.write(&[]).unwrap();let bytes=state.borrow().bytes.clone();let mut decoded=Vec::new();bzip2::read::BzDecoder::new(bytes.as_slice()).read_to_end(&mut decoded).unwrap();assert_eq!(decoded,input);assert_eq!(state.borrow().signal,1);encoder.write(&[]).unwrap();assert_eq!(state.borrow().signal,1);assert!(encoder.write(b"after finish").is_err());}
 }
 #[test]fn sevenz1646_bzip2_terminal_errors_propagate(){
  for fail in [1,2,3]{let state=Rc::new(RefCell::new(State::default()));let mut encoder=add_encoder(CountingWriter::new(Sink(state.clone())),&SevenZMethodConfiguration::new(SevenZMethod::BZIP2)).unwrap();encoder.write_all(b"data").unwrap();encoder.flush().unwrap();state.borrow_mut().fail=fail;assert!(encoder.write(&[]).is_err(),"failure {fail}");state.borrow_mut().fail=0;}
 }
}

#[cfg(all(test, feature = "additional-codecs"))]
#[path = "deflate_encoder_1647_tests.rs"]
mod deflate_encoder_1647_tests;

#[cfg(all(test, feature = "additional-codecs"))]
#[path = "ppmd_encoder_1648_tests.rs"]
mod ppmd_encoder_1648_tests;

#[cfg(test)]
#[path = "copy_encoder_1649_tests.rs"]
mod copy_encoder_1649_tests;
