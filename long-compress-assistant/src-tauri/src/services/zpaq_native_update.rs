//! A standalone native transaction group: old bytes plus one real transaction.
use super::*;
use crate::services::zpaq_multipart::{self as native, Group, Member};

pub(crate) fn output_paths(output:&Path,count:usize)->Result<Vec<PathBuf>> {
    let stem=output.file_stem().and_then(|s|s.to_str()).context("新组名称无效")?;
    anyhow::ensure!(!stem.contains(['*','?'])&&(1..=100000).contains(&count),"新组名称或部分数量无效");
    Ok((1..=count).map(|i|output.with_file_name(format!("{stem}.part{i}.zpaq"))).collect())
}
fn available(output:&Path)->Result<()> {
    let first=output_paths(output,1)?.remove(0);
    let prefix=first.file_stem().unwrap().to_string_lossy().trim_end_matches('1').to_lowercase();
    anyhow::ensure!(!output.exists()&&super::super::archive_volumes::volume_paths(output)?.is_empty(),"新备份名称已存在，请另选名称");
    for entry in fs::read_dir(output.parent().context("请选择新组保存目录")?)? {
        let entry=entry?;let name=entry.file_name().to_string_lossy().to_lowercase();
        if let Some(digits)=name.strip_prefix(&prefix).and_then(|s|s.strip_suffix(".zpaq")) {
            anyhow::ensure!(digits.is_empty()||!digits.bytes().all(|b|b.is_ascii_digit()),"新组已有部分：{}；请另选组名",entry.path().display());
        }
    }Ok(())
}
fn reference(root:&Path,parts:&[PathBuf])->Result<PathBuf> {
    let members=parts.iter().map(|path|{let metadata=fs::metadata(path)?;Ok(Member{path:path.clone(),size:metadata.len(),stamp:format!("{:?}",metadata.modified()?)})}).collect::<Result<Vec<_>>>()?;
    native::confirm(root,Group{name:"另存更新.zpaq".into(),members,missing:vec![]})
}
fn publish_parts(staged:&[PathBuf],output:&Path,flag:&Arc<AtomicBool>)->Result<()> {
    available(output)?;
    let targets=output_paths(output,staged.len())?;
    let mut published=Vec::new();
    let result=(||->Result<()> {
        for (source,target) in staged.iter().zip(&targets) {
            check(flag)?;
            publish::transfer_no_replace(source,target)?;
            published.push(target);
        }
        check(flag)?;Ok(())
    })();
    // Roll back only files moved by this invocation. Pre-existing results stay intact.
    if result.is_err() {for path in published {let _=fs::remove_file(path);}}
    result
}
pub(super) async fn update(source:&Path,group:&Group,entries:&[CompressionEntry],output:&Path,password:Option<&str>,level:u32,
    base_snapshots:usize,copied_bytes:u64,input_bytes:u64,flag:Arc<AtomicBool>,progress:Progress)->Result<UpdateSummary> {
    available(output)?;
    anyhow::ensure!(group.members.len()<100000,"原组部分数量过多，请先转换成单包再更新");
    native::inspect(source)?;
    let directory=tempfile::Builder::new().prefix(".zpaq-native-update-").tempdir_in(output.parent().unwrap())?;
    let base=directory.path().join("backup.zpaq");
    let staged=output_paths(&base,group.members.len())?;
    let (members,paths,copy_flag,copy_progress)=(group.members.clone(),staged.clone(),flag.clone(),progress.clone());
    tokio::task::spawn_blocking(move||->Result<()> {
        let mut buffer=[0u8;65536];let mut copied=0;let mut gate=super::super::progress_gate::ProgressGate::new();
        notify(&copy_progress,"copying",0,copied_bytes);
        for (member,target) in members.iter().zip(paths) {
            check(&copy_flag)?;let mut reader=fs::File::open(&member.path)?;
            let mut writer=fs::OpenOptions::new().write(true).create_new(true).open(target)?;let mut part_bytes=0;
            loop {check(&copy_flag)?;let n=reader.read(&mut buffer)?;if n==0 {break}writer.write_all(&buffer[..n])?;part_bytes+=n as u64;copied+=n as u64;
                if gate.allow(copied==copied_bytes){notify(&copy_progress,"copying",copied,copied_bytes);}}
            anyhow::ensure!(part_bytes==member.size,"复制期间原备份发生变化，请重新确认整组");writer.sync_all()?;
        }
        anyhow::ensure!(copied==copied_bytes,"原备份大小变化，请重新确认整组");Ok(())
    }).await??;
    native::inspect(source)?;check(&flag)?;notify(&progress,"updating",0,input_bytes);
    // Upstream's native writer derives salt from part 1 and continues AES at
    // the summed byte offset. It never encrypts later parts as separate archives.
    let result=append(&base.with_file_name("backup.part*.zpaq"),entries,password,level,flag.clone(),progress.clone()).await?;
    let count=group.members.len()+usize::from(result.updated_entries>0);
    let staged=output_paths(&base,count)?;
    let reference=reference(directory.path(),&staged)?;
    notify(&progress,"verifying",0,0);
    zpaq_engine::verify_at(&reference,password,None,flag.clone()).await?;
    let new=zpaq_engine::browse(&reference,password,flag.clone()).await?;
    let snapshots=new.zpaq_history.context("新组未保留历史快照")?.snapshots.len();
    anyhow::ensure!(snapshots==base_snapshots+usize::from(result.updated_entries>0),"新组快照数量不一致，未发布结果");
    native::inspect(source)?;check(&flag)?;notify(&progress,"publishing",0,0);
    publish_parts(&staged,output,&flag)?;
    notify(&progress,"completed",input_bytes,input_bytes);
    Ok(UpdateSummary{native_parts:Some(count),updated_entries:result.updated_entries,new_fragments:result.new_fragments,base_snapshots,snapshots,
        unchanged:result.updated_entries==0,copied_bytes,input_bytes})
}
