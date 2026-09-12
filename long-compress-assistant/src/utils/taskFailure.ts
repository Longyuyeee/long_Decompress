import type { TaskHistoryRecord } from '@/types/taskHistory'

export const failureCategories = {
  damaged: '文件损坏或不完整',
  'missing-volume': '缺失分卷',
  permission: '访问权限',
  timeout: '检测超时',
  'engine-unavailable': '引擎不可用',
  'engine-start': '引擎启动失败',
  io: '检测读取失败',
  'ambiguous-data': '加密数据异常（原因未确定）',
  unknown: '未分类',
} as const

export type FailureCategory = keyof typeof failureCategories

export function describeTaskFailure(record: Pick<TaskHistoryRecord, 'status' | 'errorMessage'>) {
  if (record.status !== 'failed') return null
  const message = record.errorMessage?.trim() || '原记录未保存具体失败原因'
  const marker = message.match(/\[archive-inspection:([a-z-]+)\]/)?.[1]
  const category: FailureCategory = marker && Object.hasOwn(failureCategories, marker)
    ? marker as FailureCategory : 'unknown'
  const inspection = Boolean(marker) || message.includes('归档检测失败：')
  return {
    category,
    categoryLabel: failureCategories[category],
    stage: inspection ? 'inspection' : 'unknown',
    stageLabel: inspection ? '归档检测' : '阶段未知',
    evidence: marker ? 'recorded-marker' : 'legacy-message',
    message,
  }
}

export function createTaskDiagnosticReport(record: TaskHistoryRecord, appVersion: string) {
  return JSON.stringify({
    schemaVersion: 1,
    exportedAt: new Date().toISOString(),
    appVersion,
    scope: 'single-persisted-task',
    note: '仅包含当前选中任务已保存的信息；日志可能受历史保留上限截断。应用版本为导出时版本，不代表任务执行时版本。类别仅按已保存标记解析，未知原因不推测。',
    failure: describeTaskFailure(record),
    task: {
      id: record.id, name: record.name, status: record.status,
      taskType: record.taskType, workloadKind: record.workloadKind || 'archive',
      format: record.format, sourcePaths: record.sourcePaths, outputPath: record.outputPath,
      startedAt: record.startedAt, completedAt: record.completedAt,
      durationMs: record.durationMs, processedBytes: record.processedBytes, totalBytes: record.totalBytes,
      metrics: record.metrics || null,
      errorMessage: record.errorMessage, logs: record.logs,
    },
  }, null, 2)
}
