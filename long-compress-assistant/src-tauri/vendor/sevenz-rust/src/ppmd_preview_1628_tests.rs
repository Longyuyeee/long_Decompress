//! Constructed AES/PPMd adapter negatives, separate from original-tool fixtures.
use crate::{archive::StreamMap,folder::{Coder,Folder,BindPair},Archive,SevenZArchiveEntry,SevenZMethod as M,BlockDecoder,Error};
use aes::cipher::{BlockEncryptMut,KeyIvInit,generic_array::GenericArray};
use sha2::{Digest,Sha256};
use std::io::{Cursor,Read,Seek,SeekFrom,Write};
const KEY:&[u8]=&[b'k',0];
fn coder(id:&[u8],properties:Vec<u8>)->Coder{let mut c=Coder::default();c.id_size=id.len();c.decompression_method_id_mut().copy_from_slice(id);c.num_in_streams=1;c.num_out_streams=1;c.properties=properties;c}
pub(super) fn sample(bad_padding:bool,bad_flush:bool,extra_plain:bool)->(Archive,Cursor<Vec<u8>>,Vec<u8>){
 let mut n=1;
 let (body,mut plain)=loop{let body=format!("AES PPMd complete stream {n}\n").repeat(2000).into_bytes();let mut e=ppmd_rust::Ppmd7Encoder::new(vec![],6,1024*1024).unwrap();e.write_all(&body).unwrap();let p=e.finish(false).unwrap();if p.len()%16!=0{break(body,p);}n+=1;};
 if bad_flush{*plain.last_mut().unwrap()^=1;}if extra_plain{plain.push(0);}
 let plain_size=plain.len();let mut cipher=plain;cipher.resize(plain_size.div_ceil(16)*16,0);if bad_padding{*cipher.last_mut().unwrap()=1;}
 let mut seed=KEY.to_vec();seed.extend_from_slice(&[0;8]);let key:[u8;32]=Sha256::digest(seed).into();let mut aes=cbc::Encryptor::<aes::Aes256>::new(&key.into(),&[0u8;16].into());for b in cipher.chunks_exact_mut(16){aes.encrypt_block_mut(GenericArray::from_mut_slice(b));}
 let crc=crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);let body_crc=crc.checksum(&body) as u64;
 let mut properties=vec![6];properties.extend_from_slice(&(1024u32*1024).to_le_bytes());
 let f=Folder{coders:vec![coder(M::ID_PPMD,properties),coder(M::ID_AES256SHA256,vec![0,0])],total_input_streams:2,total_output_streams:2,bind_pairs:vec![BindPair{in_index:0,out_index:1}],packed_streams:vec![1],unpack_sizes:vec![body.len() as u64,plain_size as u64],has_crc:true,crc:body_crc,num_unpack_sub_streams:1};
 let mut e=SevenZArchiveEntry::default();e.name="selected.txt".into();e.has_stream=true;e.size=body.len() as u64;e.has_crc=true;e.crc=body_crc;
 let mut a=Archive::default();a.files=vec![e];a.folders=vec![f];a.pack_sizes=vec![cipher.len() as u64];a.pack_crcs_defined.insert(0);a.pack_crcs=vec![crc.checksum(&cipher) as u64];a.stream_map=StreamMap{folder_first_pack_stream_index:vec![0],pack_stream_offsets:vec![0],folder_first_file_index:vec![0],file_folder_index:vec![Some(0)]};
 let mut data=vec![0;32];data.extend(cipher);(a,Cursor::new(data),body)
}
struct Short(Cursor<Vec<u8>>);
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);self.0.read(&mut b[..n])}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.0.seek(p)}}
#[test]
fn sevenz1628_complete_aes_ppmd_tolerates_short_reads_and_finishes_registered_receipt(){
 let(a,data,body)=sample(false,false,false);let mut data=Short(data);
 let v:Vec<u8>=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_ppmd_entry(0,1024*1024,||Ok(()))).unwrap();assert_eq!(v,body);
}
#[test]
fn sevenz1628_aes_padding_extra_plaintext_alignment_and_ppmd_finish_cannot_be_waived(){
 for(padding,flush,extra)in[(true,false,false),(false,true,false),(false,false,true)]{
  let(a,mut data,_)=sample(padding,flush,extra);let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_ppmd_entry(0,1024*1024,||Ok(())));assert!(v.is_err(),"{padding}/{flush}/{extra}");
 }
 for which in ["alignment","aes-length","power","order"]{let(mut a,mut data,_)=sample(false,false,false);match which{"alignment"=>a.pack_sizes[0]-=1,"aes-length"=>a.folders[0].unpack_sizes[1]+=16,"power"=>a.folders[0].coders[1].properties[0]=21,_=>a.folders[0].packed_streams[0]=0};let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_ppmd_entry(0,1024*1024,||Ok(())));assert!(v.is_err(),"{which}");}
}
