//! Constructed shared AES/PPMd completion checks; originals live in app fixtures.
use crate::{Archive,BlockDecoder,Error,SevenZArchiveEntry};
use std::io::{Cursor,Read,Seek,SeekFrom};
const KEY:&[u8]=&[b'k',0];
fn shared(padding:bool,flush:bool,extra:bool)->(Archive,Cursor<Vec<u8>>,Vec<u8>){
 let(mut a,data,body)=crate::ppmd_preview_1628_tests::sample(padding,flush,extra);
 let cut=body.len()/3;let crc=crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
 a.files=body.chunks(cut).enumerate().map(|(i,b)|{let mut e=SevenZArchiveEntry::default();e.name=format!("{i}.txt");e.has_stream=true;e.size=b.len() as u64;e.has_crc=true;e.crc=crc.checksum(b) as u64;e}).collect();
 a.folders[0].num_unpack_sub_streams=a.files.len();a.stream_map.file_folder_index=vec![Some(0);a.files.len()];
 (a,data,body[cut..cut*2].to_vec())
}
struct Short(Cursor<Vec<u8>>);
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);self.0.read(&mut b[..n])}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.0.seek(p)}}
#[test]
fn sevenz1630_shared_aes_short_reads_finish_receipt_and_retain_only_middle(){
 let(a,data,body)=shared(false,false,false);let mut data=Short(data);
 let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_solid_ppmd_entry(1,1024*1024,64*1024*1024,||Ok(()))).unwrap();assert_eq!(v,body);
}
#[test]
fn sevenz1630_shared_readable_flush_bad_padding_extra_plain_and_aes_alignment_never_complete(){
 for(padding,flush,extra)in[(true,false,false),(false,true,false),(false,false,true)]{
  let(a,mut data,_)=shared(padding,flush,extra);let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_solid_ppmd_entry(1,1024*1024,64*1024*1024,||Ok(())));
  let e=v.unwrap_err().to_string();if padding{assert!(e.contains("padding"),"{e}");}if flush{assert!(e.contains("range coder did not finish"),"{e}");}if extra{assert!(e.contains("plaintext was not completely consumed"),"{e}");}
 }
 for which in ["alignment","aes-length","power","order"]{let(mut a,mut data,_)=shared(false,false,false);match which{"alignment"=>a.pack_sizes[0]-=1,"aes-length"=>a.folders[0].unpack_sizes[1]+=16,"power"=>a.folders[0].coders[1].properties[0]=21,_=>a.folders[0].packed_streams[0]=0};assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_solid_ppmd_entry(1,1024*1024,64*1024*1024,||Ok(()))).is_err(),"{which}");}
}
