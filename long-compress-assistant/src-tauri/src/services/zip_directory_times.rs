//! Only explicit ZIP directory dates in the retained output selection.
use crate::models::compression::DecompressOptions;
use super::{lzop::selected::DirectoryTimes,tar_recovery_time::ModificationTime};
use anyhow::Result;
use std::{io::{Read,Seek},path::Path};

pub(crate) fn for_commit(path:&Path,staging:&Path,options:&DecompressOptions,mut check:impl FnMut()->Result<()>)->Result<DirectoryTimes>{
    if !options.preserve_paths||!options.preserve_timestamps{return Ok(Vec::new());}
    check()?;
    let reader=super::numeric_volume_reader::ZipInputReader::open(path)?;
    collect(&mut zip_aes::ZipArchive::new(reader)?,staging,options,check)
}
fn collect<R:Read+Seek>(archive:&mut zip_aes::ZipArchive<R>,staging:&Path,options:&DecompressOptions,mut check:impl FnMut()->Result<()>)->Result<DirectoryTimes>{
    let mut times=Vec::new();let mut names=std::collections::HashSet::new();
    for i in 0..archive.len(){
        check()?;let entry=archive.by_index_raw(i)?;
        if !entry.is_dir()||!super::extraction_transaction::matches_selected_entries(Path::new(entry.name()),&options.selected_entries){continue;}
        let relative=entry.mangled_name();
        if relative.as_os_str().is_empty()||!staging.join(&relative).is_dir(){continue;}
        let name=relative.to_string_lossy().replace('\\',"/").trim_end_matches('/').to_owned();
        // A duplicate directory identity cannot have one trustworthy source date.
        if !names.insert(name.to_lowercase()){times.retain(|(n,_):&(String,ModificationTime)|n.to_lowercase()!=name.to_lowercase());continue;}
        if let Some(time)=entry.last_modified().and_then(|t|super::native_extraction::zip::system_time(t.year(),t.month(),t.day(),t.hour(),t.minute(),t.second())){
            let time=filetime::FileTime::from_system_time(time);
            times.push((name,ModificationTime{seconds:time.unix_seconds(),nanoseconds:time.nanoseconds()}));
        }
    }
    check()?;Ok(times)
}
