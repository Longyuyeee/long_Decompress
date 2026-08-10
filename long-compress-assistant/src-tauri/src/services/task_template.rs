use crate::models::compression_profile::{
    AutoApplyMode, AutoApplyRule, CompressionConfig, CompressionProfile, PasswordStrategy,
};
use crate::services::compression_format::find_compression_format_capability;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

const TEMPLATE_SCHEMA: &str = "long-decompress-task-template";
const TEMPLATE_VERSION: u32 = 1;
const MAX_TEMPLATE_BYTES: u64 = 256 * 1024;
const MAX_PATTERNS: usize = 32;
const MAX_PATTERN_LENGTH: usize = 128;
const MAX_DRAFT_SOURCES: usize = 1_000;
const MAX_WATCH_PREVIEW_FILES: usize = 1_000;
const MAX_WATCH_PREVIEW_DEPTH: usize = 32;
const WATCH_STABILITY_DELAY_MS: u64 = 750;

pub(crate) fn metadata_is_link_or_reparse_point(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskTemplate {
    pub schema: String,
    pub version: u32,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub source_rules: TemplateSourceRules,
    pub target_rule: TemplateTargetRule,
    pub compression: TemplateCompression,
    pub password_strategy: TemplatePasswordStrategy,
    #[serde(default)]
    pub export_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateSourceRules {
    pub mode: TemplateSourceMode,
    #[serde(default)]
    pub include_patterns: Vec<String>,
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    pub size_range_mib: Option<(u64, u64)>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSourceMode {
    ManualSelection,
    All,
    Pattern,
    SizeRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateTargetRule {
    pub mode: TemplateTargetMode,
    pub filename_template: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateTargetMode {
    SameDirectory,
    ChooseAtRuntime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateCompression {
    pub format: String,
    pub level: u8,
    pub split_archive: bool,
    pub split_size_mib: Option<u32>,
    pub keep_structure: bool,
    pub verify_after: bool,
    pub create_solid_archive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum TemplatePasswordStrategy {
    None,
    PromptAtRuntime,
    FromVault { category_id: Option<String> },
    AutoGenerate { length: u8, save_to_vault: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTemplatePreview {
    pub template: TaskTemplate,
    pub warnings: Vec<String>,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTemplateDraftCandidate {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTemplateDraftExcluded {
    pub candidate: TaskTemplateDraftCandidate,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTemplateDraftPlan {
    pub profile_id: String,
    pub profile_name: String,
    pub accepted: Vec<TaskTemplateDraftCandidate>,
    pub excluded: Vec<TaskTemplateDraftExcluded>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTemplateWatchFolderPreview {
    pub profile_id: String,
    pub profile_name: String,
    pub root_path: String,
    pub scanned_files: usize,
    pub accepted: Vec<TaskTemplateDraftCandidate>,
    pub excluded: Vec<TaskTemplateDraftExcluded>,
    pub truncated: bool,
    pub stability_window_ms: u64,
    pub warnings: Vec<String>,
}

fn source_mode(mode: &AutoApplyMode) -> TemplateSourceMode {
    match mode {
        AutoApplyMode::All => TemplateSourceMode::All,
        AutoApplyMode::Pattern => TemplateSourceMode::Pattern,
        AutoApplyMode::SizeRange => TemplateSourceMode::SizeRange,
        AutoApplyMode::None => TemplateSourceMode::ManualSelection,
    }
}

fn password_strategy(
    profile: &CompressionProfile,
    notes: &mut Vec<String>,
) -> TemplatePasswordStrategy {
    match &profile.password_strategy {
        PasswordStrategy::FromVault { .. } => {
            notes.push("保险箱分类绑定已移除；执行时需要重新选择".to_string());
            TemplatePasswordStrategy::FromVault { category_id: None }
        }
        PasswordStrategy::AutoGenerate {
            length,
            save_to_vault: _,
        } => TemplatePasswordStrategy::AutoGenerate {
            length: *length,
            save_to_vault: false,
        },
        PasswordStrategy::Fixed if profile.config.password.is_some() => {
            notes.push("固定密码已移除；导入后需要运行时输入密码".to_string());
            TemplatePasswordStrategy::PromptAtRuntime
        }
        _ if profile.config.password.is_some() => {
            notes.push("配置中的密码已移除；导入后需要运行时输入密码".to_string());
            TemplatePasswordStrategy::PromptAtRuntime
        }
        _ => TemplatePasswordStrategy::None,
    }
}

pub fn template_from_profile(profile: &CompressionProfile) -> TaskTemplate {
    let mut export_notes = Vec::new();
    if profile.config.delete_after {
        export_notes.push("删除源文件设置未导出；模板导入后始终关闭该选项".to_string());
    }
    let password_strategy = password_strategy(profile, &mut export_notes);
    TaskTemplate {
        schema: TEMPLATE_SCHEMA.to_string(),
        version: TEMPLATE_VERSION,
        name: profile.name.clone(),
        icon: profile.icon.clone(),
        description: profile.description.clone(),
        source_rules: TemplateSourceRules {
            mode: source_mode(&profile.auto_apply.mode),
            include_patterns: profile.auto_apply.file_patterns.clone(),
            exclude_patterns: profile.auto_apply.exclude_patterns.clone(),
            size_range_mib: profile.auto_apply.size_range,
        },
        target_rule: TemplateTargetRule {
            mode: TemplateTargetMode::ChooseAtRuntime,
            filename_template: profile.config.filename_template.clone(),
        },
        compression: TemplateCompression {
            format: profile.config.format.clone(),
            level: profile.config.level,
            split_archive: profile.config.split_archive,
            split_size_mib: profile.config.split_size,
            keep_structure: profile.config.keep_structure,
            verify_after: profile.config.verify_after,
            create_solid_archive: profile.config.create_solid_archive,
        },
        password_strategy,
        export_notes,
    }
}

fn validate_filename_template(value: Option<&str>) -> Result<()> {
    let Some(value) = value else { return Ok(()) };
    if value.len() > 120
        || value
            .chars()
            .any(|character| matches!(character, '/' | '\\' | '\0'))
        || value.contains("..")
    {
        anyhow::bail!("文件名模板包含不安全路径或长度超过 120 个字符");
    }
    let remainder = value
        .replace("{name}", "")
        .replace("{date}", "")
        .replace("{time}", "");
    if remainder.contains(['{', '}']) {
        anyhow::bail!("文件名模板只能使用 {{name}}、{{date}} 和 {{time}} 变量");
    }
    Ok(())
}

fn validate_patterns(patterns: &[String]) -> Result<()> {
    if patterns.len() > MAX_PATTERNS {
        anyhow::bail!("源文件规则最多允许 {MAX_PATTERNS} 个模式");
    }
    for pattern in patterns {
        if pattern.is_empty() || pattern.len() > MAX_PATTERN_LENGTH || pattern.contains('\0') {
            anyhow::bail!("源文件模式为空或长度超过 {MAX_PATTERN_LENGTH} 个字符");
        }
        glob::Pattern::new(pattern).with_context(|| format!("无效的源文件模式: {pattern}"))?;
    }
    Ok(())
}

pub fn validate_profile_source_rules(rules: &AutoApplyRule) -> Result<()> {
    if rules.file_patterns.len() + rules.exclude_patterns.len() > MAX_PATTERNS {
        anyhow::bail!("源文件包含与排除规则合计最多允许 {MAX_PATTERNS} 个模式");
    }
    validate_patterns(&rules.file_patterns)?;
    validate_patterns(&rules.exclude_patterns)?;
    if matches!(rules.mode, AutoApplyMode::Pattern) && rules.file_patterns.is_empty() {
        anyhow::bail!("按模式筛选时必须至少填写一条包含规则");
    }
    if matches!(rules.mode, AutoApplyMode::SizeRange) {
        let Some((minimum, maximum)) = rules.size_range else {
            anyhow::bail!("按大小筛选时必须填写大小范围");
        };
        if minimum > maximum {
            anyhow::bail!("源文件大小范围的最小值不能大于最大值");
        }
    }
    Ok(())
}

fn validate_template(template: &TaskTemplate) -> Result<Vec<String>> {
    if template.schema != TEMPLATE_SCHEMA || template.version != TEMPLATE_VERSION {
        anyhow::bail!("不支持的任务模板架构或版本");
    }
    if template.name.trim().is_empty() || template.name.chars().count() > 50 {
        anyhow::bail!("模板名称不能为空且不能超过 50 个字符");
    }
    if template.icon.chars().count() > 8 || template.description.chars().count() > 160 {
        anyhow::bail!("模板图标或说明超过允许长度");
    }
    let capability = find_compression_format_capability(&template.compression.format)
        .filter(|capability| capability.can_compress)
        .ok_or_else(|| anyhow::anyhow!("模板使用了不支持创建的压缩格式"))?;
    if template.compression.level > 9 {
        anyhow::bail!("压缩等级必须在 0 到 9 之间");
    }
    if template.compression.split_archive {
        if !capability.supports_split {
            anyhow::bail!("所选格式不支持分卷压缩");
        }
        if template.compression.split_size_mib.unwrap_or(0) == 0 {
            anyhow::bail!("启用分卷时必须提供大于 0 MiB 的分卷大小");
        }
    }
    if template.compression.create_solid_archive && capability.format != "7z" {
        anyhow::bail!("只有 7Z 模板可以启用固实压缩");
    }
    validate_filename_template(template.target_rule.filename_template.as_deref())?;
    if template.source_rules.include_patterns.len() + template.source_rules.exclude_patterns.len()
        > MAX_PATTERNS
    {
        anyhow::bail!("源文件包含与排除规则合计最多允许 {MAX_PATTERNS} 个模式");
    }
    validate_patterns(&template.source_rules.include_patterns)?;
    validate_patterns(&template.source_rules.exclude_patterns)?;
    if let Some((minimum, maximum)) = template.source_rules.size_range_mib {
        if minimum > maximum {
            anyhow::bail!("源文件大小范围的最小值不能大于最大值");
        }
    }
    if matches!(template.source_rules.mode, TemplateSourceMode::Pattern)
        && template.source_rules.include_patterns.is_empty()
    {
        anyhow::bail!("按模式匹配的模板必须至少包含一个源文件模式");
    }
    if matches!(template.source_rules.mode, TemplateSourceMode::SizeRange)
        && template.source_rules.size_range_mib.is_none()
    {
        anyhow::bail!("按大小匹配的模板必须提供大小范围");
    }
    if let TemplatePasswordStrategy::AutoGenerate { length, .. } = &template.password_strategy {
        if !(8..=128).contains(length) {
            anyhow::bail!("自动生成密码长度必须在 8 到 128 之间");
        }
    }

    let mut warnings = template.export_notes.clone();
    if !template.compression.verify_after {
        warnings.push("该模板关闭了压缩后完整性校验；建议应用后重新开启".to_string());
    }
    if !matches!(
        template.source_rules.mode,
        TemplateSourceMode::ManualSelection
    ) {
        warnings.push("源文件规则只作为推荐条件导入，自动应用默认保持关闭".to_string());
    }
    if capability.requires_winrar {
        warnings.push("创建 RAR 需要本机安装 WinRAR".to_string());
    }
    if capability.requires_7za {
        warnings.push("该格式需要可用的 7-Zip 引擎".to_string());
    }
    Ok(warnings)
}

fn read_template_file(path: &Path) -> Result<(Vec<u8>, String)> {
    let metadata = std::fs::metadata(path).context("无法读取任务模板文件")?;
    if !metadata.is_file() || metadata.len() > MAX_TEMPLATE_BYTES {
        anyhow::bail!("任务模板必须是小于 256 KiB 的普通文件");
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(MAX_TEMPLATE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_TEMPLATE_BYTES {
        anyhow::bail!("任务模板超过 256 KiB 限制");
    }
    let digest = hex::encode(Sha256::digest(&bytes));
    Ok((bytes, digest))
}

fn parse_template(bytes: &[u8]) -> Result<TaskTemplate> {
    let template: TaskTemplate = serde_json::from_slice(bytes).context("任务模板 JSON 无效")?;
    validate_template(&template)?;
    Ok(template)
}

pub fn export_profile_template(profile: &CompressionProfile, path: &Path) -> Result<TaskTemplate> {
    let template = template_from_profile(profile);
    validate_template(&template)?;
    let json = serde_json::to_vec_pretty(&template)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .context("无法创建任务模板文件")?;
    file.write_all(&json)?;
    file.sync_all()?;
    Ok(template)
}

pub fn preview_template_file(path: &Path) -> Result<TaskTemplatePreview> {
    let (bytes, content_sha256) = read_template_file(path)?;
    let template = parse_template(&bytes)?;
    let warnings = validate_template(&template)?;
    Ok(TaskTemplatePreview {
        template,
        warnings,
        content_sha256,
    })
}

fn auto_apply_mode(mode: TemplateSourceMode) -> AutoApplyMode {
    match mode {
        TemplateSourceMode::All => AutoApplyMode::All,
        TemplateSourceMode::Pattern => AutoApplyMode::Pattern,
        TemplateSourceMode::SizeRange => AutoApplyMode::SizeRange,
        TemplateSourceMode::ManualSelection => AutoApplyMode::None,
    }
}

fn pattern_matches_source(
    pattern: &str,
    candidate: &TaskTemplateDraftCandidate,
    relative_path: Option<&str>,
) -> bool {
    let Ok(compiled) = glob::Pattern::new(pattern) else {
        return false;
    };
    let normalized_path = candidate.path.replace('\\', "/");
    compiled.matches(&candidate.name)
        || compiled.matches(&normalized_path)
        || relative_path.is_some_and(|path| compiled.matches(path))
}

fn source_rule_exclusion(
    profile: &CompressionProfile,
    candidate: &TaskTemplateDraftCandidate,
) -> Option<String> {
    source_rule_exclusion_with_relative_path(profile, candidate, None)
}

fn source_rule_exclusion_with_relative_path(
    profile: &CompressionProfile,
    candidate: &TaskTemplateDraftCandidate,
    relative_path: Option<&str>,
) -> Option<String> {
    let rules = &profile.auto_apply;
    if rules
        .exclude_patterns
        .iter()
        .any(|pattern| pattern_matches_source(pattern, candidate, relative_path))
    {
        return Some("命中排除规则".to_string());
    }

    let requires_file_metadata = !rules.exclude_patterns.is_empty()
        || matches!(
            rules.mode,
            AutoApplyMode::Pattern | AutoApplyMode::SizeRange
        );
    if candidate.is_directory && requires_file_metadata {
        return Some("规则型模板首阶段不展开目录，请显式选择文件".to_string());
    }

    match rules.mode {
        AutoApplyMode::None | AutoApplyMode::All => None,
        AutoApplyMode::Pattern => {
            if rules
                .file_patterns
                .iter()
                .any(|pattern| pattern_matches_source(pattern, candidate, relative_path))
            {
                None
            } else {
                Some("未命中包含规则".to_string())
            }
        }
        AutoApplyMode::SizeRange => {
            let Some((minimum, maximum)) = rules.size_range else {
                return Some("模板缺少文件大小范围".to_string());
            };
            let size_mib = candidate.size / (1024 * 1024);
            if size_mib < minimum || size_mib > maximum {
                Some(format!("不在 {minimum}–{maximum} MiB 范围内"))
            } else {
                None
            }
        }
    }
}

/// Scan a user-selected folder once and preview which stable files match a profile.
/// This function never persists a watcher, creates a task, or mutates profile/task storage.
pub fn preview_profile_watch_folder(
    profile: &CompressionProfile,
    root: &Path,
) -> Result<TaskTemplateWatchFolderPreview> {
    preview_profile_watch_folder_with_delay(
        profile,
        root,
        Duration::from_millis(WATCH_STABILITY_DELAY_MS),
    )
}

fn preview_profile_watch_folder_with_delay(
    profile: &CompressionProfile,
    root: &Path,
    stability_delay: Duration,
) -> Result<TaskTemplateWatchFolderPreview> {
    validate_profile_source_rules(&profile.auto_apply)?;
    let root_metadata = std::fs::symlink_metadata(root)
        .with_context(|| format!("无法读取预览文件夹：{}", root.display()))?;
    if metadata_is_link_or_reparse_point(&root_metadata) {
        anyhow::bail!("预览根目录不能是符号链接或目录联接");
    }
    if !root_metadata.is_dir() {
        anyhow::bail!("只读预览需要选择文件夹");
    }

    let mut scanned_files = 0usize;
    let mut excluded = Vec::new();
    let mut pending_stability = Vec::new();
    let mut truncated = false;
    let mut walk_errors = 0usize;
    let mut skipped_links = 0usize;
    let mut depth_limit_reached = false;

    for entry in walkdir::WalkDir::new(root)
        .min_depth(1)
        .max_depth(MAX_WATCH_PREVIEW_DEPTH)
        .follow_links(false)
        .sort_by_file_name()
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                walk_errors += 1;
                continue;
            }
        };
        let path = entry.path();
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => {
                walk_errors += 1;
                continue;
            }
        };
        if metadata_is_link_or_reparse_point(&metadata) {
            skipped_links += 1;
            continue;
        }
        if metadata.is_dir() && entry.depth() == MAX_WATCH_PREVIEW_DEPTH {
            depth_limit_reached = true;
        }
        if !metadata.is_file() {
            continue;
        }
        if scanned_files >= MAX_WATCH_PREVIEW_FILES {
            truncated = true;
            break;
        }
        scanned_files += 1;
        let candidate = TaskTemplateDraftCandidate {
            path: path.to_string_lossy().into_owned(),
            name: entry.file_name().to_string_lossy().into_owned(),
            size: metadata.len(),
            is_directory: false,
        };
        let relative_path = path
            .strip_prefix(root)
            .ok()
            .map(|value| value.to_string_lossy().replace('\\', "/"));

        if let Some(reason) =
            source_rule_exclusion_with_relative_path(profile, &candidate, relative_path.as_deref())
        {
            excluded.push(TaskTemplateDraftExcluded { candidate, reason });
            continue;
        }

        match metadata.modified() {
            Ok(modified) => pending_stability.push((candidate, modified)),
            Err(_) => excluded.push(TaskTemplateDraftExcluded {
                candidate,
                reason: "无法读取文件修改时间，不能判断稳定性".to_string(),
            }),
        }
    }

    if !pending_stability.is_empty() && !stability_delay.is_zero() {
        std::thread::sleep(stability_delay);
    }

    let mut accepted = Vec::new();
    for (candidate, initial_modified) in pending_stability {
        let current_metadata = std::fs::symlink_metadata(&candidate.path);
        let is_stable = current_metadata.as_ref().is_ok_and(|metadata| {
            metadata.is_file()
                && !metadata_is_link_or_reparse_point(metadata)
                && metadata.len() == candidate.size
                && metadata
                    .modified()
                    .is_ok_and(|modified| modified == initial_modified)
        });
        if is_stable {
            accepted.push(candidate);
        } else {
            excluded.push(TaskTemplateDraftExcluded {
                candidate,
                reason: "文件在稳定观察窗口内发生变化、消失或无法读取".to_string(),
            });
        }
    }

    let stability_window_ms = stability_delay.as_millis().min(u64::MAX as u128) as u64;
    let mut warnings = vec![
        "本结果仅为一次性只读预览，不会保存监控、创建草稿或启动压缩".to_string(),
        format!(
            "稳定性仅依据间隔 {stability_window_ms} 毫秒的两次文件元数据快照，不代表后台持续监控"
        ),
    ];
    if matches!(profile.auto_apply.mode, AutoApplyMode::None) {
        warnings.push("当前配置组为手动选择模式，本次仅展示全部稳定普通文件".to_string());
    }
    if truncated {
        warnings.push(format!(
            "文件数量超过上限，仅审计前 {MAX_WATCH_PREVIEW_FILES} 个普通文件"
        ));
    }
    if walk_errors > 0 {
        warnings.push(format!("有 {walk_errors} 个路径无法读取，未纳入结果"));
    }
    if skipped_links > 0 {
        warnings.push(format!(
            "已跳过 {skipped_links} 个符号链接或目录联接，未跟随到外部路径"
        ));
    }
    if depth_limit_reached {
        warnings.push(format!(
            "目录层级超过 {MAX_WATCH_PREVIEW_DEPTH} 层，更深内容未纳入结果"
        ));
    }
    if profile.config.delete_after {
        warnings.push("预览不会采用删除源文件设置".to_string());
    }
    if profile.config.password.is_some()
        || !matches!(&profile.password_strategy, PasswordStrategy::None)
    {
        warnings.push("预览不会读取、填入或生成密码".to_string());
    }

    accepted.sort_by(|left, right| left.path.to_lowercase().cmp(&right.path.to_lowercase()));
    excluded.sort_by(|left, right| {
        left.candidate
            .path
            .to_lowercase()
            .cmp(&right.candidate.path.to_lowercase())
    });

    Ok(TaskTemplateWatchFolderPreview {
        profile_id: profile.id.clone(),
        profile_name: profile.name.clone(),
        root_path: root.to_string_lossy().into_owned(),
        scanned_files,
        accepted,
        excluded,
        truncated,
        stability_window_ms,
        warnings,
    })
}

/// Build a read-only source plan from a profile. This never mutates profile or task storage.
pub fn plan_profile_draft(
    profile: &CompressionProfile,
    file_paths: &[String],
) -> Result<TaskTemplateDraftPlan> {
    if file_paths.is_empty() {
        anyhow::bail!("至少选择一个源文件");
    }
    if file_paths.len() > MAX_DRAFT_SOURCES {
        anyhow::bail!("单次任务模板草稿最多选择 {MAX_DRAFT_SOURCES} 个源项");
    }
    validate_profile_source_rules(&profile.auto_apply)?;

    let mut accepted = Vec::new();
    let mut excluded = Vec::new();
    let mut seen = HashSet::new();
    for file_path in file_paths {
        let path = Path::new(file_path);
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(file_path)
            .to_string();
        let normalized = file_path.replace('/', "\\").to_lowercase();
        if !seen.insert(normalized) {
            let metadata = std::fs::metadata(path).ok();
            excluded.push(TaskTemplateDraftExcluded {
                candidate: TaskTemplateDraftCandidate {
                    path: file_path.clone(),
                    name,
                    size: metadata.as_ref().map(|value| value.len()).unwrap_or(0),
                    is_directory: metadata.as_ref().is_some_and(|value| value.is_dir()),
                },
                reason: "重复源项已忽略".to_string(),
            });
            continue;
        }
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => {
                excluded.push(TaskTemplateDraftExcluded {
                    candidate: TaskTemplateDraftCandidate {
                        path: file_path.clone(),
                        name,
                        size: 0,
                        is_directory: false,
                    },
                    reason: "源项不存在或无法读取".to_string(),
                });
                continue;
            }
        };
        let candidate = TaskTemplateDraftCandidate {
            path: file_path.clone(),
            name,
            size: metadata.len(),
            is_directory: metadata.is_dir(),
        };
        if !metadata.is_file() && !metadata.is_dir() {
            excluded.push(TaskTemplateDraftExcluded {
                candidate,
                reason: "不是普通文件或目录".to_string(),
            });
            continue;
        }
        if let Some(reason) = source_rule_exclusion(profile, &candidate) {
            excluded.push(TaskTemplateDraftExcluded { candidate, reason });
        } else {
            accepted.push(candidate);
        }
    }

    let mut warnings = vec!["该计划只会创建压缩草稿，不会启动任务".to_string()];
    if !excluded.is_empty() {
        warnings.push(format!("有 {} 个源项未通过模板规则", excluded.len()));
    }
    if profile.config.delete_after {
        warnings.push("创建草稿时将强制关闭删除源文件".to_string());
    }
    if profile.config.password.is_some()
        || !matches!(&profile.password_strategy, PasswordStrategy::None)
    {
        warnings.push("创建草稿时不会自动填入或生成密码".to_string());
    }
    Ok(TaskTemplateDraftPlan {
        profile_id: profile.id.clone(),
        profile_name: profile.name.clone(),
        accepted,
        excluded,
        warnings,
    })
}

pub fn import_template_profile(path: &Path, expected_sha256: &str) -> Result<CompressionProfile> {
    let (bytes, actual_sha256) = read_template_file(path)?;
    if actual_sha256 != expected_sha256 {
        anyhow::bail!("任务模板在预览后发生变化，请重新预览再导入");
    }
    let template = parse_template(&bytes)?;
    let mut profile = CompressionProfile::new(
        template.name.trim().to_string(),
        template.icon,
        template.description.trim().to_string(),
        CompressionConfig {
            format: template.compression.format,
            level: template.compression.level,
            password: None,
            split_archive: template.compression.split_archive,
            split_size: template.compression.split_size_mib,
            keep_structure: template.compression.keep_structure,
            delete_after: false,
            verify_after: template.compression.verify_after,
            create_solid_archive: template.compression.create_solid_archive,
            filename_template: template.target_rule.filename_template,
            extra_params: Default::default(),
        },
    );
    profile.auto_apply = AutoApplyRule {
        enabled: false,
        mode: auto_apply_mode(template.source_rules.mode),
        file_patterns: template.source_rules.include_patterns,
        exclude_patterns: template.source_rules.exclude_patterns,
        size_range: template.source_rules.size_range_mib,
    };
    profile.password_strategy = match template.password_strategy {
        TemplatePasswordStrategy::FromVault { .. } => PasswordStrategy::None,
        TemplatePasswordStrategy::AutoGenerate {
            length,
            save_to_vault: _,
        } => PasswordStrategy::AutoGenerate {
            length,
            save_to_vault: false,
        },
        TemplatePasswordStrategy::None | TemplatePasswordStrategy::PromptAtRuntime => {
            PasswordStrategy::None
        }
    };
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::compression_profile::{AutoApplyRule, ProfileStats};

    fn profile() -> CompressionProfile {
        CompressionProfile {
            id: "profile-1".to_string(),
            name: "安全备份".to_string(),
            icon: "📦".to_string(),
            description: "portable".to_string(),
            config: CompressionConfig {
                format: "7z".to_string(),
                level: 7,
                password: Some("must-never-export".to_string()),
                split_archive: false,
                split_size: None,
                keep_structure: true,
                delete_after: true,
                verify_after: true,
                create_solid_archive: true,
                filename_template: Some("{name}-{date}".to_string()),
                extra_params: [("unsafe".to_string(), "ignored".to_string())].into(),
            },
            auto_apply: AutoApplyRule {
                enabled: true,
                mode: AutoApplyMode::Pattern,
                file_patterns: vec!["*.txt".to_string()],
                exclude_patterns: vec!["*.tmp".to_string()],
                size_range: None,
            },
            password_strategy: PasswordStrategy::Fixed,
            stats: ProfileStats::default(),
            created_at: 0,
            last_used_at: None,
        }
    }

    #[test]
    fn export_never_contains_password_delete_source_or_extra_engine_parameters() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("safe.longtask.json");
        export_profile_template(&profile(), &path).unwrap();
        let json = std::fs::read_to_string(&path).unwrap();
        assert!(!json.contains("must-never-export"));
        assert!(!json.contains("deleteAfter"));
        assert!(!json.contains("unsafe"));
        assert!(json.contains("prompt_at_runtime"));
        assert!(json.contains("固定密码已移除"));
    }

    #[test]
    fn preview_is_bounded_strict_and_reports_normalized_risks() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("template.json");
        export_profile_template(&profile(), &path).unwrap();
        let preview = preview_template_file(&path).unwrap();
        assert_eq!(preview.content_sha256.len(), 64);
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("自动应用")));

        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        value["unexpected"] = serde_json::json!(true);
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(preview_template_file(&path)
            .unwrap_err()
            .to_string()
            .contains("JSON"));
    }

    #[test]
    fn import_is_bound_to_preview_hash_and_forces_safe_runtime_defaults() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("template.json");
        export_profile_template(&profile(), &path).unwrap();
        let preview = preview_template_file(&path).unwrap();
        let imported = import_template_profile(&path, &preview.content_sha256).unwrap();
        assert!(imported.config.password.is_none());
        assert!(!imported.config.delete_after);
        assert!(!imported.auto_apply.enabled);
        assert!(matches!(imported.auto_apply.mode, AutoApplyMode::Pattern));
        assert_eq!(imported.auto_apply.exclude_patterns, vec!["*.tmp"]);

        std::fs::write(&path, b"{}").unwrap();
        let error = import_template_profile(&path, &preview.content_sha256).unwrap_err();
        assert!(error.to_string().contains("发生变化"));
    }

    #[test]
    fn draft_plan_uses_real_metadata_and_exclusion_rules_without_starting_work() {
        let temp = tempfile::tempdir().unwrap();
        let accepted_path = temp.path().join("keep.txt");
        let excluded_path = temp.path().join("skip.tmp");
        let directory_path = temp.path().join("nested");
        std::fs::write(&accepted_path, b"keep").unwrap();
        std::fs::write(&excluded_path, b"skip").unwrap();
        std::fs::create_dir(&directory_path).unwrap();
        let missing_path = temp.path().join("missing.txt");

        let plan = plan_profile_draft(
            &profile(),
            &[
                accepted_path.to_string_lossy().into_owned(),
                accepted_path.to_string_lossy().into_owned(),
                excluded_path.to_string_lossy().into_owned(),
                directory_path.to_string_lossy().into_owned(),
                missing_path.to_string_lossy().into_owned(),
            ],
        )
        .unwrap();

        assert_eq!(plan.accepted.len(), 1);
        assert_eq!(plan.accepted[0].name, "keep.txt");
        assert_eq!(plan.excluded.len(), 4);
        assert!(plan
            .excluded
            .iter()
            .any(|item| item.reason.contains("重复源项")));
        assert!(plan
            .excluded
            .iter()
            .any(|item| item.reason.contains("排除规则")));
        assert!(plan
            .excluded
            .iter()
            .any(|item| item.reason.contains("不展开目录")));
        assert!(plan
            .excluded
            .iter()
            .any(|item| item.reason.contains("无法读取")));
        assert!(plan
            .warnings
            .iter()
            .any(|warning| warning.contains("不会启动任务")));
    }

    #[test]
    fn watch_folder_preview_uses_relative_rules_and_rejects_changing_files() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        let accepted_path = nested.join("keep.log");
        let changing_path = nested.join("changing.log");
        let excluded_path = nested.join("skip.tmp");
        let unmatched_path = temp.path().join("root.log");
        std::fs::write(&accepted_path, b"stable").unwrap();
        std::fs::write(&changing_path, b"first").unwrap();
        std::fs::write(&excluded_path, b"excluded").unwrap();
        std::fs::write(&unmatched_path, b"unmatched").unwrap();

        let mut source = profile();
        source.auto_apply.file_patterns = vec!["nested/*.log".to_string()];
        let changing_path_for_thread = changing_path.clone();
        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            std::fs::write(changing_path_for_thread, b"changed-and-longer").unwrap();
        });

        let preview = preview_profile_watch_folder_with_delay(
            &source,
            temp.path(),
            Duration::from_millis(150),
        )
        .unwrap();
        writer.join().unwrap();

        assert_eq!(preview.scanned_files, 4);
        assert_eq!(preview.accepted.len(), 1);
        assert_eq!(preview.accepted[0].name, "keep.log");
        assert_eq!(preview.excluded.len(), 3);
        assert!(preview
            .excluded
            .iter()
            .any(|item| item.candidate.name == "skip.tmp" && item.reason.contains("排除规则")));
        assert!(preview.excluded.iter().any(|item| {
            item.candidate.name == "root.log" && item.reason.contains("未命中包含规则")
        }));
        assert!(preview.excluded.iter().any(|item| {
            item.candidate.name == "changing.log" && item.reason.contains("发生变化")
        }));
        assert!(!preview.truncated);
        assert_eq!(preview.stability_window_ms, 150);
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("不会保存监控、创建草稿或启动压缩")));
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("不会读取、填入或生成密码")));
    }

    #[test]
    fn watch_folder_preview_rejects_non_directory_roots() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("not-a-folder.txt");
        std::fs::write(&file_path, b"content").unwrap();
        let error = preview_profile_watch_folder_with_delay(&profile(), &file_path, Duration::ZERO)
            .unwrap_err();
        assert!(error.to_string().contains("需要选择文件夹"));
    }

    #[test]
    fn vault_bindings_and_vault_writes_are_never_portable() {
        let mut source = profile();
        source.config.password = None;
        source.password_strategy = PasswordStrategy::FromVault {
            category_id: Some("private-category".to_string()),
        };
        let template = template_from_profile(&source);
        assert!(matches!(
            template.password_strategy,
            TemplatePasswordStrategy::FromVault { category_id: None }
        ));
        assert!(template
            .export_notes
            .iter()
            .any(|note| note.contains("分类绑定已移除")));

        source.password_strategy = PasswordStrategy::AutoGenerate {
            length: 24,
            save_to_vault: true,
        };
        let template = template_from_profile(&source);
        assert!(matches!(
            template.password_strategy,
            TemplatePasswordStrategy::AutoGenerate {
                length: 24,
                save_to_vault: false
            }
        ));

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("generated.longtask.json");
        std::fs::write(&path, serde_json::to_vec(&template).unwrap()).unwrap();
        let preview = preview_template_file(&path).unwrap();
        let imported = import_template_profile(&path, &preview.content_sha256).unwrap();
        assert!(matches!(
            imported.password_strategy,
            PasswordStrategy::AutoGenerate {
                length: 24,
                save_to_vault: false
            }
        ));
    }

    #[test]
    fn rejects_unsafe_filename_templates_and_unsupported_format_combinations() {
        let mut template = template_from_profile(&profile());
        template.target_rule.filename_template = Some("../{name}".to_string());
        assert!(validate_template(&template).is_err());

        template.target_rule.filename_template = Some("{name}".to_string());
        template.compression.format = "tar.gz".to_string();
        template.compression.split_archive = true;
        template.compression.split_size_mib = Some(10);
        assert!(validate_template(&template).is_err());
    }
}
