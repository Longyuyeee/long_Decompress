//! User-selected 7Z output method; legacy requests keep LZMA2.
use std::{io::{Cursor,Read},sync::atomic::{AtomicBool,Ordering}};
#[derive(Debug,Clone,Copy,Default,serde::Serialize,serde::Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
pub enum SevenZipMethod { #[default] Lzma2, Lzma, Bzip2, Deflate, Ppmd, Copy }
impl SevenZipMethod {
    pub(crate) fn from_setting(value:&str)->Option<Self>{match value {"lzma2"=>Some(Self::Lzma2),"lzma"=>Some(Self::Lzma),"bzip2"=>Some(Self::Bzip2),"deflate"=>Some(Self::Deflate),"ppmd"=>Some(Self::Ppmd),"copy"=>Some(Self::Copy),_=>None}}
    pub(crate) fn configuration(self,level:u32)->sevenz_rust::SevenZMethodConfiguration {
        use sevenz_rust::{SevenZMethod as M,SevenZMethodConfiguration as C,MethodOptions as O};
        let level=level.clamp(1,9);
        match self {
            Self::Lzma2=>sevenz_rust::lzma::LZMA2Options::with_preset(level).into(),
            Self::Lzma=>C::new(M::LZMA).with_options(O::LZMA2(sevenz_rust::lzma::LZMA2Options::with_preset(level))),
            Self::Bzip2=>C::new(M::BZIP2).with_options(O::Num(level)),
            Self::Deflate=>C::new(M::DEFLATE).with_options(O::Num(level)),
            Self::Copy=>C::new(M::COPY),
            Self::Ppmd=>C::new(M::PPMD).with_options(O::Num(level)),
        }
    }
    pub(crate) fn as_str(self)->&'static str {match self {Self::Lzma2=>"lzma2",Self::Lzma=>"lzma",Self::Bzip2=>"bzip2",Self::Deflate=>"deflate",Self::Ppmd=>"ppmd",Self::Copy=>"copy"}}
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

#[derive(Debug,Clone,Copy,Default,serde::Serialize,serde::Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
pub enum SevenZipFilter { #[default] None, Delta, BcjX86, BcjArm32, BcjArmThumb, BcjArm64 }
pub(crate) fn default_delta_distance()->u32 {1}
impl SevenZipFilter {
    pub(crate) fn as_str(self)->&'static str {match self {Self::None=>"none",Self::Delta=>"delta",Self::BcjX86=>"bcj_x86",Self::BcjArm32=>"bcj_arm32",Self::BcjArmThumb=>"bcj_arm_thumb",Self::BcjArm64=>"bcj_arm64"}}
    pub(crate) fn from_setting(s:&str)->Option<Self>{match s {"none"=>Some(Self::None),"delta"=>Some(Self::Delta),"bcj_x86"=>Some(Self::BcjX86),"bcj_arm32"=>Some(Self::BcjArm32),"bcj_arm_thumb"=>Some(Self::BcjArmThumb),"bcj_arm64"=>Some(Self::BcjArm64),_=>None}}
    pub(crate) fn configuration(self,method:SevenZipMethod,distance:u32)->anyhow::Result<Option<sevenz_rust::SevenZMethodConfiguration>> {
        if self==Self::None || !matches!(method,SevenZipMethod::Lzma2|SevenZipMethod::Lzma){return Ok(None)}
        if self==Self::BcjX86 { return Ok(Some(sevenz_rust::SevenZMethodConfiguration::new(sevenz_rust::SevenZMethod::BCJ_X86_FILTER))); }
        if self==Self::BcjArm32 { return Ok(Some(sevenz_rust::SevenZMethodConfiguration::new(sevenz_rust::SevenZMethod::BCJ_ARM_FILTER))); }
        if self==Self::BcjArmThumb { return Ok(Some(sevenz_rust::SevenZMethodConfiguration::new(sevenz_rust::SevenZMethod::BCJ_ARM_THUMB_FILTER))); }
        if self==Self::BcjArm64 { return Ok(Some(sevenz_rust::SevenZMethodConfiguration::new(sevenz_rust::SevenZMethod::BCJ_ARM64_FILTER))); }
        anyhow::ensure!((1..=256).contains(&distance),"Delta 距离请输入 1～256 的整数");
        Ok(Some(sevenz_rust::SevenZMethodConfiguration::new(sevenz_rust::SevenZMethod::DELTA_FILTER).with_options(sevenz_rust::MethodOptions::Num(distance))))
    }
}
pub(crate) fn sample_size_filtered(sample:&[u8],method:SevenZipMethod,level:u32,filter:SevenZipFilter,distance:u32,cancelled:&AtomicBool)->anyhow::Result<usize>{
    struct Sample<'a>{input:&'a[u8],cancelled:&'a AtomicBool}
    impl Read for Sample<'_>{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{if self.cancelled.load(Ordering::Acquire){return Err(std::io::Error::new(std::io::ErrorKind::Interrupted,"Compression analysis cancelled"));}self.input.read(b)}}
    let mut methods=vec![method.configuration(level)];if let Some(delta)=filter.configuration(method,distance)? {methods.push(delta)}
    let mut writer=sevenz_rust::SevenZWriter::new(Cursor::new(Vec::new()))?;writer.set_content_methods(methods);
    let mut entry=sevenz_rust::SevenZArchiveEntry::default();entry.name="sample".into();writer.push_archive_entry(entry,Some(Sample{input:sample,cancelled}))?;
    Ok(writer.finish()?.into_inner().len())
}
