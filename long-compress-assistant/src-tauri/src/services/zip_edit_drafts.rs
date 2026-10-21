//! Application-owned ZIP edit references; credentials never belong in a draft.
use anyhow::{ensure,Result};
use std::{path::Path,io::Write};
static ACCESS:std::sync::Mutex<()>=std::sync::Mutex::new(());
const NAME:&str="zip-edit-drafts-v1.json";
fn validate(data:&str)->Result<()>{
    ensure!(data.len()<=64*1024*1024,"编辑草稿过大，请分批整理后另存");
    let value:serde_json::Value=serde_json::from_str(data)?;
    ensure!(value["version"]==1&&value["sessions"].is_array(),"编辑草稿格式无效");
    fn clean(value:&serde_json::Value)->bool{match value{serde_json::Value::Object(map)=>map.iter().all(|(key,value)|!["password","sourcePassword","outputPassword","credentials"].contains(&key.as_str())&&(key=="encryptions"||clean(value))),serde_json::Value::Array(rows)=>rows.iter().all(clean),_=>true}}
    ensure!(clean(&value),"编辑草稿不能包含密码");Ok(())
}
pub fn load(directory:&Path)->Result<Option<String>>{
    let _guard=ACCESS.lock().unwrap_or_else(|error|error.into_inner());let path=directory.join(NAME);
    if !path.exists(){return Ok(None)}
    let data=std::fs::read_to_string(path)?;validate(&data)?;Ok(Some(data))
}
pub fn save(directory:&Path,data:Option<&str>)->Result<()>{
    let _guard=ACCESS.lock().unwrap_or_else(|error|error.into_inner());let path=directory.join(NAME);
    if let Some(data)=data{validate(data)?;std::fs::create_dir_all(directory)?;let mut file=tempfile::NamedTempFile::new_in(directory)?;file.write_all(data.as_bytes())?;file.as_file().sync_all()?;file.persist(path)?;}
    else if path.exists(){std::fs::remove_file(path)?;}Ok(())
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn zip_edit1546_drafts_replace_atomically_without_credentials(){let dir=tempfile::tempdir().unwrap();let first=r#"{"version":1,"sessions":[{"sourcePath":"source.zip","outputMode":"keep"}]}"#;save(dir.path(),Some(first)).unwrap();assert_eq!(load(dir.path()).unwrap().as_deref(),Some(first));assert!(save(dir.path(),Some(r#"{"version":1,"sessions":[{"sourcePassword":"secret"}]}"#)).is_err());assert_eq!(load(dir.path()).unwrap().as_deref(),Some(first));let second=r#"{"version":1,"sessions":[]}"#;save(dir.path(),Some(second)).unwrap();assert_eq!(load(dir.path()).unwrap().as_deref(),Some(second));save(dir.path(),None).unwrap();assert!(load(dir.path()).unwrap().is_none());}
}
