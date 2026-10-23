//! Partial adapter semantics are independent of complete Deflate/AES readers.
use crate::{Archive,BlockDecoder,Error};use std::io::{Cursor,Read,Seek,SeekFrom};
struct Short{data:Cursor<Vec<u8>>,read:usize}
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);let n=self.data.read(&mut b[..n])?;self.read+=n;Ok(n)}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.data.seek(p)}}
fn sample(aes:bool,end:bool)->(Archive,Short,Vec<u8>){let(a,data,body)=if aes{crate::deflate_aes_preview_1634_tests::sample(end,false,false)}else{crate::deflate_preview_1633_tests::sample(end,false)};(a,Short{data,read:0},body)}
#[test]
fn sevenz1636_partial_short_reads_do_not_consume_or_certify_unread_tail(){
 for aes in [false,true]{let(mut a,mut data,body)=sample(aes,false);a.files[0].crc^=1;a.folders[0].crc^=1;a.pack_crcs[0]^=1;
  let bytes=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,if aes{&[b'k',0]}else{&[]},&mut data).read_deflate_entry_prefix(0,1024,||Ok(()))).unwrap();assert_eq!(bytes,&body[..1024]);assert_eq!(data.read,32768);
 }
 // Large declared packed/output sizes never become allocations or read targets.
 let(mut a,mut data,body)=sample(false,false);a.files[0].size=1<<40;a.folders[0].unpack_sizes[0]=1<<40;a.pack_sizes[0]=1<<40;
 let bytes=BlockDecoder::new(0,&a,&[],&mut data).read_deflate_entry_prefix(0,1024,||Ok(())).unwrap();assert_eq!(bytes,&body[..1024]);assert_eq!(data.read,32768);
}
#[test]
fn sevenz1636_short_declared_streams_cancel_and_other_full_aes_receipts_remain_errors(){
 for aes in [false,true]{let(mut a,mut data,_)=sample(aes,true);a.files[0].size+=1;a.folders[0].unpack_sizes[0]+=1;assert!(BlockDecoder::new(0,&a,if aes{&[b'k',0]}else{&[]},&mut data).read_deflate_entry_prefix(0,60000,||Ok(())).is_err());}
 let(a,mut data,_)=sample(true,false);let value=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||{let _full=crate::aes256sha256::PreviewAes::new(Cursor::new(data.data.get_ref()[32..].to_vec()),&[0,0],&[b'k',0],a.folders[0].unpack_sizes[1] as usize)?;BlockDecoder::new(0,&a,&[b'k',0],&mut data).read_deflate_entry_prefix(0,1024,||Ok(()))});assert!(value.unwrap_err().to_string().contains("未完整消费"));
 let(a,mut data,_)=sample(false,false);let mut calls=0;let error=BlockDecoder::new(0,&a,&[],&mut data).read_deflate_entry_prefix(0,1024,||{calls+=1;if calls==4{Err(Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(error.to_string().contains("取消"));assert_eq!(calls,4);
}
