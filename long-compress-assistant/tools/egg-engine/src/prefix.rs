//! Partial, bounded non-solid text only. Full verification keeps the old path.
use super::*;
#[derive(Debug)]
struct PrefixComplete;
impl std::fmt::Display for PrefixComplete {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("EGG preview prefix complete") }
}
impl std::error::Error for PrefixComplete {}
struct PrefixWriter { bytes:Vec<u8>, stop:bool }
impl Write for PrefixWriter {
    fn write(&mut self,data:&[u8])->io::Result<usize> {
        let n=data.len().min(MAX_TEXT as usize-self.bytes.len());
        self.bytes.extend_from_slice(&data[..n]);
        if self.stop && self.bytes.len()==MAX_TEXT as usize {
            return Err(io::Error::other(PrefixComplete));
        }
        Ok(n)
    }
    fn flush(&mut self)->io::Result<()> { Ok(()) }
}
pub fn preview(a:&mut EggArchive<egg_volumes::Reader>,chosen:&[EggFileEntry],req:&Request)->Result<Value> {
    ensure!(!a.is_solid && chosen.len()==1 && !chosen[0].is_directory(),"请选择一份非固实 EGG 文本");
    let e=&chosen[0];
    ensure!(e.uncompressed_size>MAX_TEXT,"小文本应使用完整预览");
    ensure!(e.blocks.iter().all(|b|matches!(b.compression_method,CompressionMethod::Store|CompressionMethod::Deflate|CompressionMethod::Bzip2|CompressionMethod::Lzma)),
        "EGG 大文本前缀目前支持存储、Deflate、Bzip2 和 LZMA，请先提取后打开");
    plan(a,chosen)?;
    let mut input=PreviewInput{inner:&mut a.reader,left:MAX_TEXT_INPUT,limited:false};
    let setup=complex::cipher(&mut input,e,req.password.as_deref(),req.encoding.as_deref());
    ensure!(!input.limited,"EGG 文本前缀读取超过 64 MiB 范围，请先提取后打开");
    let mut crypto=setup?;
    let mut output=PrefixWriter{bytes:Vec::with_capacity(MAX_TEXT as usize),stop:false};
    for b in &e.blocks {
        output.stop=b.uncompressed_size as u64>MAX_TEXT-output.bytes.len() as u64;
        input.seek(SeekFrom::Start(b.data_pos))?;
        let mut part=(&mut input).take(b.compressed_size as u64);
        let mut checked=CheckedWriter{inner:&mut output,bytes:0,limit:b.uncompressed_size as u64};
        let cipher=crypto.as_mut().map(|c|&mut **c as &mut dyn Decryptor);
        let result=complex::stream(&mut part,&mut checked,b.compression_method,b.compressed_size as u64,b.uncompressed_size as u64,cipher);
        let written=checked.bytes;
        ensure!(!input.limited,"EGG 文本前缀读取超过 64 MiB 范围，请先提取后打开");
        match result {
            Err(EggError::Io(error)) if error.get_ref().is_some_and(|e|e.is::<PrefixComplete>())=>{
                ensure!(output.bytes.len()==MAX_TEXT as usize,"EGG 文本前缀未完成");
                return Ok(json!({"bytes":output.bytes,"totalSize":e.uncompressed_size,"inputBytes":MAX_TEXT_INPUT-input.left,"partial":true}));
            }
            Err(error)=>return Err(map_error(error,e.encrypt_info.is_some())),
            Ok(crc)=>ensure!(written==b.uncompressed_size as u64 && crc==b.crc32,
                "{} EGG 文本前缀之前的数据块大小或 CRC 校验失败",if e.encrypt_info.is_some(){"[archive-inspection:ambiguous-data]"}else{"[archive-inspection:damaged]"}),
        }
        // Blocks wholly within the prefix still require full size, CRC and end checks.
        if output.bytes.len()==MAX_TEXT as usize {
            return Ok(json!({"bytes":output.bytes,"totalSize":e.uncompressed_size,"inputBytes":MAX_TEXT_INPUT-input.left,"partial":true}));
        }
    }
    anyhow::bail!("[archive-inspection:damaged] EGG 文本在预览范围前已结束且与声明大小不符，请先检查或重新获取来源")
}
