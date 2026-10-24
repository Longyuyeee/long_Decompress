//! User-selected 7Z output method; legacy requests keep LZMA2.
use std::{io::{Cursor,Read},sync::atomic::{AtomicBool,Ordering}};
#[derive(Debug,Clone,Copy,Default,serde::Serialize,serde::Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
pub enum SevenZipMethod { #[default] Lzma2, Lzma, Bzip2 }
impl SevenZipMethod {
    pub(crate) fn from_setting(value:&str)->Option<Self>{match value {"lzma2"=>Some(Self::Lzma2),"lzma"=>Some(Self::Lzma),"bzip2"=>Some(Self::Bzip2),_=>None}}
    pub(crate) fn configuration(self,level:u32)->sevenz_rust::SevenZMethodConfiguration {
        use sevenz_rust::{SevenZMethod as M,SevenZMethodConfiguration as C,MethodOptions as O};
        let level=level.clamp(1,9);
        match self {
            Self::Lzma2=>sevenz_rust::lzma::LZMA2Options::with_preset(level).into(),
            Self::Lzma=>C::new(M::LZMA).with_options(O::LZMA2(sevenz_rust::lzma::LZMA2Options::with_preset(level))),
            Self::Bzip2=>C::new(M::BZIP2).with_options(O::Num(level)),
        }
    }
    pub(crate) fn as_str(self)->&'static str {match self {Self::Lzma2=>"lzma2",Self::Lzma=>"lzma",Self::Bzip2=>"bzip2"}}
}
pub(crate) fn sample_size(sample:&[u8],method:SevenZipMethod,level:u32,cancelled:&AtomicBool)->anyhow::Result<usize>{
    struct Sample<'a>{input:&'a[u8],cancelled:&'a AtomicBool}
    impl Read for Sample<'_>{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{if self.cancelled.load(Ordering::Acquire){return Err(std::io::Error::new(std::io::ErrorKind::Interrupted,"Compression analysis cancelled"));}self.input.read(b)}}
    let mut writer=sevenz_rust::SevenZWriter::new(Cursor::new(Vec::new()))?;
    writer.set_content_methods(vec![method.configuration(level)]);
    let mut entry=sevenz_rust::SevenZArchiveEntry::default();entry.name="sample".into();
    writer.push_archive_entry(entry,Some(Sample{input:sample,cancelled}))?;
    Ok(writer.finish()?.into_inner().len())
}
