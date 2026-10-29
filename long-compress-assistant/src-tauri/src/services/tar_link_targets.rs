//! Resolve complete archive identities without following OS links or chains.
use anyhow::{Context, Result};
use std::collections::HashMap;

pub(crate) struct Record {
    pub key: String,
    pub content: bool,
    pub target: Option<String>,
}

pub(crate) fn resolve(
    records: &[Record],
    mut check: impl FnMut() -> Result<()>,
) -> Result<Vec<Option<usize>>> {
    let mut names = HashMap::new();
    for (index, entry) in records.iter().enumerate() {
        check()?;
        names
            .entry(entry.key.as_str())
            .and_modify(|value| *value = None)
            .or_insert(Some(index));
    }
    records.iter().enumerate().map(|(index, entry)| {
        check()?;
        let Some(target) = &entry.target else { return Ok(None); };
        let source = names.get(target.as_str()).copied().flatten()
            .context("[archive-inspection:unsupported] TAR.LZO 链接目标缺失或名称有歧义，未确认内容")?;
        anyhow::ensure!(source != index && records[source].content && records[source].target.is_none(),
            "[archive-inspection:unsupported] TAR.LZO 链接目标不是唯一普通内容文件，目录、符号链接和链接链条暂不支持");
        Ok(Some(source))
    }).collect()
}
