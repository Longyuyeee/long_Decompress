//! Only the new shared PPMd prefix adapter; complete-reader tests stay separate.
use crate::{Archive,BlockDecoder,Error,SevenZArchiveEntry};
use std::io::{Cursor,Read,Seek,SeekFrom};
const KEY:&[u8]=&[b'k',0];
fn sample()->(Archive,Cursor<Vec<u8>>,Vec<u8>,usize){
 let(mut a,data,body)=crate::ppmd_preview_1628_tests::sample(false,true,false);
 let cut=body.len()/3;
 a.files=body.chunks(cut).enumerate().map(|(i,b)|{let mut e=SevenZArchiveEntry::default();e.name=format!("{i}.txt");e.has_stream=true;e.size=b.len() as u64;e.has_crc=true;e.crc=0; e}).collect();
 a.folders[0].num_unpack_sub_streams=a.files.len();a.stream_map.file_folder_index=vec![Some(0);a.files.len()];a.folders[0].crc^=1;
 (a,data,body,cut)
}
struct Short(Cursor<Vec<u8>>);
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);self.0.read(&mut b[..n])}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.0.seek(p)}}
#[test]
fn sevenz1632_shared_ppmd_short_reads_retain_only_selected_prefix_without_unread_crc_or_finish(){
 let(a,data,body,offset)=sample();let mut data=Short(data);
 let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_solid_entry_prefix(1,1024,(offset+1024) as u64,||Ok(()))).unwrap();assert_eq!(v,&body[offset..offset+1024]);
}
#[test]
fn sevenz1632_shared_prefix_never_waives_full_aes_and_bounds_skip_before_retaining(){
 let(a,mut data,_,_)=sample();
 let r=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||{let _full=crate::aes256sha256::PreviewAes::new(Cursor::new(data.get_ref()[32..].to_vec()),&[0,0],KEY,a.folders[0].unpack_sizes[1] as usize)?;BlockDecoder::new(0,&a,KEY,&mut data).read_solid_entry_prefix(1,1024,64*1024*1024,||Ok(()))});assert!(r.unwrap_err().to_string().contains("AES 数据未完整消费"));
 for which in ["order","memory","props","mapping","anti","sum","packed-map","alignment","power","range","cap"]{
  let(mut a,mut data,_,offset)=sample();match which{
   "order"=>a.folders[0].coders[0].properties[0]=1,
   "memory"=>a.folders[0].coders[0].properties[1..5].copy_from_slice(&(65u32*1024*1024).to_le_bytes()),
   "props"=>{a.folders[0].coders[0].properties.pop();},
   "mapping"=>a.stream_map.file_folder_index[0]=None,"anti"=>a.files[0].is_anti_item=true,"sum"=>a.files[1].size+=1,
   "packed-map"=>a.folders[0].packed_streams[0]=0,"alignment"=>a.pack_sizes[0]-=1,"power"=>a.folders[0].coders[1].properties[0]=21,_=>()}
  assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_solid_entry_prefix(1,if which=="cap"{1024*1024+1}else{1024},if which=="range"{(offset+1023) as u64}else{64*1024*1024},||Ok(()))).is_err(),"{which}");
 }
 let(a,mut data,_,_)=sample();let mut n=0;let r=BlockDecoder::new(0,&a,KEY,&mut data).read_solid_entry_prefix(1,1024,64*1024*1024,||{n+=1;if n==3{Err(Error::other("预览已取消"))}else{Ok(())}});assert!(r.unwrap_err().to_string().contains("取消"));assert_eq!(n,3);
}
