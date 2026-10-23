//! Shared streaming boundaries and terminal proof, plain and full AES.
use crate::{Archive,BlockDecoder,Error};use std::io::{Cursor,Read,Seek,SeekFrom};
fn sample(aes:bool,end:bool,extra:bool,padding:bool)->(Archive,Cursor<Vec<u8>>,Vec<u8>){
 let(mut a,data,body)=if aes{crate::deflate_aes_preview_1634_tests::sample(end,extra,padding)}else{crate::deflate_preview_1633_tests::sample(end,extra)};
 let crc=crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);let mut offset=0;let e=a.files[0].clone();a.files.clear();
 for (i,n) in [7,0,40000,19993].into_iter().enumerate(){let mut f=e.clone();f.name=format!("{i}.bin");f.size=n as u64;f.crc=crc.checksum(&body[offset..offset+n]) as u64;a.files.push(f);offset+=n;}
 a.folders[0].num_unpack_sub_streams=4;a.stream_map.file_folder_index=vec![Some(0);4];(a,data,body)
}
struct Short(Cursor<Vec<u8>>);
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);self.0.read(&mut b[..n])}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.0.seek(p)}}
#[test]
fn sevenz1635_short_reads_cross_empty_and_multiple_files_only_keep_selected(){
 for aes in [false,true]{for(i,start,n)in[(0,0,7),(1,7,0),(2,7,40000),(3,40007,19993)]{let(a,data,body)=sample(aes,true,false,false);let mut data=Short(data);let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,if aes{&[b'k',0]}else{&[]},&mut data).read_solid_deflate_entry(i,1024*1024,64*1024*1024,||Ok(()))).unwrap();assert_eq!(v,&body[start..start+n]);}}
}
#[test]
fn sevenz1635_selected_body_cannot_waive_terminal_crc_or_aes_padding(){
 for aes in [false,true]{for(end,extra,padding)in[(false,false,false),(true,true,false),(true,false,true)]{if padding&&!aes{continue;}let(a,mut data,_)=sample(aes,end,extra,padding);assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,if aes{&[b'k',0]}else{&[]},&mut data).read_solid_deflate_entry(0,1024*1024,64*1024*1024,||Ok(()))).is_err());}}
 for which in ["before-crc","after-crc","empty-crc","folder-crc","pack-crc","count","sum","map","packed","missing-crc","input-limit","output-limit"]{let(mut a,mut data,_)=sample(false,true,false,false);match which{
  "before-crc"=>a.files[0].crc^=1,"after-crc"=>a.files[3].crc^=1,"empty-crc"=>a.files[1].crc=1,"folder-crc"=>a.folders[0].crc^=1,"pack-crc"=>a.pack_crcs[0]^=1,"count"=>a.folders[0].num_unpack_sub_streams-=1,"sum"=>a.files[3].size+=1,"map"=>a.stream_map.file_folder_index[3]=None,"packed"=>a.folders[0].packed_streams[0]=1,"missing-crc"=>{a.folders[0].has_crc=false;a.files[3].has_crc=false;},"input-limit"=>a.pack_sizes[0]=64*1024*1024+1,_=>a.folders[0].unpack_sizes[0]=64*1024*1024+1};assert!(BlockDecoder::new(0,&a,&[],&mut data).read_solid_deflate_entry(2,1024*1024,64*1024*1024,||Ok(())).is_err(),"{which}");}
}
