//! New full AES/Deflate64 adapter only; past codec/AES matrices stay separate.
use crate::{folder::{Coder,BindPair},Archive,BlockDecoder,Error,SevenZMethod as M};
use aes::cipher::{BlockEncryptMut,KeyIvInit,generic_array::GenericArray};
use sha2::{Digest,Sha256};use std::io::{Cursor,Read,Seek,SeekFrom};
const KEY:&[u8]=&[b'k',0];
pub(super) fn sample(final_block:bool,extra:bool,bad_padding:bool)->(Archive,Cursor<Vec<u8>>,Vec<u8>){
 let(mut a,data,body)=encrypt(crate::deflate64_preview_1638_tests::sample(final_block,extra),bad_padding);
 let crc=crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);let mut offset=0;let e=a.files[0].clone();a.files.clear();
 for(i,n)in[7,0,40000,19993].into_iter().enumerate(){let mut f=e.clone();f.name=format!("{i}.txt");f.size=n as u64;f.crc=crc.checksum(&body[offset..offset+n]) as u64;a.files.push(f);offset+=n;}
 a.folders[0].num_unpack_sub_streams=4;a.stream_map.file_folder_index=vec![Some(0);4];(a,data,body)
}
fn encrypt((mut a,data,body):(Archive,Cursor<Vec<u8>>,Vec<u8>),bad_padding:bool)->(Archive,Cursor<Vec<u8>>,Vec<u8>){
 let plain=&data.get_ref()[32..];let mut cipher=plain.to_vec();cipher.resize(plain.len().div_ceil(16)*16,0);if bad_padding{*cipher.last_mut().unwrap()=1;}
 let mut seed=KEY.to_vec();seed.extend_from_slice(&[0;8]);let key:[u8;32]=Sha256::digest(seed).into();let mut aes=cbc::Encryptor::<aes::Aes256>::new(&key.into(),&[0u8;16].into());for b in cipher.chunks_exact_mut(16){aes.encrypt_block_mut(GenericArray::from_mut_slice(b));}
 let mut c=Coder::default();c.id_size=M::ID_AES256SHA256.len();c.decompression_method_id_mut().copy_from_slice(M::ID_AES256SHA256);c.num_in_streams=1;c.num_out_streams=1;c.properties=vec![0,0];
 let f=&mut a.folders[0];f.coders.push(c);f.total_input_streams=2;f.total_output_streams=2;f.bind_pairs=vec![BindPair{in_index:0,out_index:1}];f.packed_streams=vec![1];f.unpack_sizes.push(plain.len() as u64);
 a.pack_sizes[0]=cipher.len() as u64;a.pack_crcs[0]=crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC).checksum(&cipher) as u64;
 let mut bytes=vec![0;32];bytes.extend(cipher);(a,Cursor::new(bytes),body)
}
struct Short(Cursor<Vec<u8>>);
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);self.0.read(&mut b[..n])}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.0.seek(p)}}
#[test]
fn sevenz1641_shared_aes_short_reads_cross_all_files_and_retain_only_selected(){
 for(i,start,n)in[(0,0,7),(1,7,0),(2,7,40000),(3,40007,19993)]{let(a,data,body)=sample(true,false,false);let mut data=Short(data);let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_solid_deflate64_entry(i,1024*1024,64*1024*1024,||Ok(()))).unwrap();assert_eq!(v,&body[start..start+n]);}
}
#[test]
fn sevenz1641_shared_end_never_waives_aes_padding_other_full_receipts_or_parameters(){
 for(end,extra,padding)in[(false,false,false),(true,true,false),(true,false,true)]{let(a,mut data,_)=sample(end,extra,padding);let e=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_solid_deflate64_entry(0,1024*1024,64*1024*1024,||Ok(()))).unwrap_err().to_string();assert!(e.contains(if padding{"padding"}else if extra{"coverage"}else{"did not finish"}),"{e}");}
 let(a,mut data,_)=sample(true,false,false);let r=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||{let _other=crate::aes256sha256::PreviewAes::new(Cursor::new(data.get_ref()[32..].to_vec()),&[0,0],KEY,a.folders[0].unpack_sizes[1] as usize)?;BlockDecoder::new(0,&a,KEY,&mut data).read_solid_deflate64_entry(2,1024*1024,64*1024*1024,||Ok(()))});assert!(r.unwrap_err().to_string().contains("AES 数据未完整消费"));
 for which in ["alignment","aes-size","power","properties","chain","before-crc","after-crc","folder-crc","pack-crc"]{let(mut a,mut data,_)=sample(true,false,false);match which{"alignment"=>a.pack_sizes[0]-=1,"aes-size"=>a.folders[0].unpack_sizes[1]+=16,"power"=>a.folders[0].coders[1].properties[0]=21,"properties"=>{a.folders[0].coders[1].properties.pop();},"chain"=>a.folders[0].packed_streams[0]=0,"before-crc"=>a.files[0].crc^=1,"after-crc"=>a.files[3].crc^=1,"folder-crc"=>a.folders[0].crc^=1,_=>a.pack_crcs[0]^=1};assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_solid_deflate64_entry(2,1024*1024,64*1024*1024,||Ok(()))).is_err(),"{which}");}
 let(a,mut data,_)=sample(true,false,false);let mut calls=0;let e=BlockDecoder::new(0,&a,KEY,&mut data).read_solid_deflate64_entry(3,1024*1024,64*1024*1024,||{calls+=1;if calls==6{Err(Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));assert_eq!(calls,6);
}
#[test]
fn sevenz1641_encrypted_shared_huffman_prefetch_does_not_hide_extra_plaintext(){
 for extra in 0..=3 {
  let(mut a,mut data,_)=crate::deflate64_preview_1638_tests::sample(true,false);let e=a.files[0].clone();a.files=vec![e.clone(),e];for(i,e)in a.files.iter_mut().enumerate(){e.name=format!("{i}.txt");e.size=0;e.crc=0;}
  a.folders[0].num_unpack_sub_streams=2;a.stream_map.file_folder_index=vec![Some(0);2];a.folders[0].unpack_sizes[0]=0;a.folders[0].crc=0;
  data.get_mut().truncate(32);data.get_mut().extend([3,0]);data.get_mut().extend(vec![0;extra]);let(a,mut data,_)=encrypt((a,data,vec![]),false);
  let r=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_solid_deflate64_entry(1,1024*1024,64*1024*1024,||Ok(())));
  if extra==0{assert!(r.unwrap().is_empty());}else{assert!(r.unwrap_err().to_string().contains("coverage"));}
 }
}
