import type { TaskHistoryRecord } from '@/types/taskHistory'

export const failureCategories = {
  'source-changed': '源文件已变化',
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

const stageLabels: Record<string, string> = {
  inspection: '归档检测', unknown: '阶段未知', 'Pre-checking': '预检', Extracting: '解压',
  Verifying: '校验', Finalizing: '完成处理', 'password-attempt': '密码验证',
  Probing: '媒体探测', Encoding: '编码', Transforming: '转换', Validating: '输出校验',
  Publishing: '发布输出', 'still-encoding': '编码',
}

export function describeTaskFailure(record: Pick<TaskHistoryRecord, 'status' | 'errorMessage' | 'failure'>) {
  if (record.status !== 'failed') return null
  const message = record.errorMessage?.trim() || '原记录未保存具体失败原因'
  const marker = message.match(/\[archive-inspection:([a-z-]+)\]/)?.[1]
  const sourceStage = message.match(/\[archive-source:(Pre-checking|Extracting):source-changed\]/)?.[1]
  const category: FailureCategory = marker && Object.hasOwn(failureCategories, marker)
    ? marker as FailureCategory : 'unknown'
  const inspection = Boolean(marker) || message.includes('归档检测失败：')
  const persisted = record.failure?.schemaVersion === 1 ? record.failure : null
  const savedCategory = persisted && Object.hasOwn(failureCategories, persisted.category)
    ? persisted.category as FailureCategory : sourceStage ? 'source-changed' : category
  const stage = persisted && Object.hasOwn(stageLabels, persisted.stage)
    ? persisted.stage : sourceStage || (inspection ? 'inspection' : 'unknown')
  return {
    category: savedCategory,
    categoryLabel: failureCategories[savedCategory],
    stage,
    stageLabel: stageLabels[stage] + (persisted?.evidence === 'observed-stage' ? '（最后观测阶段）' : ''),
    evidence: persisted?.evidence || (marker || sourceStage ? 'recorded-marker' : 'legacy-message'),
    message,
  }
}

export function createTaskDiagnosticReport(record: TaskHistoryRecord, appVersion: string) {
  return JSON.stringify({
    schemaVersion: 1,
    exportedAt: new Date().toISOString(),
    appVersion,
    scope: 'single-persisted-task',
    note: '仅包含当前选中任务已保存的信息；日志可能受历史保留上限截断。应用版本为导出时版本，不代表任务执行时版本。类别和阶段优先使用已保存结构化信息，旧记录按标记解析，未知原因不推测。',
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
