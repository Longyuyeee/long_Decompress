//! Only new Bzip2 selected-reader behavior; original-tool coverage is in app tests.
use crate::{folder::{Coder,BindPair},Archive,BlockDecoder,Error,SevenZMethod as M};
use aes::cipher::{BlockEncryptMut,KeyIvInit,generic_array::GenericArray};
use sha2::{Digest,Sha256};use std::io::{Cursor,Read,Seek,SeekFrom,Write};
const KEY:&[u8]=&[b'k',0];
fn sample(shared:bool,encrypted:bool,mutation:&str)->(Archive,Cursor<Vec<u8>>,Vec<u8>){
 let(mut a,_,body)=crate::deflate64_preview_1638_tests::sample(true,false);
 let mut encoder=bzip2::write::BzEncoder::new(Vec::new(),bzip2::Compression::new(9));encoder.write_all(&body).unwrap();let mut packed=encoder.finish().unwrap();
 match mutation {"footer"=>{packed.truncate(packed.len()-5);},"extra"=>packed.push(0),"second"=>{let again=packed.clone();packed.extend(again);},"bad-magic"=>packed[0]^=1,"internal-crc"=>packed[10]^=1,_=>()}
 let c=&mut a.folders[0].coders[0];c.id_size=M::ID_BZIP2.len();c.decompression_method_id_mut().copy_from_slice(M::ID_BZIP2);
 let crc=crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
 if shared {let mut offset=0;let e=a.files[0].clone();a.files.clear();for(i,n)in[7,0,40000,19993].into_iter().enumerate(){let mut e=e.clone();e.name=format!("{i}.txt");e.size=n as u64;e.crc=crc.checksum(&body[offset..offset+n]) as u64;a.files.push(e);offset+=n;}a.folders[0].num_unpack_sub_streams=4;a.stream_map.file_folder_index=vec![Some(0);4];}
 if encrypted {
  let plain=packed.len();packed.resize(plain.div_ceil(16)*16,0);if mutation=="padding"{assert!(plain<packed.len());*packed.last_mut().unwrap()=1;}
  let mut seed=KEY.to_vec();seed.extend([0;8]);let key:[u8;32]=Sha256::digest(seed).into();let mut aes=cbc::Encryptor::<aes::Aes256>::new(&key.into(),&[0u8;16].into());for b in packed.chunks_exact_mut(16){aes.encrypt_block_mut(GenericArray::from_mut_slice(b));}
  let mut c=Coder::default();c.id_size=M::ID_AES256SHA256.len();c.decompression_method_id_mut().copy_from_slice(M::ID_AES256SHA256);c.num_in_streams=1;c.num_out_streams=1;c.properties=vec![0,0];
  let f=&mut a.folders[0];f.coders.push(c);f.total_input_streams=2;f.total_output_streams=2;f.bind_pairs=vec![BindPair{in_index:0,out_index:1}];f.packed_streams=vec![1];f.unpack_sizes.push(plain as u64);
 }
 a.pack_sizes[0]=packed.len() as u64;a.pack_crcs[0]=crc.checksum(&packed) as u64;let mut bytes=vec![0;32];bytes.extend(packed);(a,Cursor::new(bytes),body)
}
struct Short(Cursor<Vec<u8>>);
impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(3);self.0.read(&mut b[..n])}}
impl Seek for Short{fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.0.seek(p)}}
#[test]
fn sevenz1644_single_shared_plain_aes_short_reads_retain_first_empty_middle_last(){
 for encrypted in [false,true]{for shared in [false,true]{for(i,start,n)in if shared{vec![(0,0,7),(1,7,0),(2,7,40000),(3,40007,19993)]}else{vec![(0,0,60000)]}{let(a,data,body)=sample(shared,encrypted,"");let mut data=Short(data);let v=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,if encrypted{KEY}else{&[]},&mut data).read_bzip2_entry(i,1024*1024,64*1024*1024,||Ok(()))).unwrap();assert_eq!(v,&body[start..start+n]);}}}
}
#[test]
fn sevenz1644_readable_body_missing_footer_extra_stream_and_internal_crc_never_complete(){
 // Body can already be readable before the final stream CRC/end is available.
 let(_,data,body)=sample(false,false,"footer");let mut codec=bzip2::Decompress::new(false);let mut out=vec![0;body.len()+1];let status=codec.decompress(&data.get_ref()[32..],&mut out).unwrap();assert_ne!(status,bzip2::Status::StreamEnd);assert_eq!(codec.total_out(),body.len() as u64);assert_eq!(&out[..body.len()],body);
 for encrypted in [false,true]{for mutation in ["footer","extra","second","bad-magic","internal-crc"]{let(a,mut data,_)=sample(true,encrypted,mutation);assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,if encrypted{KEY}else{&[]},&mut data).read_bzip2_entry(2,1024*1024,64*1024*1024,||Ok(()))).is_err(),"{encrypted} {mutation}");}}
}
#[test]
fn sevenz1644_shared_crc_mapping_sizes_methods_and_mid_read_cancel_remain_required(){
 for encrypted in [false,true]{for which in ["before-crc","after-crc","folder-crc","pack-crc","missing-crc","count","sum","mapping","properties","pack-map","input-limit","shared-limit","selected-limit"]{let(mut a,mut data,_)=sample(true,encrypted,"");match which{"before-crc"=>a.files[0].crc^=1,"after-crc"=>a.files[3].crc^=1,"folder-crc"=>a.folders[0].crc^=1,"pack-crc"=>a.pack_crcs[0]^=1,"missing-crc"=>{a.files[0].has_crc=false;a.folders[0].has_crc=false;},"count"=>a.folders[0].num_unpack_sub_streams-=1,"sum"=>a.files[2].size-=1,"mapping"=>a.stream_map.file_folder_index[2]=None,"properties"=>a.folders[0].coders[0].properties.push(1),"pack-map"=>a.folders[0].packed_streams[0]=99,"input-limit"=>a.pack_sizes[0]=64*1024*1024+1,_=>()};let(max_bytes,max_shared)=match which{"shared-limit"=>(1024*1024,59999),"selected-limit"=>(39999,64*1024*1024),_=>(1024*1024,64*1024*1024)};assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,if encrypted{KEY}else{&[]},&mut data).read_bzip2_entry(2,max_bytes,max_shared,||Ok(()))).is_err(),"{encrypted} {which}");}}
 let(a,mut data,_)=sample(true,true,"");let mut calls=0;let e=BlockDecoder::new(0,&a,KEY,&mut data).read_bzip2_entry(3,1024*1024,64*1024*1024,||{calls+=1;if calls==6{Err(Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));assert_eq!(calls,6);
}
#[test]
fn sevenz1644_bzip2_terminal_cannot_waive_aes_padding_parameters_or_other_full_receipts(){
 let(a,mut data,_)=sample(true,true,"padding");let e=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_bzip2_entry(0,1024*1024,64*1024*1024,||Ok(()))).unwrap_err();assert!(e.to_string().contains("padding"),"{e}");
 let(a,mut data,_)=sample(true,true,"");let r=crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||{let _other=crate::aes256sha256::PreviewAes::new(Cursor::new(data.get_ref()[32..].to_vec()),&[0,0],KEY,a.folders[0].unpack_sizes[1] as usize)?;BlockDecoder::new(0,&a,KEY,&mut data).read_bzip2_entry(2,1024*1024,64*1024*1024,||Ok(()))});assert!(r.unwrap_err().to_string().contains("AES 数据未完整消费"));
 for which in ["alignment","size","power","properties","chain"]{let(mut a,mut data,_)=sample(true,true,"");match which{"alignment"=>a.pack_sizes[0]-=1,"size"=>a.folders[0].unpack_sizes[1]+=16,"power"=>a.folders[0].coders[1].properties[0]=21,"properties"=>{a.folders[0].coders[1].properties.pop();},_=>a.folders[0].packed_streams[0]=0};assert!(crate::with_preview_limits::<_,Error>(64*1024*1024,20,||Ok(()),||BlockDecoder::new(0,&a,KEY,&mut data).read_bzip2_entry(2,1024*1024,64*1024*1024,||Ok(()))).is_err(),"{which}");}
}
