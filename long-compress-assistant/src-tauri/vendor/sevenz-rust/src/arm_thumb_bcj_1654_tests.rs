use crate::{bcj::{encoder::BcjEncoder, BCJFilter, SimpleReader}, encoders::*, lzma::CountingWriter, SevenZMethod as M, SevenZMethodConfiguration as C};
use std::{cell::RefCell, io::{self, Cursor, Read, Seek, SeekFrom, Write}, rc::Rc};
const INPUT: &[u8] = include_bytes!("../../../../tests/fixtures/archives/sevenz-arm-thumb-bcj-create-1654/input.bin");
const FILTERED: &[u8] = include_bytes!("../../../../tests/fixtures/archives/sevenz-arm-thumb-bcj-create-1654/filtered-0.bin");
#[derive(Default)] struct State { bytes: Vec<u8>, ends: usize, fail: u8 }
#[derive(Clone)] struct Sink(Rc<RefCell<State>>);
impl Write for Sink {
 fn write(&mut self,b:&[u8])->io::Result<usize>{let mut s=self.0.borrow_mut();if s.fail==1 || s.fail==3&&b.is_empty(){return Err(io::Error::other("injected write"));}if b.is_empty(){s.ends+=1;}s.bytes.extend_from_slice(b);Ok(b.len())}
 fn flush(&mut self)->io::Result<()>{if self.0.borrow().fail==2{Err(io::Error::other("injected flush"))}else{Ok(())}}
}
#[test]
fn sevenz1654_arm_chunks_match_independent_liblzma_without_flushing_instruction_tail(){
 for size in [1,2,3,4,5,31,4095,8191,8192,16385] {
  let state=Rc::new(RefCell::new(State::default()));let mut writer=BcjEncoder::new_arm_thumb(Sink(state.clone()));
  for chunk in INPUT.chunks(size){writer.write_all(chunk).unwrap();writer.flush().unwrap();}
  writer.write(&[]).unwrap();writer.flush().unwrap();writer.write(&[]).unwrap();assert_eq!(state.borrow().bytes,FILTERED,"chunk {size}");assert_eq!(state.borrow().ends,1);assert!(writer.write(b"late").is_err());
 }
 let mut props=[0u8;64];assert!(get_options_as_properties(M::BCJ_ARM_THUMB_FILTER,None,&mut props).is_empty());
 assert!(add_encoder(CountingWriter::new(Vec::new()),&C::new(M::BCJ_ARM_THUMB_FILTER).with_options(crate::MethodOptions::Num(4))).is_err());
}
#[test]
fn sevenz1654_arm_wrap_properties_empty_tail_and_sticky_failures(){
 for (start,filtered) in [(0x7ffffffeusize,include_bytes!("../../../../tests/fixtures/archives/sevenz-arm-thumb-bcj-create-1654/filtered-2147483646.bin").as_slice()),(0xfffffffe,include_bytes!("../../../../tests/fixtures/archives/sevenz-arm-thumb-bcj-create-1654/filtered-4294967294.bin").as_slice())] {
  let mut encoder=BCJFilter::new_arm_thumb(start,true);let mut bytes=INPUT.to_vec();encoder.code(&mut bytes);assert_eq!(bytes,filtered);
  let mut decoded=Vec::new();SimpleReader::new_arm_thumb_at(filtered,start).read_to_end(&mut decoded).unwrap();assert_eq!(decoded,INPUT);assert_eq!(crate::bcj::arm_thumb_start_position(&(start as u32).to_le_bytes()).unwrap(),start);
 }
 for len in 0..=4 {let state=Rc::new(RefCell::new(State::default()));let mut writer=BcjEncoder::new_arm_thumb(Sink(state.clone()));writer.write_all(&b"\xff\xf7\xff\xff"[..len]).unwrap();writer.write(&[]).unwrap();assert_eq!(state.borrow().bytes.len(),len);assert_eq!(state.borrow().ends,1);}
 for fail in [1,2,3] {let state=Rc::new(RefCell::new(State::default()));let mut writer=BcjEncoder::new_arm_thumb(Sink(state.clone()));writer.write_all(b"before").unwrap();state.borrow_mut().fail=fail;let failed=match fail{1=>writer.write(b"next").is_err(),2=>writer.flush().is_err(),_=>writer.write(&[]).is_err()};assert!(failed);state.borrow_mut().fail=0;assert!(writer.write(&[]).is_err());assert!(writer.flush().is_err());}
 for properties in [vec![0],vec![0;3],vec![0;5],1u32.to_le_bytes().to_vec()] {assert!(crate::bcj::arm_thumb_start_position(&properties).is_err());}assert_eq!(crate::bcj::arm_thumb_start_position(&[]).unwrap(),0);
}
struct Short(Cursor<Vec<u8>>);
impl Read for Short {fn read(&mut self,b:&mut[u8])->io::Result<usize>{let n=b.len().min(3);self.0.read(&mut b[..n])}}
impl Seek for Short {fn seek(&mut self,p:SeekFrom)->io::Result<u64>{self.0.seek(p)}}
#[test]
fn sevenz1654_arm_prefix_preserves_all_four_shared_entry_phases_and_charges_lookahead(){
 use crate::{SevenZWriter,SevenZReader,SevenZArchiveEntry as E,Password,BlockDecoder,SourceReader,SeqReader};
 for offset in 0..4usize {
  let joined:Vec<u8>=(0..1100u32).flat_map(|i|[i as u8,0xf0,(i>>8) as u8,0xf8]).collect();let before=joined[..offset].to_vec();let body=joined[offset..].to_vec();
  let mut writer=SevenZWriter::new(Cursor::new(Vec::new())).unwrap();writer.set_content_methods(vec![crate::lzma::LZMA2Options::with_preset(0).into(),C::new(M::BCJ_ARM_THUMB_FILTER)]);
  let entry=|name:&str|{let mut e=E::default();e.name=name.into();e.has_stream=true;e};
  if offset==0 {writer.push_archive_entry(entry("body.bin"),Some(Cursor::new(body.clone()))).unwrap();} else {writer.push_archive_entries(vec![entry("before.bin"),entry("body.bin")],SeqReader::new(vec![SourceReader::new(Cursor::new(before)),SourceReader::new(Cursor::new(body.clone()))])).unwrap();}
  let input=writer.finish().unwrap().into_inner();let parsed=SevenZReader::new(Cursor::new(input.clone()),input.len() as u64,Password::empty()).unwrap();let mut source=Short(Cursor::new(input));let prefix=1025;
  let bytes=if offset==0 {BlockDecoder::new(0,parsed.archive(),&[],&mut source).read_single_entry_prefix(0,prefix,||Ok(())).unwrap()} else {BlockDecoder::new(0,parsed.archive(),&[],&mut source).read_solid_entry_prefix(1,prefix,(offset+prefix+3) as u64,||Ok(())).unwrap()};assert_eq!(bytes,&body[..prefix],"offset {offset}");
  if offset>0 {assert!(BlockDecoder::new(0,parsed.archive(),&[],&mut source).read_solid_entry_prefix(1,prefix,(offset+prefix+2) as u64,||Ok(())).is_err());}
 }
}
