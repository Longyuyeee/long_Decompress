//! Shared partial streaming, mapping, terminal and scope semantics.
use crate::{Archive,BlockDecoder,Error};use std::io::{Cursor,Read,Seek,SeekFrom};
struct Short{data:Cursor<Vec<u8>>,read:usize}
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);let n=self.data.read(&mut b[..n])?;self.read+=n;Ok(n)}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.data.seek(p)}}
fn sample(aes:bool,end:bool)->(Archive,Short,Vec<u8>){
 let(mut a,data,body)=if aes{crate::deflate_aes_preview_1634_tests::sample(end,false,false)}else{crate::deflate_preview_1633_tests::sample(end,false)};
 let crc=crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);let mut offset=0;let e=a.files[0].clone();a.files.clear();
 for(i,n)in [40000,0,15000,5000].into_iter().enumerate(){let mut f=e.clone();f.name=format!("{i}.txt");f.size=n as u64;f.crc=crc.checksum(&body[offset..offset+n]) as u64;a.files.push(f);offset+=n;}
 a.folders[0].num_unpack_sub_streams=4;a.stream_map.file_folder_index=vec![Some(0);4];(a,Short{data,read:0},body)
}
#[test]
fn sevenz1637_short_reads_skip_and_cross_empty_dependencies_only_retain_selected(){
 for aes in [false,true]{for(i,offset)in [(0,0),(2,40000),(3,55000)]{let(mut a,mut data,body)=sample(aes,false);for e in &mut a.files{e.crc^=1;}a.folders[0].crc^=1;a.pack_crcs[0]^=1;
  let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,if aes{&[b'k',0]}else{&[]},&mut data).read_solid_deflate_entry_prefix(i,1024,64*1024*1024,||Ok(()))).unwrap();assert_eq!(v,&body[offset..offset+1024]);
 }}
 let(mut a,mut data,body)=sample(false,false);a.files[3].size=(1<<40)-55000;a.folders[0].unpack_sizes[0]=1<<40;a.pack_sizes[0]=1<<40;
 let v=BlockDecoder::new(0,&a,&[],&mut data).read_solid_deflate_entry_prefix(0,1024,64*1024*1024,||Ok(())).unwrap();assert_eq!(v,&body[..1024]);assert_eq!(data.read,32768);
}
#[test]
fn sevenz1637_exact_dependency_budget_mapping_and_observed_short_end_are_required(){
 for budget in [41024,41023]{let(a,mut data,_)=sample(false,false);assert_eq!(BlockDecoder::new(0,&a,&[],&mut data).read_solid_deflate_entry_prefix(2,1024,budget,||Ok(())).is_ok(),budget==41024);}
 for aes in [false,true]{let(mut a,mut data,_)=sample(aes,true);a.files[3].size+=10000;a.folders[0].unpack_sizes[0]+=10000;assert!(BlockDecoder::new(0,&a,if aes{&[b'k',0]}else{&[]},&mut data).read_solid_deflate_entry_prefix(3,5000,64*1024*1024,||Ok(())).is_err());}
 for which in ["mapping","sum","count","anti","properties","packed","cap","decoded"]{let(mut a,mut data,_)=sample(false,false);match which{"mapping"=>a.stream_map.file_folder_index[0]=None,"sum"=>a.files[0].size+=1,"count"=>a.folders[0].num_unpack_sub_streams-=1,"anti"=>a.files[0].is_anti_item=true,"properties"=>a.folders[0].coders[0].properties.push(1),"packed"=>a.folders[0].packed_streams[0]=1,_=>{}};assert!(BlockDecoder::new(0,&a,&[],&mut data).read_solid_deflate_entry_prefix(2,if which=="cap"{1048577}else{1024},if which=="decoded"{64*1024*1024+1}else{64*1024*1024},||Ok(())).is_err(),"{which}");}
}
#[test]
fn sevenz1637_skip_cancellation_and_other_full_aes_receipts_cannot_be_waived(){
 let(a,mut data,_)=sample(false,false);let mut n=0;let e=BlockDecoder::new(0,&a,&[],&mut data).read_solid_deflate_entry_prefix(2,1024,64*1024*1024,||{n+=1;if n==4{Err(Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));assert_eq!(n,4);
 let(a,mut data,_)=sample(true,false);let r=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||{let _full=crate::aes256sha256::PreviewAes::new(Cursor::new(data.data.get_ref()[32..].to_vec()),&[0,0],&[b'k',0],a.folders[0].unpack_sizes[1] as usize)?;BlockDecoder::new(0,&a,&[b'k',0],&mut data).read_solid_deflate_entry_prefix(2,1024,64*1024*1024,||Ok(()))});assert!(r.unwrap_err().to_string().contains("未完整消费"));
}
