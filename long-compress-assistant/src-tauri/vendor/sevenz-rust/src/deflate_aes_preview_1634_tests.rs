//! New full AES/Deflate adapter only; past codec/AES matrices stay separate.
use crate::{folder::{Coder,BindPair},Archive,BlockDecoder,Error,SevenZMethod as M};
use aes::cipher::{BlockEncryptMut,KeyIvInit,generic_array::GenericArray};
use sha2::{Digest,Sha256};use std::io::{Cursor,Read,Seek,SeekFrom};
const KEY:&[u8]=&[b'k',0];
fn sample(final_block:bool,extra:bool,bad_padding:bool)->(Archive,Cursor<Vec<u8>>,Vec<u8>){
 let(mut a,data,body)=crate::deflate_preview_1633_tests::sample(final_block,extra);
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
fn sevenz1634_complete_aes_deflate_short_reads_finish_both_streams_and_receipt(){
 let(a,data,body)=sample(true,false,false);let mut data=Short(data);let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_deflate_entry(0,1024*1024,||Ok(()))).unwrap();assert_eq!(v,body);
}
#[test]
fn sevenz1634_deflate_end_never_waives_aes_padding_extra_plain_or_other_full_receipts(){
 for(final_block,extra,padding)in[(false,false,false),(true,true,false),(true,false,true)]{
  let(a,mut data,_)=sample(final_block,extra,padding);let e=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_deflate_entry(0,1024*1024,||Ok(()))).unwrap_err().to_string();assert!(e.contains(if padding{"padding"}else if extra{"coverage"}else{"did not finish"}),"{e}");
 }
 let(a,mut data,_)=sample(true,false,false);let r=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||{let _other_full=crate::aes256sha256::PreviewAes::new(Cursor::new(data.get_ref()[32..].to_vec()),&[0,0],KEY,a.folders[0].unpack_sizes[1] as usize)?;BlockDecoder::new(0,&a,KEY,&mut data).read_deflate_entry(0,1024*1024,||Ok(()))});assert!(r.unwrap_err().to_string().contains("AES 数据未完整消费"));
 for which in ["alignment","aes-length","power","properties","chain"]{let(mut a,mut data,_)=sample(true,false,false);match which{"alignment"=>a.pack_sizes[0]-=1,"aes-length"=>a.folders[0].unpack_sizes[1]+=16,"power"=>a.folders[0].coders[1].properties[0]=21,"properties"=>{a.folders[0].coders[1].properties.pop();},_=>a.folders[0].packed_streams[0]=0};assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_deflate_entry(0,1024*1024,||Ok(()))).is_err(),"{which}");}
}
