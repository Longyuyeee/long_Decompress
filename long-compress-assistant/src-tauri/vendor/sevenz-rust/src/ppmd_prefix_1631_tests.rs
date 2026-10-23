//! New partial PPMd adapter scope, separate from complete-reader checks.
use crate::{BlockDecoder,Error};use std::io::{Cursor,Read,Seek,SeekFrom};
const KEY:&[u8]=&[b'k',0];
struct Short(Cursor<Vec<u8>>);
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);self.0.read(&mut b[..n])}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.0.seek(p)}}
#[test]
fn sevenz1631_partial_ppmd_short_reads_never_claim_unread_finish_or_crc(){
 let(mut a,data,body)=crate::ppmd_preview_1628_tests::sample(false,true,false);a.files[0].crc^=1;a.folders[0].crc^=1;let mut data=Short(data);
 let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_single_entry_prefix(0,1024,||Ok(()))).unwrap();assert_eq!(v,&body[..1024]);
}
#[test]
fn sevenz1631_partial_ppmd_never_waives_full_aes_receipt_and_retains_bounds_and_cancel(){
 let(a,mut data,_)=crate::ppmd_preview_1628_tests::sample(false,false,false);
 let r=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||{let _incomplete=crate::aes256sha256::PreviewAes::new(Cursor::new(data.get_ref()[32..].to_vec()),&[0,0],KEY,a.folders[0].unpack_sizes[1] as usize)?;BlockDecoder::new(0,&a,KEY,&mut data).read_single_entry_prefix(0,1024,||Ok(()))});assert!(r.unwrap_err().to_string().contains("AES 数据未完整消费"));
 for which in ["order","memory","mapping","aes-length","power","cap"]{
  let(mut a,mut data,_)=crate::ppmd_preview_1628_tests::sample(false,false,false);match which{"order"=>a.folders[0].coders[0].properties[0]=1,"memory"=>a.folders[0].coders[0].properties[1..5].copy_from_slice(&(65u32*1024*1024).to_le_bytes()),"mapping"=>a.stream_map.file_folder_index[0]=None,"aes-length"=>a.folders[0].unpack_sizes[1]+=16,"power"=>a.folders[0].coders[1].properties[0]=21,_=>()};
  assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_single_entry_prefix(0,if which=="cap"{1024*1024+1}else{1024},||Ok(()))).is_err(),"{which}");
 }
 let(a,mut data,_)=crate::ppmd_preview_1628_tests::sample(false,false,false);let mut n=0;let e=BlockDecoder::new(0,&a,KEY,&mut data).read_single_entry_prefix(0,1024,||{n+=1;if n==3{Err(Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));
}
