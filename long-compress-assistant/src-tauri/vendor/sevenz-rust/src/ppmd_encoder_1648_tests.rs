use super::*;
use std::{cell::RefCell, io::{self, Cursor, Read}, rc::Rc};

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
fn sevenz1648_ppmd_properties_streaming_flush_and_size_defined_finish() {
    for level in 1..=9 {
        for input in [Vec::new(), b"PPMdH must continue after ordinary flush\n".repeat(256)] {
            let config = SevenZMethodConfiguration::new(SevenZMethod::PPMD).with_options(MethodOptions::Num(level));
            let mut properties = [0; 64];
            let properties = get_options_as_properties(config.method, config.options.as_ref(), &mut properties).to_vec();
            assert_eq!(properties.len(), 5);
            let order = properties[0] as u32;
            let memory = u32::from_le_bytes(properties[1..5].try_into().unwrap());
            let state = Rc::new(RefCell::new(State::default()));
            let mut encoder = add_encoder(CountingWriter::new(Sink(state.clone())), &config).unwrap();
            let middle = input.len() / 2;
            encoder.write_all(&input[..middle]).unwrap();
            encoder.flush().unwrap();
            encoder.write_all(&input[middle..]).unwrap();
            encoder.flush().unwrap();
            encoder.write(&[]).unwrap();
            let data = state.borrow().bytes.clone();
            let mut decoder = ppmd_rust::Ppmd7Decoder::new(Cursor::new(data.clone()), order, memory).unwrap();
            let mut decoded = vec![0; input.len()];
            decoder.read_exact(&mut decoded).unwrap();
            assert_eq!(decoded, input);
            assert_eq!(decoder.finish_7z().unwrap().position(), data.len() as u64);
            encoder.write(&[]).unwrap();
            assert_eq!(state.borrow().signals, 1);
            assert!(encoder.write(b"closed").is_err());
        }
    }
}

#[test]
fn sevenz1648_ppmd_terminal_errors_cannot_turn_into_success() {
    for failure in [1, 2, 3] {
        let state = Rc::new(RefCell::new(State::default()));
        let mut encoder = add_encoder(CountingWriter::new(Sink(state.clone())),
            &SevenZMethodConfiguration::new(SevenZMethod::PPMD)).unwrap();
        encoder.write_all(b"PPMd range finish and AES finish must report failure").unwrap();
        encoder.flush().unwrap();
        state.borrow_mut().failure = failure;
        assert!(encoder.write(&[]).is_err(), "failure {failure}");
        state.borrow_mut().failure = 0;
        assert!(encoder.write(&[]).is_err());
        assert!(encoder.flush().is_err());
    }
}

#[test]
fn sevenz1648_ppmd_level_properties_are_bounded_and_invalid_options_rejected() {
    let expected = [(2,4),(3,4),(4,8),(5,8),(6,16),(7,16),(8,32),(10,32),(12,64)];
    for (i, (order, mib)) in expected.into_iter().enumerate() {
        assert_eq!(ppmd_encoder_settings(Some(&MethodOptions::Num(i as u32 + 1))).unwrap(), (order, mib * 1024 * 1024));
    }
    assert_eq!(ppmd_encoder_settings(None).unwrap(), (7, 16 * 1024 * 1024));
    assert_eq!(ppmd_encoder_settings(Some(&MethodOptions::Num(0))).unwrap(), (2, 4 * 1024 * 1024));
    assert_eq!(ppmd_encoder_settings(Some(&MethodOptions::Num(u32::MAX))).unwrap(), (12, 64 * 1024 * 1024));
    let invalid = SevenZMethodConfiguration::new(SevenZMethod::PPMD).with_options(MethodOptions::LZMA2(LZMA2Options::default()));
    assert!(add_encoder(CountingWriter::new(Vec::new()), &invalid).is_err());
}
