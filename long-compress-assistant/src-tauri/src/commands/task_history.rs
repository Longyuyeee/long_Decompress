use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashSet;

const MAX_HISTORY_RECORDS: i64 = 500;
const MAX_SOURCE_PATHS: usize = 128;
const MAX_LOGS: usize = 200;
const MAX_TEXT_CHARS: usize = 4_096;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryLog {
    pub timestamp: String,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryRecord {
    pub id: String,
    pub name: String,
    pub task_type: String,
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
    Ok(record)
}

#[tauri::command]
pub async fn save_task_history(record: TaskHistoryRecord) -> Result<(), String> {
    let record = sanitize_record(record)?;
    let pool = crate::database::connection::get_pool()
        .await
        .map_err(|error| format!("任务历史数据库不可用: {error}"))?;
    let source_paths = serde_json::to_string(&record.source_paths)
        .map_err(|error| format!("序列化来源路径失败: {error}"))?;
    let logs = serde_json::to_string(&record.logs)
        .map_err(|error| format!("序列化任务日志失败: {error}"))?;
    let updated_at = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO task_operation_history (
            id, name, task_type, status, source_paths, output_path, format,
            started_at, completed_at, duration_ms, processed_bytes, total_bytes,
            error_message, logs, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            task_type = excluded.task_type,
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
    .execute(&pool)
    .await
    .map_err(|error| format!("保存任务历史失败: {error}"))?;

    sqlx::query(
        "DELETE FROM task_operation_history WHERE id NOT IN (SELECT id FROM task_operation_history ORDER BY completed_at DESC LIMIT ?)",
    )
    .bind(MAX_HISTORY_RECORDS)
    .execute(&pool)
    .await
    .map_err(|error| format!("整理任务历史失败: {error}"))?;
    Ok(())
}

#[tauri::command]
pub async fn list_task_history(limit: Option<i64>) -> Result<Vec<TaskHistoryRecord>, String> {
    let pool = crate::database::connection::get_pool()
        .await
        .map_err(|error| format!("任务历史数据库不可用: {error}"))?;
    let rows = sqlx::query_as::<_, TaskHistoryRow>(
        r#"
        SELECT id, name, task_type, status, source_paths, output_path, format,
               started_at, completed_at, duration_ms, processed_bytes, total_bytes,
               error_message, logs
        FROM task_operation_history
        ORDER BY completed_at DESC
        LIMIT ?
        "#,
    )
    .bind(limit.unwrap_or(MAX_HISTORY_RECORDS).clamp(1, MAX_HISTORY_RECORDS))
    .fetch_all(&pool)
    .await
    .map_err(|error| format!("读取任务历史失败: {error}"))?;

    rows.into_iter()
        .map(|row| {
            Ok(TaskHistoryRecord {
                id: row.id,
                name: row.name,
                task_type: row.task_type,
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
            status: "running".into(), source_paths: vec![], output_path: String::new(),
            format: None, started_at: None, completed_at: Utc::now().to_rfc3339(),
            duration_ms: 0, processed_bytes: 0, total_bytes: 0, error_message: None, logs: vec![],
        };
        assert!(sanitize_record(record).is_err());
    }
}
