use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashSet;

const MAX_HISTORY_RECORDS: i64 = 500;
const MAX_SOURCE_PATHS: usize = 128;
const MAX_LOGS: usize = 200;
const MAX_TEXT_CHARS: usize = 4_096;
const MAX_METRIC_LABEL_CHARS: usize = 128;

fn default_workload_kind() -> String {
    "archive".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryLog {
    pub timestamp: String,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageFileMetricsV1 {
    pub format: String,
    pub encoded_width: u64,
    pub encoded_height: u64,
    pub visible_width: u64,
    pub visible_height: u64,
    pub orientation: u8,
    pub frame_count: u64,
    pub has_alpha: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageMediaMetricsV1 {
    pub input: ImageFileMetricsV1,
    pub output: ImageFileMetricsV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaMetricsV1 {
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub frame_count: Option<u64>,
    pub duration_ms: Option<u64>,
    pub page_count: Option<u64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub container: Option<String>,
    pub has_alpha: Option<bool>,
    #[serde(default)]
    pub image: Option<ImageMediaMetricsV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskHistoryMetricsV1 {
    pub schema_version: u32,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub savings_ratio: f64,
    pub media: Option<MediaMetricsV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryRecord {
    pub id: String,
    pub name: String,
    pub task_type: String,
    #[serde(default = "default_workload_kind")]
    pub workload_kind: String,
    #[serde(default)]
    pub metrics: Option<TaskHistoryMetricsV1>,
    pub status: String,
    pub source_paths: Vec<String>,
    pub output_path: String,
    pub format: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: String,
    pub duration_ms: i64,
    pub processed_bytes: i64,
    pub total_bytes: i64,
    pub error_message: Option<String>,
    pub logs: Vec<TaskHistoryLog>,
}

#[derive(Debug, FromRow)]
struct TaskHistoryRow {
    id: String,
    name: String,
    task_type: String,
    workload_kind: Option<String>,
    metrics: Option<String>,
    status: String,
    source_paths: String,
    output_path: String,
    format: Option<String>,
    started_at: Option<String>,
    completed_at: String,
    duration_ms: i64,
    processed_bytes: i64,
    total_bytes: i64,
    error_message: Option<String>,
    logs: String,
}

fn truncate_text(value: &str) -> String {
    value.chars().take(MAX_TEXT_CHARS).collect()
}

fn redact_sensitive_text(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let ascii_markers = ["password:", "password=", "pwd:", "pwd="];
    let localized_markers = ["密码：", "密码:"];
    let marker = ascii_markers
        .iter()
        .filter_map(|marker| lower.find(marker).map(|index| (index, marker.len())))
        .chain(
            localized_markers
                .iter()
                .filter_map(|marker| value.find(marker).map(|index| (index, marker.len()))),
        )
        .min_by_key(|(index, _)| *index);

    if let Some((index, marker_len)) = marker {
        format!("{}{} [已隐藏]", &value[..index], &value[index..index + marker_len])
    } else {
        value.to_string()
    }
}

fn is_high_frequency_log(message: &str) -> bool {
    message.contains("字典攻击进度")
        || (message.contains("候选 [")
            && (message.contains("未匹配") || message.contains("验证异常")))
}

fn evenly_sample(indices: &[usize], count: usize) -> Vec<usize> {
    if indices.len() <= count {
        return indices.to_vec();
    }
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![indices[0]];
    }
    (0..count)
        .map(|position| indices[position * (indices.len() - 1) / (count - 1)])
        .collect()
}

fn retain_auditable_logs(logs: Vec<TaskHistoryLog>) -> Vec<TaskHistoryLog> {
    if logs.len() <= MAX_LOGS {
        return logs;
    }
    let last = logs.len() - 1;
    let priority = logs.iter().enumerate()
        .filter(|(index, log)| {
            *index == 0
                || *index == last
                || !log.severity.eq_ignore_ascii_case("info")
                || !is_high_frequency_log(&log.message)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let selected = if priority.len() >= MAX_LOGS {
        evenly_sample(&priority, MAX_LOGS)
    } else {
        let priority_set = priority.iter().copied().collect::<HashSet<_>>();
        let candidates = (0..logs.len())
            .filter(|index| !priority_set.contains(index))
            .collect::<Vec<_>>();
        let mut selected = priority;
        let remaining = MAX_LOGS - selected.len();
        selected.extend(evenly_sample(&candidates, remaining));
        selected.sort_unstable();
        selected.dedup();
        selected
    };
    let selected = selected.into_iter().collect::<HashSet<_>>();
    logs.into_iter().enumerate()
        .filter_map(|(index, log)| selected.contains(&index).then_some(log))
        .collect()
}

fn sanitize_record(mut record: TaskHistoryRecord) -> Result<TaskHistoryRecord, String> {
    if !matches!(record.task_type.as_str(), "compression" | "decompression") {
        return Err("不支持的任务类型".to_string());
    }
    if !matches!(record.workload_kind.as_str(), "archive" | "image" | "video" | "pdf") {
        return Err("不支持的工作负载类型".to_string());
    }
    if !matches!(record.status.as_str(), "completed" | "failed" | "cancelled") {
        return Err("只能保存已结束任务".to_string());
    }
    if record.id.trim().is_empty() {
        return Err("任务 ID 不能为空".to_string());
    }

    record.name = truncate_text(record.name.trim());
    record.output_path = truncate_text(record.output_path.trim());
    record.format = record.format.map(|value| truncate_text(value.trim()));
    record.error_message = record.error_message.map(|value| {
        truncate_text(redact_sensitive_text(value.trim()).trim())
    });
    record.source_paths = record
        .source_paths
        .into_iter()
        .take(MAX_SOURCE_PATHS)
        .map(|path| truncate_text(path.trim()))
        .filter(|path| !path.is_empty())
        .collect();
    record.logs = retain_auditable_logs(record.logs)
        .into_iter()
        .map(|mut log| {
            log.message = truncate_text(redact_sensitive_text(log.message.trim()).trim());
            log.severity = truncate_text(log.severity.trim());
            log
        })
        .collect();
    record.duration_ms = record.duration_ms.max(0);
    record.processed_bytes = record.processed_bytes.max(0);
    record.total_bytes = record.total_bytes.max(0);
    if let Some(metrics) = record.metrics.as_mut() {
        if metrics.schema_version != 1 {
            return Err("不支持的任务指标版本".to_string());
        }
        if record.workload_kind == "archive" && metrics.media.is_some() {
            return Err("归档任务不能写入媒体指标".to_string());
        }
        metrics.savings_ratio = if metrics.input_bytes > 0 {
            (metrics.input_bytes as f64 - metrics.output_bytes as f64)
                / metrics.input_bytes as f64
        } else {
            0.0
        };
        if let Some(media) = metrics.media.as_mut() {
            if media.image.is_some() && record.workload_kind != "image" {
                return Err("只有图片任务可以写入图片输入/输出事实".to_string());
            }
            for label in [&mut media.video_codec, &mut media.audio_codec, &mut media.container] {
                *label = label
                    .take()
                    .map(|value| value.chars().take(MAX_METRIC_LABEL_CHARS).collect());
            }
            if let Some(image) = media.image.as_mut() {
                sanitize_image_facts(&mut image.input)?;
                sanitize_image_facts(&mut image.output)?;
            }
        }
    }
    Ok(record)
}

fn sanitize_image_facts(facts: &mut ImageFileMetricsV1) -> Result<(), String> {
    facts.format = facts.format.trim().to_ascii_lowercase();
    if !matches!(facts.format.as_str(), "jpeg" | "png" | "webp") {
        return Err("图片指标格式必须是 jpeg、png 或 webp".to_string());
    }
    if facts.encoded_width == 0
        || facts.encoded_height == 0
        || facts.visible_width == 0
        || facts.visible_height == 0
        || facts.frame_count == 0
        || !(1..=8).contains(&facts.orientation)
    {
        return Err("图片指标包含无效的尺寸、方向或帧数".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn save_task_history(record: TaskHistoryRecord) -> Result<(), String> {
    let pool = crate::database::connection::get_pool()
        .await
        .map_err(|error| format!("任务历史数据库不可用: {error}"))?;
    save_task_history_to_pool(&pool, record).await
}

async fn save_task_history_to_pool(
    pool: &sqlx::SqlitePool,
    record: TaskHistoryRecord,
) -> Result<(), String> {
    let record = sanitize_record(record)?;
    let source_paths = serde_json::to_string(&record.source_paths)
        .map_err(|error| format!("序列化来源路径失败: {error}"))?;
    let logs = serde_json::to_string(&record.logs)
        .map_err(|error| format!("序列化任务日志失败: {error}"))?;
    let metrics = record.metrics.as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| format!("序列化任务指标失败: {error}"))?;
    let updated_at = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO task_operation_history (
            id, name, task_type, workload_kind, metrics, status, source_paths, output_path, format,
            started_at, completed_at, duration_ms, processed_bytes, total_bytes,
            error_message, logs, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            task_type = excluded.task_type,
            workload_kind = excluded.workload_kind,
            metrics = excluded.metrics,
            status = excluded.status,
            source_paths = excluded.source_paths,
            output_path = excluded.output_path,
            format = excluded.format,
            started_at = excluded.started_at,
            completed_at = excluded.completed_at,
            duration_ms = excluded.duration_ms,
            processed_bytes = excluded.processed_bytes,
            total_bytes = excluded.total_bytes,
            error_message = excluded.error_message,
            logs = excluded.logs,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&record.id)
    .bind(&record.name)
    .bind(&record.task_type)
    .bind(&record.workload_kind)
    .bind(metrics)
    .bind(&record.status)
    .bind(source_paths)
    .bind(&record.output_path)
    .bind(&record.format)
    .bind(&record.started_at)
    .bind(&record.completed_at)
    .bind(record.duration_ms)
    .bind(record.processed_bytes)
    .bind(record.total_bytes)
    .bind(&record.error_message)
    .bind(logs)
    .bind(updated_at)
    .execute(pool)
    .await
    .map_err(|error| format!("保存任务历史失败: {error}"))?;

    sqlx::query(
        "DELETE FROM task_operation_history WHERE id NOT IN (SELECT id FROM task_operation_history ORDER BY completed_at DESC LIMIT ?)",
    )
    .bind(MAX_HISTORY_RECORDS)
    .execute(pool)
    .await
    .map_err(|error| format!("整理任务历史失败: {error}"))?;
    Ok(())
}

#[tauri::command]
pub async fn list_task_history(limit: Option<i64>) -> Result<Vec<TaskHistoryRecord>, String> {
    let pool = crate::database::connection::get_pool()
        .await
        .map_err(|error| format!("任务历史数据库不可用: {error}"))?;
    list_task_history_from_pool(&pool, limit).await
}

async fn list_task_history_from_pool(
    pool: &sqlx::SqlitePool,
    limit: Option<i64>,
) -> Result<Vec<TaskHistoryRecord>, String> {
    let rows = sqlx::query_as::<_, TaskHistoryRow>(
        r#"
        SELECT id, name, task_type, workload_kind, metrics, status, source_paths, output_path, format,
               started_at, completed_at, duration_ms, processed_bytes, total_bytes,
               error_message, logs
        FROM task_operation_history
        ORDER BY completed_at DESC
        LIMIT ?
        "#,
    )
    .bind(limit.unwrap_or(MAX_HISTORY_RECORDS).clamp(1, MAX_HISTORY_RECORDS))
    .fetch_all(pool)
    .await
    .map_err(|error| format!("读取任务历史失败: {error}"))?;

    rows.into_iter()
        .map(|row| {
            Ok(TaskHistoryRecord {
                id: row.id,
                name: row.name,
                task_type: row.task_type,
                workload_kind: row.workload_kind.unwrap_or_else(default_workload_kind),
                metrics: row.metrics
                    .map(|metrics| serde_json::from_str(&metrics)
                        .map_err(|error| format!("解析任务指标失败: {error}")))
                    .transpose()?,
                status: row.status,
                source_paths: serde_json::from_str(&row.source_paths)
                    .map_err(|error| format!("解析任务来源失败: {error}"))?,
                output_path: row.output_path,
                format: row.format,
                started_at: row.started_at,
                completed_at: row.completed_at,
                duration_ms: row.duration_ms,
                processed_bytes: row.processed_bytes,
                total_bytes: row.total_bytes,
                error_message: row.error_message,
                logs: serde_json::from_str(&row.logs)
                    .map_err(|error| format!("解析任务日志失败: {error}"))?,
            })
        })
        .collect()
}

#[tauri::command]
pub async fn delete_task_history(id: String) -> Result<(), String> {
    let pool = crate::database::connection::get_pool()
        .await
        .map_err(|error| format!("任务历史数据库不可用: {error}"))?;
    sqlx::query("DELETE FROM task_operation_history WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|error| format!("删除任务历史失败: {error}"))?;
    Ok(())
}

#[tauri::command]
pub async fn clear_task_history() -> Result<(), String> {
    let pool = crate::database::connection::get_pool()
        .await
        .map_err(|error| format!("任务历史数据库不可用: {error}"))?;
    sqlx::query("DELETE FROM task_operation_history")
        .execute(&pool)
        .await
        .map_err(|error| format!("清空任务历史失败: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_sensitive_logs_and_limits_payload() {
        let record = TaskHistoryRecord {
            id: "task-1".into(),
            name: "sample.zip".into(),
            task_type: "compression".into(),
            workload_kind: "archive".into(),
            metrics: Some(TaskHistoryMetricsV1 {
                schema_version: 1, input_bytes: 1_200, output_bytes: 600,
                savings_ratio: 99.0, media: None,
            }),
            status: "completed".into(),
            source_paths: vec!["C:/data".into()],
            output_path: "C:/sample.zip".into(),
            format: Some("zip".into()),
            started_at: None,
            completed_at: Utc::now().to_rfc3339(),
            duration_ms: -5,
            processed_bytes: -1,
            total_bytes: 12,
            error_message: None,
            logs: vec![
                TaskHistoryLog { timestamp: "now".into(), message: "开始压缩".into(), severity: "info".into() },
                TaskHistoryLog { timestamp: "now".into(), message: "password: top-secret".into(), severity: "info".into() },
                TaskHistoryLog { timestamp: "now".into(), message: "压缩完成".into(), severity: "success".into() },
            ],
        };

        let sanitized = sanitize_record(record).expect("record should be valid");
        assert_eq!(sanitized.logs.len(), 3);
        assert!(sanitized.logs.iter().all(|log| !log.message.contains("top-secret")));
        assert!(sanitized.logs[1].message.contains("[已隐藏]"));
        assert_eq!(sanitized.duration_ms, 0);
        assert_eq!(sanitized.processed_bytes, 0);
        assert_eq!(sanitized.metrics.unwrap().savings_ratio, 0.5);
    }

    #[test]
    fn preserves_safe_password_workflow_logs() {
        assert_eq!(
            redact_sensitive_text("保险箱候选 [1/2] → 未匹配"),
            "保险箱候选 [1/2] → 未匹配"
        );
        assert_eq!(
            redact_sensitive_text("解压密码：open-sesame"),
            "解压密码： [已隐藏]"
        );
    }

    #[test]
    fn long_dictionary_history_preserves_milestones_and_samples_candidates() {
        let mut logs = vec![TaskHistoryLog {
            timestamp: "0".into(),
            message: "正在检索密码保险箱...".into(),
            severity: "info".into(),
        }];
        for index in 1..=391 {
            logs.push(TaskHistoryLog {
                timestamp: index.to_string(),
                message: format!("内置字典候选 [{index}/391] · 8 字符 → 未匹配"),
                severity: "info".into(),
            });
            if index == 200 {
                logs.push(TaskHistoryLog {
                    timestamp: "milestone".into(),
                    message: "字典尝试阶段检查点".into(),
                    severity: "warning".into(),
                });
            }
        }
        logs.push(TaskHistoryLog {
            timestamp: "end".into(),
            message: "所有候选均未匹配，等待手动输入".into(),
            severity: "warning".into(),
        });

        let retained = retain_auditable_logs(logs);

        assert_eq!(retained.len(), MAX_LOGS);
        assert_eq!(retained.first().unwrap().message, "正在检索密码保险箱...");
        assert!(retained.iter().any(|log| log.message == "字典尝试阶段检查点"));
        assert_eq!(retained.last().unwrap().message, "所有候选均未匹配，等待手动输入");
        assert!(retained.iter().any(|log| log.message.contains("候选 [1/391]")));
        assert!(retained.iter().any(|log| log.message.contains("候选 [391/391]")));
    }

    #[test]
    fn rejects_non_terminal_status() {
        let record = TaskHistoryRecord {
            id: "task-1".into(), name: "task".into(), task_type: "decompression".into(),
            workload_kind: "archive".into(), metrics: None,
            status: "running".into(), source_paths: vec![], output_path: String::new(),
            format: None, started_at: None, completed_at: Utc::now().to_rfc3339(),
            duration_ms: 0, processed_bytes: 0, total_bytes: 0, error_message: None, logs: vec![],
        };
        assert!(sanitize_record(record).is_err());
    }

    #[test]
    fn old_json_defaults_to_archive_without_inventing_metrics() {
        let json = serde_json::json!({
            "id": "legacy-task", "name": "legacy.zip", "taskType": "compression",
            "status": "completed", "sourcePaths": ["C:/source"],
            "outputPath": "C:/legacy.zip", "format": "zip", "startedAt": null,
            "completedAt": Utc::now().to_rfc3339(), "durationMs": 10,
            "processedBytes": 12, "totalBytes": 12, "errorMessage": null, "logs": []
        });
        let record: TaskHistoryRecord = serde_json::from_value(json).unwrap();
        assert_eq!(record.workload_kind, "archive");
        assert!(record.metrics.is_none());
    }

    #[test]
    fn rejects_media_metrics_on_archive_history() {
        let record = TaskHistoryRecord {
            id: "task-media".into(), name: "task".into(), task_type: "compression".into(),
            workload_kind: "archive".into(),
            metrics: Some(TaskHistoryMetricsV1 {
                schema_version: 1, input_bytes: 12, output_bytes: 8, savings_ratio: 0.0,
                media: Some(MediaMetricsV1 {
                    width: Some(1), height: Some(1), frame_count: None, duration_ms: None,
                    page_count: None, video_codec: None, audio_codec: None,
                    container: Some("png".into()), has_alpha: Some(true),
                    image: None,
                }),
            }),
            status: "completed".into(), source_paths: vec![], output_path: String::new(),
            format: None, started_at: None, completed_at: Utc::now().to_rfc3339(),
            duration_ms: 0, processed_bytes: 0, total_bytes: 0, error_message: None, logs: vec![],
        };
        assert!(sanitize_record(record).is_err());
    }

    #[test]
    fn preserves_verified_image_input_and_output_facts() {
        let json = serde_json::json!({
            "id": "image-task", "name": "converted.png", "taskType": "compression",
            "workloadKind": "image", "status": "completed",
            "sourcePaths": ["C:/source.jpg"], "outputPath": "C:/converted.png",
            "format": "png", "startedAt": null, "completedAt": Utc::now().to_rfc3339(),
            "durationMs": 20, "processedBytes": 1200, "totalBytes": 1200,
            "errorMessage": null, "logs": [],
            "metrics": {
                "schemaVersion": 1, "inputBytes": 1200, "outputBytes": 700,
                "savingsRatio": 99,
                "media": {
                    "image": {
                        "input": {
                            "format": "JPEG", "encodedWidth": 640, "encodedHeight": 360,
                            "visibleWidth": 360, "visibleHeight": 640, "orientation": 6,
                            "frameCount": 1, "hasAlpha": false
                        },
                        "output": {
                            "format": "png", "encodedWidth": 360, "encodedHeight": 640,
                            "visibleWidth": 360, "visibleHeight": 640, "orientation": 1,
                            "frameCount": 1, "hasAlpha": false
                        }
                    }
                }
            }
        });
        let record: TaskHistoryRecord = serde_json::from_value(json).unwrap();
        let sanitized = sanitize_record(record).expect("verified image facts should persist");
        let metrics = sanitized.metrics.unwrap();
        assert_eq!(metrics.savings_ratio, 500.0 / 1200.0);
        let image = metrics.media.unwrap().image.unwrap();
        assert_eq!(image.input.format, "jpeg");
        assert_eq!(image.input.orientation, 6);
        assert_eq!(image.output.format, "png");
        assert_eq!(
            (image.output.visible_width, image.output.visible_height),
            (360, 640)
        );
    }

    #[test]
    fn rejects_invalid_or_unknown_image_facts() {
        let invalid: Result<ImageFileMetricsV1, _> = serde_json::from_value(serde_json::json!({
            "format": "jpeg", "encodedWidth": 640, "encodedHeight": 360,
            "visibleWidth": 360, "visibleHeight": 640, "orientation": 0,
            "frameCount": 1, "hasAlpha": false
        }));
        let mut invalid = invalid.unwrap();
        assert!(sanitize_image_facts(&mut invalid).is_err());

        let unknown: Result<ImageFileMetricsV1, _> = serde_json::from_value(serde_json::json!({
            "format": "jpeg", "encodedWidth": 640, "encodedHeight": 360,
            "visibleWidth": 360, "visibleHeight": 640, "orientation": 6,
            "frameCount": 1, "hasAlpha": false, "browserWidth": 360
        }));
        assert!(unknown.is_err());
    }

    #[tokio::test]
    async fn real_image_terminal_history_survives_sqlite_close_and_reopen() {
        use crate::services::image_compression_service::{
            compress_single_image, plan_image_destination, ImageCompressionMode,
            ImageCompressionOutcome, ImageCompressionRequest, ImageConflictPolicy,
            ImageDestinationPlan, ImageFileFormat,
        };
        use std::path::Path;
        use std::sync::atomic::AtomicBool;

        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("test-results")
            .join("media-fixture-audit")
            .join("fixtures")
            .join("images")
            .join("transparent.png");
        if !source.exists() {
            eprintln!("fixture is absent; run npm run test:fixtures:media:images");
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let ImageDestinationPlan::Ready { destination } = plan_image_destination(
            &source,
            Some(temp.path()),
            ImageFileFormat::Png,
            ImageConflictPolicy::Rename,
            &[],
        )
        .unwrap() else {
            panic!("empty output directory must be ready");
        };
        let outcome = compress_single_image(
            &ImageCompressionRequest {
                source: source.clone(),
                destination: destination.clone(),
                mode: ImageCompressionMode::Lossless,
                quality: 82,
                target_format: ImageFileFormat::Png,
                max_dimensions: None,
                preserve_metadata: true,
                only_if_smaller: false,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
        let ImageCompressionOutcome::Published { input, output } = outcome else {
            panic!("real PNG must publish for history verification");
        };
        let image_fact =
            |facts: crate::services::image_compression_service::ImageCompressionFacts| {
                ImageFileMetricsV1 {
                    format: match facts.format {
                        ImageFileFormat::Jpeg => "jpeg",
                        ImageFileFormat::Png => "png",
                        ImageFileFormat::WebP => "webp",
                    }
                    .to_string(),
                    encoded_width: u64::from(facts.encoded_width),
                    encoded_height: u64::from(facts.encoded_height),
                    visible_width: u64::from(facts.visible_width),
                    visible_height: u64::from(facts.visible_height),
                    orientation: facts.orientation,
                    frame_count: u64::from(facts.frame_count),
                    has_alpha: facts.has_alpha,
                }
            };
        let input_bytes = input.encoded_bytes;
        let output_bytes = output.encoded_bytes;
        let savings_ratio = if input_bytes == 0 {
            0.0
        } else {
            ((input_bytes as f64 - output_bytes as f64) / input_bytes as f64).clamp(0.0, 1.0)
        };
        let completed = TaskHistoryRecord {
            id: "real-image-completed".into(),
            name: "transparent.png".into(),
            task_type: "compression".into(),
            workload_kind: "image".into(),
            metrics: Some(TaskHistoryMetricsV1 {
                schema_version: 1,
                input_bytes,
                output_bytes,
                savings_ratio,
                media: Some(MediaMetricsV1 {
                    width: None,
                    height: None,
                    frame_count: None,
                    duration_ms: None,
                    page_count: None,
                    video_codec: None,
                    audio_codec: None,
                    container: None,
                    has_alpha: None,
                    image: Some(ImageMediaMetricsV1 {
                        input: image_fact(input),
                        output: image_fact(output),
                    }),
                }),
            }),
            status: "completed".into(),
            source_paths: vec![source.to_string_lossy().into_owned()],
            output_path: destination.to_string_lossy().into_owned(),
            format: Some("png".into()),
            started_at: Some("2026-08-28T01:00:00Z".into()),
            completed_at: "2026-08-28T01:00:01Z".into(),
            duration_ms: 1_000,
            processed_bytes: input_bytes as i64,
            total_bytes: input_bytes as i64,
            error_message: None,
            logs: vec![],
        };
        let terminal_without_metrics =
            |id: &str, status: &str, error: Option<&str>, second: u8| TaskHistoryRecord {
                id: id.into(),
                name: "photo.webp".into(),
                task_type: "compression".into(),
                workload_kind: "image".into(),
                metrics: None,
                status: status.into(),
                source_paths: vec![source.to_string_lossy().into_owned()],
                output_path: temp
                    .path()
                    .join(format!("{id}.webp"))
                    .to_string_lossy()
                    .into_owned(),
                format: Some("webp".into()),
                started_at: Some("2026-08-28T01:00:00Z".into()),
                completed_at: format!("2026-08-28T01:00:0{second}Z"),
                duration_ms: i64::from(second) * 1_000,
                processed_bytes: 0,
                total_bytes: 0,
                error_message: error.map(str::to_string),
                logs: vec![],
            };

        let database_path = temp.path().join("image-history.db");
        let connection = crate::database::connection::DatabaseConnection::new(&database_path, None)
            .await
            .unwrap();
        save_task_history_to_pool(connection.pool(), completed)
            .await
            .unwrap();
        save_task_history_to_pool(
            connection.pool(),
            terminal_without_metrics("real-image-failed", "failed", Some("真实编码失败"), 2),
        )
        .await
        .unwrap();
        save_task_history_to_pool(
            connection.pool(),
            terminal_without_metrics("real-image-cancelled", "cancelled", None, 3),
        )
        .await
        .unwrap();
        connection.pool().close().await;
        drop(connection);

        let reopened = crate::database::connection::DatabaseConnection::new(&database_path, None)
            .await
            .unwrap();
        let records = list_task_history_from_pool(reopened.pool(), Some(10))
            .await
            .unwrap();
        assert_eq!(
            records
                .iter()
                .map(|record| record.status.as_str())
                .collect::<Vec<_>>(),
            vec!["cancelled", "failed", "completed"]
        );
        let completed = records
            .iter()
            .find(|record| record.id == "real-image-completed")
            .unwrap();
        let metrics = completed.metrics.as_ref().unwrap();
        assert_eq!(
            metrics.input_bytes,
            std::fs::metadata(&source).unwrap().len()
        );
        assert_eq!(
            metrics.output_bytes,
            std::fs::metadata(&destination).unwrap().len()
        );
        assert!((metrics.savings_ratio - savings_ratio).abs() < f64::EPSILON);
        assert_eq!(
            metrics
                .media
                .as_ref()
                .unwrap()
                .image
                .as_ref()
                .unwrap()
                .output
                .format,
            "png"
        );
        assert!(database_path.is_file());
        assert!(std::fs::metadata(database_path).unwrap().len() > 0);
    }
}
