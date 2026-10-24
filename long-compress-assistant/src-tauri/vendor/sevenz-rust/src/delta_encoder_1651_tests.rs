use crate::encoders::*;
use std::{cell::RefCell,io::{self,Read,Write},rc::Rc};
#[derive(Default)]struct State{bytes:Vec<u8>,signals:usize,failure:u8}
#[derive(Clone)]struct Sink(Rc<RefCell<State>>);
impl Write for Sink{
 fn write(&mut self,b:&[u8])->io::Result<usize>{let mut s=self.0.borrow_mut();if s.failure==3 || s.failure==1&&b.is_empty(){return Err(io::Error::other("injected write"))}if b.is_empty(){s.signals+=1}s.bytes.extend_from_slice(b);Ok(b.len())}
 fn flush(&mut self)->io::Result<()>{if self.0.borrow().failure==2{Err(io::Error::other("injected flush"))}else{Ok(())}}
}
#[test]
fn sevenz1651_delta_all_distances_chunk_history_flush_and_end(){
 let data:Vec<u8>=(0..65539).map(|i|(i*31+i/7) as u8).collect();
 for distance in 1..=256u32{
  let state=Rc::new(RefCell::new(State::default()));let config=crate::SevenZMethodConfiguration::new(crate::SevenZMethod::DELTA_FILTER).with_options(crate::MethodOptions::Num(distance));let mut props=[0;64];assert_eq!(get_options_as_properties(config.method,config.options.as_ref(),&mut props),&[(distance-1) as u8]);
  let mut encoder=add_encoder(crate::lzma::CountingWriter::new(Sink(state.clone())),&config).unwrap();for chunk in data.chunks(1031){encoder.write_all(chunk).unwrap();encoder.flush().unwrap()}encoder.write(&[]).unwrap();encoder.write(&[]).unwrap();assert_eq!(state.borrow().signals,1);assert!(encoder.write(b"after finish").is_err());
  let encoded=state.borrow().bytes.clone();let expected:Vec<_>=data.iter().enumerate().map(|(i,&b)|b.wrapping_sub(if i>=distance as usize{data[i-distance as usize]}else{0})).collect();assert_eq!(encoded,expected);
  let mut decoded=Vec::new();crate::delta::DeltaReader::new(encoded.as_slice(),distance as usize).read_to_end(&mut decoded).unwrap();assert_eq!(decoded,data);
 }
}
#[test]
fn sevenz1651_delta_empty_invalid_and_failure_stays_failed(){
 for distance in [0,257]{assert!(crate::delta_encoder::DeltaEncoder::new(Vec::new(),distance).is_err());}
 let mut empty=crate::delta_encoder::DeltaEncoder::new(Vec::new(),1).unwrap();assert_eq!(empty.write(&[]).unwrap(),0);assert!(empty.write(b"late").is_err());
 for failure in [1,2,3]{let state=Rc::new(RefCell::new(State::default()));let mut encoder=crate::delta_encoder::DeltaEncoder::new(Sink(state.clone()),256).unwrap();encoder.write_all(b"start").unwrap();state.borrow_mut().failure=failure;let failed=match failure{3=>encoder.write(b"data").is_err(),2=>encoder.flush().is_err(),_=>encoder.write(&[]).is_err()};assert!(failed);state.borrow_mut().failure=0;assert!(encoder.write(&[]).is_err());assert!(encoder.flush().is_err());assert!(encoder.write(b"retry").is_err());}
}
