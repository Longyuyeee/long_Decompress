//! Constructed terminal-stream cases supplement original-tool app fixtures.
use crate::{archive::StreamMap,folder::{Coder,Folder},Archive,BlockDecoder,Error,SevenZArchiveEntry,SevenZMethod as M};
use std::io::{Cursor,Read,Seek,SeekFrom};
pub(super) fn sample(final_block:bool,extra:bool)->(Archive,Cursor<Vec<u8>>,Vec<u8>){
 let body:Vec<u8>=(0..60000).map(|n|(n*13%251) as u8).collect();let n=body.len() as u16;
 let mut packed=vec![u8::from(final_block)];packed.extend(n.to_le_bytes());packed.extend((!n).to_le_bytes());packed.extend(&body);if extra{packed.push(0);}
 let crc=crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);let value=crc.checksum(&body) as u64;
 let mut c=Coder::default();c.id_size=M::ID_DEFLATE64.len();c.decompression_method_id_mut().copy_from_slice(M::ID_DEFLATE64);c.num_in_streams=1;c.num_out_streams=1;
 let f=Folder{coders:vec![c],total_input_streams:1,total_output_streams:1,bind_pairs:vec![],packed_streams:vec![0],unpack_sizes:vec![body.len() as u64],has_crc:true,crc:value,num_unpack_sub_streams:1};
 let mut e=SevenZArchiveEntry::default();e.name="selected.txt".into();e.has_stream=true;e.size=body.len() as u64;e.has_crc=true;e.crc=value;
 let mut a=Archive::default();a.files=vec![e];a.folders=vec![f];a.pack_sizes=vec![packed.len() as u64];a.pack_crcs_defined.insert(0);a.pack_crcs=vec![crc.checksum(&packed) as u64];a.stream_map=StreamMap{folder_first_pack_stream_index:vec![0],pack_stream_offsets:vec![0],folder_first_file_index:vec![0],file_folder_index:vec![Some(0)]};
 let mut data=vec![0;32];data.extend(packed);(a,Cursor::new(data),body)
}
struct Short(Cursor<Vec<u8>>);
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);self.0.read(&mut b[..n])}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.0.seek(p)}}
#[test]
fn sevenz1638_complete_deflate_short_reads_have_exact_output_and_compressed_coverage(){
 let(a,data,body)=sample(true,false);let mut data=Short(data);let v=BlockDecoder::new(0,&a,&[],&mut data).read_deflate64_entry(0,1024*1024,||Ok(())).unwrap();assert_eq!(v,body);
}
#[test]
fn sevenz1638_body_without_terminal_end_trailing_input_and_conflicting_metadata_never_complete(){
 for(final_block,extra)in[(false,false),(true,true)]{let(a,mut data,_)=sample(final_block,extra);let e=BlockDecoder::new(0,&a,&[],&mut data).read_deflate64_entry(0,1024*1024,||Ok(())).unwrap_err();assert!(e.to_string().contains(if extra{"coverage"}else{"did not finish"}),"{e}");}
 for which in ["crc","pack-crc","missing-crc","short-size","long-size","truncated","mapping","properties","shared","pack-map","input-limit"]{
  let(mut a,mut data,_)=sample(true,false);match which{
   "crc"=>a.files[0].crc^=1,"pack-crc"=>a.pack_crcs[0]^=1,"missing-crc"=>{a.files[0].has_crc=false;a.folders[0].has_crc=false;},
   "short-size"=>{a.files[0].size-=1;a.folders[0].unpack_sizes[0]-=1;},"long-size"=>{a.files[0].size+=1;a.folders[0].unpack_sizes[0]+=1;},
   "truncated"=>a.pack_sizes[0]-=1,"mapping"=>a.stream_map.file_folder_index[0]=None,"properties"=>a.folders[0].coders[0].properties.push(1),
   "shared"=>a.folders[0].num_unpack_sub_streams=2,"pack-map"=>a.folders[0].packed_streams[0]=1,_=>a.pack_sizes[0]=64*1024*1024+1}
  assert!(BlockDecoder::new(0,&a,&[],&mut data).read_deflate64_entry(0,1024*1024,||Ok(())).is_err(),"{which}");
 }
 let(a,mut data,_)=sample(true,false);let mut calls=0;let e=BlockDecoder::new(0,&a,&[],&mut data).read_deflate64_entry(0,1024*1024,||{calls+=1;if calls==4{Err(Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));assert_eq!(calls,4);
}

#[test]
fn sevenz1638_huffman_prefetch_does_not_count_trailing_whole_bytes_as_stream_data(){
 // Fixed-Huffman empty final block. Lookahead can load a following zero byte.
 for extra in 0..=3 {
  let(mut a,mut data,_)=sample(true,false);let mut packed=vec![3,0];packed.extend(vec![0;extra]);
  a.files[0].size=0;a.folders[0].unpack_sizes[0]=0;a.files[0].crc=0;a.folders[0].crc=0;
  a.pack_sizes[0]=packed.len() as u64;a.pack_crcs[0]=crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC).checksum(&packed) as u64;
  data.get_mut().truncate(32);data.get_mut().extend(packed);
  let result=BlockDecoder::new(0,&a,&[],&mut data).read_deflate64_entry(0,1024*1024,||Ok(()));
  if extra==0 {assert!(result.unwrap().is_empty());}else{assert!(result.unwrap_err().to_string().contains("coverage"));}
 }
}
