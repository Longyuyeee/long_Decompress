use super::*;
use std::{cell::RefCell, io, rc::Rc};

#[derive(Default)]
struct State { bytes: Vec<u8>, signals: usize, failure: u8 }
#[derive(Clone)]
struct Sink(Rc<RefCell<State>>);
impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut state = self.0.borrow_mut();
        if state.failure == 1 || state.failure == 2 && bytes.is_empty() {
            return Err(io::Error::other("injected terminal write"));
        }
        if bytes.is_empty() { state.signals += 1; }
        state.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.0.borrow().failure == 3 { Err(io::Error::other("injected terminal flush")) } else { Ok(()) }
    }
}

#[test]
fn sevenz1647_raw_deflate_reaches_stream_end_before_drop_once() {
    for level in [1, 6, 9] {
        for input in [Vec::new(), b"raw deflate payload + trailer\n".repeat(4096)] {
            let state = Rc::new(RefCell::new(State::default()));
            let mut encoder = add_encoder(CountingWriter::new(Sink(state.clone())),
                &SevenZMethodConfiguration::new(SevenZMethod::DEFLATE).with_options(MethodOptions::Num(level))).unwrap();
            encoder.write_all(&input).unwrap();
            encoder.flush().unwrap();
            let before = state.borrow().bytes.clone();
            let mut scratch = vec![0; input.len() + 1];
            assert_ne!(flate2::Decompress::new(false).decompress(&before, &mut scratch, flate2::FlushDecompress::Finish).unwrap(), flate2::Status::StreamEnd);
            encoder.write(&[]).unwrap();
            let encoded = state.borrow().bytes.clone();
            let mut decoder = flate2::Decompress::new(false);
            assert_eq!(decoder.decompress(&encoded, &mut scratch, flate2::FlushDecompress::Finish).unwrap(), flate2::Status::StreamEnd);
            assert_eq!(decoder.total_in() as usize, encoded.len());
            assert_eq!(decoder.total_out() as usize, input.len());
            assert_eq!(&scratch[..input.len()], input);
            encoder.write(&[]).unwrap();
            assert_eq!(state.borrow().signals, 1);
            assert!(encoder.write(b"closed").is_err());
        }
    }
}

#[test]
fn sevenz1647_raw_deflate_terminal_errors_propagate() {
    for failure in [1, 2, 3] {
        let state = Rc::new(RefCell::new(State::default()));
        let mut encoder = add_encoder(CountingWriter::new(Sink(state.clone())),
            &SevenZMethodConfiguration::new(SevenZMethod::DEFLATE)).unwrap();
        encoder.write_all(b"finish must report errors").unwrap();
        encoder.flush().unwrap();
        state.borrow_mut().failure = failure;
        assert!(encoder.write(&[]).is_err(), "failure {failure}");
        state.borrow_mut().failure = 0;
    }
}
