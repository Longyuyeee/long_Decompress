//! Ordinary shared Deflate64 stream boundaries, all-file CRC and real finish.
use crate::{Archive,BlockDecoder,Error};use std::io::{Cursor,Read,Seek,SeekFrom};
fn sample(end:bool,extra:bool)->(Archive,Cursor<Vec<u8>>,Vec<u8>){
 let(mut a,data,body)=crate::deflate64_preview_1638_tests::sample(end,extra);
 let crc=crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);let mut offset=0;let e=a.files[0].clone();a.files.clear();
 for (i,n) in [7,0,40000,19993].into_iter().enumerate(){let mut f=e.clone();f.name=format!("{i}.txt");f.size=n as u64;f.crc=crc.checksum(&body[offset..offset+n]) as u64;a.files.push(f);offset+=n;}
 a.folders[0].num_unpack_sub_streams=4;a.stream_map.file_folder_index=vec![Some(0);4];(a,data,body)
}
struct Short(Cursor<Vec<u8>>);
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);self.0.read(&mut b[..n])}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.0.seek(p)}}
#[test]
fn sevenz1640_short_reads_cross_empty_and_multiple_files_only_keep_selected(){
 for(i,start,n)in[(0,0,7),(1,7,0),(2,7,40000),(3,40007,19993)]{let(a,data,body)=sample(true,false);let mut data=Short(data);let v=BlockDecoder::new(0,&a,&[],&mut data).read_solid_deflate64_entry(i,1024*1024,64*1024*1024,||Ok(())).unwrap();assert_eq!(v,&body[start..start+n]);}
}
#[test]
fn sevenz1640_selected_body_cannot_waive_terminal_or_any_mapped_crc_and_cancel(){
 for(end,extra)in[(false,false),(true,true)]{let(a,mut data,_)=sample(end,extra);assert!(BlockDecoder::new(0,&a,&[],&mut data).read_solid_deflate64_entry(0,1024*1024,64*1024*1024,||Ok(())).is_err());}
 for which in ["before-crc","after-crc","empty-crc","folder-crc","pack-crc","count","sum","map","packed","missing-crc","input-limit","output-limit","properties","anti"]{let(mut a,mut data,_)=sample(true,false);match which{
  "before-crc"=>a.files[0].crc^=1,"after-crc"=>a.files[3].crc^=1,"empty-crc"=>a.files[1].crc=1,"folder-crc"=>a.folders[0].crc^=1,"pack-crc"=>a.pack_crcs[0]^=1,"count"=>a.folders[0].num_unpack_sub_streams-=1,"sum"=>a.files[3].size+=1,"map"=>a.stream_map.file_folder_index[3]=None,"packed"=>a.folders[0].packed_streams[0]=1,"missing-crc"=>{a.folders[0].has_crc=false;a.files[3].has_crc=false;},"input-limit"=>a.pack_sizes[0]=64*1024*1024+1,"output-limit"=>a.folders[0].unpack_sizes[0]=64*1024*1024+1,"properties"=>a.folders[0].coders[0].properties.push(1),_=>a.files[0].is_anti_item=true};assert!(BlockDecoder::new(0,&a,&[],&mut data).read_solid_deflate64_entry(2,1024*1024,64*1024*1024,||Ok(())).is_err(),"{which}");}
 let(a,mut data,_)=sample(true,false);let mut calls=0;let e=BlockDecoder::new(0,&a,&[],&mut data).read_solid_deflate64_entry(3,1024*1024,64*1024*1024,||{calls+=1;if calls==6{Err(Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));assert_eq!(calls,6);
}
#[test]
fn sevenz1640_shared_huffman_terminal_extent_excludes_prefetched_whole_bytes(){
 for extra in 0..=3 {
  let(mut a,mut data,_)=sample(true,false);let mut packed=vec![3,0];packed.extend(vec![0;extra]);
  for e in &mut a.files {e.size=0;e.crc=0;}a.folders[0].unpack_sizes[0]=0;a.folders[0].crc=0;
  a.pack_sizes[0]=packed.len() as u64;a.pack_crcs[0]=crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC).checksum(&packed) as u64;
  data.get_mut().truncate(32);data.get_mut().extend(packed);
  let result=BlockDecoder::new(0,&a,&[],&mut data).read_solid_deflate64_entry(1,1024*1024,64*1024*1024,||Ok(()));
  if extra==0 {assert!(result.unwrap().is_empty());}else{assert!(result.unwrap_err().to_string().contains("coverage"));}
 }
}
