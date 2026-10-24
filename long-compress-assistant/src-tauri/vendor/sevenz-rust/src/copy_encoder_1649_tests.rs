use super::*;
use std::{cell::RefCell, io, rc::Rc};
#[derive(Default)]struct State { bytes: Vec<u8>, signals: usize, failure: u8 }
#[derive(Clone)]struct Sink(Rc<RefCell<State>>);
impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut state=self.0.borrow_mut();
        if state.failure==1 && bytes.is_empty() { return Err(io::Error::other("injected terminal write")); }
        if bytes.is_empty() { state.signals+=1; }
        state.bytes.extend_from_slice(bytes); Ok(bytes.len())
    }
    fn flush(&mut self)->io::Result<()> { if self.0.borrow().failure==2 {Err(io::Error::other("injected terminal flush"))}else{Ok(())} }
}
#[test]
fn sevenz1649_copy_exact_bytes_flush_continuation_terminal_and_options() {
    for input in [Vec::new(),(0..65539).map(|i|(i*31%256) as u8).collect()] {
        let state=Rc::new(RefCell::new(State::default()));
        let config=SevenZMethodConfiguration::new(SevenZMethod::COPY);
        assert!(get_options_as_properties(config.method,None,&mut[0;64]).is_empty());
        let mut encoder=add_encoder(CountingWriter::new(Sink(state.clone())),&config).unwrap();
        let middle=input.len()/2;
        encoder.write_all(&input[..middle]).unwrap();encoder.flush().unwrap();
        encoder.write_all(&input[middle..]).unwrap();encoder.write(&[]).unwrap();
        assert_eq!(state.borrow().bytes,input);assert_eq!(state.borrow().signals,1);
        encoder.write(&[]).unwrap();assert_eq!(state.borrow().signals,1);
        assert!(encoder.write(b"after finish").is_err());encoder.flush().unwrap();
    }
    assert!(add_encoder(CountingWriter::new(Vec::new()),&SevenZMethodConfiguration::new(SevenZMethod::COPY).with_options(MethodOptions::Num(9))).is_err());
}
#[test]
fn sevenz1649_copy_terminal_failure_remains_failed() {
    for failure in [1,2] {
        let state=Rc::new(RefCell::new(State::default()));
        let mut encoder=add_encoder(CountingWriter::new(Sink(state.clone())),&SevenZMethodConfiguration::new(SevenZMethod::COPY)).unwrap();
        encoder.write_all(b"unaligned data for downstream AES").unwrap();encoder.flush().unwrap();
        state.borrow_mut().failure=failure;assert!(encoder.write(&[]).is_err());
        state.borrow_mut().failure=0;assert!(encoder.write(&[]).is_err());assert!(encoder.flush().is_err());assert!(encoder.write(b"retry").is_err());
    }
}

#[test]
fn sevenz1649_copy_zero_crc_metadata_independent_readback() {
    use crate::{SevenZArchiveEntry,SevenZWriter,SourceReader,SeqReader};
    use std::{io::Cursor,path::Path,process::Command};
    let zero_crc=vec![0x9d,0x0a,0xd9,0x6d];
    assert_eq!(crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC).checksum(&zero_crc),0);
    let dir=tempfile::tempdir().unwrap();
    for (index,solid,bodies) in [
        (0,false,vec![b"nonzero CRC".to_vec(),Vec::new()]),
        (1,true,vec![Vec::new(),Vec::new()]),
        (2,true,vec![zero_crc.clone()]),
        (3,false,vec![b"nonzero CRC".to_vec(),zero_crc]),
    ] {
        let file=dir.path().join(format!("case-{index}.7z"));
        let mut w=SevenZWriter::create(&file).unwrap();w.set_content_methods(vec![SevenZMethodConfiguration::new(SevenZMethod::COPY)]);
        let entries:Vec<_>=bodies.iter().enumerate().map(|(i,_)|{let mut e=SevenZArchiveEntry::default();e.name=format!("file-{i}");e.has_stream=true;e.is_anti_item=true;e}).collect();
        if solid {w.push_archive_entries(entries,SeqReader::new(bodies.into_iter().map(|b|SourceReader::new(Cursor::new(b))).collect())).unwrap();}
        else {for (entry,body) in entries.into_iter().zip(bodies){w.push_archive_entry(entry,Some(Cursor::new(body))).unwrap();}}
        w.finish().unwrap();
        let output=Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/archive-engine/7z.exe")).args(["t","-y"]).arg(&file).output().unwrap();
        assert!(output.status.success(),"case {index}: {} {}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
    }
}
