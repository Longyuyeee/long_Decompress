//! New single/shared Bzip2 partial adapter; full decoding stays independent.
use crate::{BlockDecoder,Error};use std::io::{Cursor,Read,Seek,SeekFrom};
const KEY:&[u8]=&[b'k',0];
struct Short{data:Cursor<Vec<u8>>,read:usize}
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);let n=self.data.read(&mut b[..n])?;self.read+=n;Ok(n)}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.data.seek(p)}}
#[test]
fn sevenz1645_short_reads_single_shared_skip_empty_and_drain_before_refill(){
 for encrypted in [false,true]{for shared in [false,true]{let mut reads=None;for(i,offset,cap)in if shared{vec![(0,0,3),(2,7,1024),(3,40007,1024)]}else{vec![(0,0,1024)]}{let(mut a,data,body)=crate::bzip2_preview_1644_tests::sample(shared,encrypted,"");for e in &mut a.files{e.crc^=1;}a.folders[0].crc^=1;a.pack_crcs[0]^=1;let mut data=Short{data,read:0};
  let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,if encrypted{KEY}else{&[]},&mut data).read_bzip2_entry_prefix(i,cap,64*1024*1024,||Ok(()))).unwrap();assert_eq!(v,&body[offset..offset+cap]);if let Some(n)=reads{assert_eq!(data.read,n,"buffered block must drain before refill");}else{reads=Some(data.read);}
 }}}
 let(mut a,mut data,body)=crate::bzip2_preview_1644_tests::sample(true,false,"");a.files[3].size+=1<<40;a.folders[0].unpack_sizes[0]+=1<<40;a.pack_sizes[0]=1<<40;
 let v=BlockDecoder::new(0,&a,&[],&mut data).read_bzip2_entry_prefix(2,1024,64*1024*1024,||Ok(())).unwrap();assert_eq!(v,&body[7..1031]);assert!(data.position()<32+32768);
}
#[test]
fn sevenz1645_exact_dependency_budget_bad_mapping_observed_short_end_and_parameters_reject(){
 for budget in [1031,1030]{let(a,mut data,_)=crate::bzip2_preview_1644_tests::sample(true,false,"");assert_eq!(BlockDecoder::new(0,&a,&[],&mut data).read_bzip2_entry_prefix(2,1024,budget,||Ok(())).is_ok(),budget==1031);}
 for encrypted in [false,true]{let(mut a,mut data,_)=crate::bzip2_preview_1644_tests::sample(true,encrypted,"");a.files[3].size+=10000;a.folders[0].unpack_sizes[0]+=10000;assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,if encrypted{KEY}else{&[]},&mut data).read_bzip2_entry_prefix(3,19994,64*1024*1024,||Ok(()))).is_err());}
 for which in ["mapping","sum","count","anti","properties","packed","cap","decoded","alignment","aes-size","power","bad-before"]{let(mut a,mut data,_)=crate::bzip2_preview_1644_tests::sample(true,true,"");match which{"mapping"=>a.stream_map.file_folder_index[0]=None,"sum"=>a.files[0].size+=1,"count"=>a.folders[0].num_unpack_sub_streams-=1,"anti"=>a.files[0].is_anti_item=true,"properties"=>a.folders[0].coders[0].properties.push(1),"packed"=>a.folders[0].packed_streams[0]=0,"alignment"=>a.pack_sizes[0]-=1,"aes-size"=>a.folders[0].unpack_sizes[1]+=16,"power"=>a.folders[0].coders[1].properties[0]=21,"bad-before"=>data.get_mut()[32]^=255,_=>{}};assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_bzip2_entry_prefix(2,if which=="cap"{1048577}else{1024},if which=="decoded"{64*1024*1024+1}else{64*1024*1024},||Ok(()))).is_err(),"{which}");}
}
#[test]
fn sevenz1645_unread_tail_crc_and_partial_aes_do_not_waive_complete_receipts_or_cancel(){
 for encrypted in [false,true]{for mutation in ["footer","extra"]{let(a,mut data,body)=crate::bzip2_preview_1644_tests::sample(true,encrypted,mutation);let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,if encrypted{KEY}else{&[]},&mut data).read_bzip2_entry_prefix(2,1024,64*1024*1024,||Ok(()))).unwrap();assert_eq!(v,&body[7..1031]);assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,if encrypted{KEY}else{&[]},&mut data).read_bzip2_entry(2,1024*1024,64*1024*1024,||Ok(()))).is_err());}}
 let(a,mut data,_)=crate::bzip2_preview_1644_tests::sample(true,true,"");let r=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||{let _full=crate::aes256sha256::PreviewAes::new(Cursor::new(data.get_ref()[32..].to_vec()),&[0,0],KEY,a.folders[0].unpack_sizes[1] as usize)?;BlockDecoder::new(0,&a,KEY,&mut data).read_bzip2_entry_prefix(2,1024,64*1024*1024,||Ok(()))});assert!(r.unwrap_err().to_string().contains("未完整消费"));
 let(a,mut data,_)=crate::bzip2_preview_1644_tests::sample(true,false,"");let mut calls=0;let e=BlockDecoder::new(0,&a,&[],&mut data).read_bzip2_entry_prefix(2,1024,64*1024*1024,||{calls+=1;if calls==4{Err(Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));assert_eq!(calls,4);
}
