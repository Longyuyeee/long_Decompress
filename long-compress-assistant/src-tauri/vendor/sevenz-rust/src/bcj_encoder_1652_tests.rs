use crate::{bcj::{encoder::X86Encoder, BCJFilter, SimpleReader}, encoders::*, lzma::CountingWriter, SevenZMethod as M, SevenZMethodConfiguration as C};
use std::{cell::RefCell, io::{self, Read, Write}, rc::Rc};
const INPUT: &[u8] = include_bytes!("../../../../tests/fixtures/archives/sevenz-x86-bcj-create-1652/input.bin");
const FILTERED: &[u8] = include_bytes!("../../../../tests/fixtures/archives/sevenz-x86-bcj-create-1652/filtered-0.bin");
#[derive(Default)]
struct State { bytes: Vec<u8>, ends: usize, fail: u8 }
#[derive(Clone)]
struct Sink(Rc<RefCell<State>>);
impl Write for Sink {
 fn write(&mut self,b:&[u8])->io::Result<usize>{let mut s=self.0.borrow_mut();if s.fail==1 || s.fail==3&&b.is_empty(){return Err(io::Error::other("injected write"));}if b.is_empty(){s.ends+=1;}s.bytes.extend_from_slice(b);Ok(b.len())}
 fn flush(&mut self)->io::Result<()>{if self.0.borrow().fail==2{Err(io::Error::other("injected flush"))}else{Ok(())}}
}
#[test]
fn sevenz1652_bcj_chunks_match_independent_liblzma_and_keep_flush_tail(){
 for size in [1,2,3,4,5,31,4095,8191,8192,16385] {
  let state=Rc::new(RefCell::new(State::default()));let mut writer=X86Encoder::new(Sink(state.clone()));
  for chunk in INPUT.chunks(size){writer.write_all(chunk).unwrap();writer.flush().unwrap();}
  writer.write(&[]).unwrap();writer.flush().unwrap();writer.write(&[]).unwrap();
  assert_eq!(state.borrow().bytes,FILTERED,"chunk {size}");assert_eq!(state.borrow().ends,1);assert!(writer.write(b"late").is_err());
 }
 let mut props=[0u8;64];assert!(get_options_as_properties(M::BCJ_X86_FILTER,None,&mut props).is_empty());
 assert!(add_encoder(CountingWriter::new(Vec::new()),&C::new(M::BCJ_X86_FILTER).with_options(crate::MethodOptions::Num(4))).is_err());
}
#[test]
fn sevenz1652_bcj_position_wrap_properties_and_sticky_failures(){
 for (start,filtered) in [(0x7ffffffdusize,include_bytes!("../../../../tests/fixtures/archives/sevenz-x86-bcj-create-1652/filtered-2147483645.bin").as_slice()),(0xfffffffd,include_bytes!("../../../../tests/fixtures/archives/sevenz-x86-bcj-create-1652/filtered-4294967293.bin").as_slice())] {
  let mut encoder=BCJFilter::new_x86(start,true);let mut bytes=INPUT.to_vec();encoder.code(&mut bytes);assert_eq!(bytes,filtered);
  let mut decoded=Vec::new();SimpleReader::new_x86_at(filtered,start).read_to_end(&mut decoded).unwrap();assert_eq!(decoded,INPUT);
 }
 for len in 0..=5 {let state=Rc::new(RefCell::new(State::default()));let mut writer=X86Encoder::new(Sink(state.clone()));writer.write_all(&b"\xe8\xff\xff\xff\xff"[..len]).unwrap();writer.write(&[]).unwrap();assert_eq!(state.borrow().bytes.len(),len);assert_eq!(state.borrow().ends,1);}
 for fail in [1,2,3] {let state=Rc::new(RefCell::new(State::default()));let mut writer=X86Encoder::new(Sink(state.clone()));writer.write_all(b"before").unwrap();state.borrow_mut().fail=fail;let failed=match fail{1=>writer.write(b"next").is_err(),2=>writer.flush().is_err(),_=>writer.write(&[]).is_err()};assert!(failed);state.borrow_mut().fail=0;assert!(writer.write(&[]).is_err());assert!(writer.flush().is_err());}
 for properties in [vec![0],vec![0;3],vec![0;5]] {assert!(crate::bcj::x86_start_position(&properties).is_err());}
}
