import type { TaskHistoryRecord } from '@/types/taskHistory'

export const failureCategories = {
  'rollback-incomplete': '回滚不完整（需人工核对）',
  'output-conflict': '输出目标冲突（未覆盖）',
  'publication-failed': '输出提交失败',
  'verification-failed': '压缩产物校验未通过',
  'source-changed': '源文件已变化',
  'source-missing': '源文件不存在',
  'source-unavailable': '源文件无法检查',
  'source-invalid': '源路径不是普通文件',
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
  const sourceMarker = message.match(/\[archive-source:(Pre-checking|Extracting):(source-changed|source-missing|source-unavailable|source-invalid)\]/)
  const sourceStage = sourceMarker?.[1]
  const verification = message.includes('[archive-output:Verifying:verification-failed]')
  const publication = message.match(/\[archive-output:Publishing:(output-conflict|publication-failed|rollback-incomplete)\]/)?.[1] as FailureCategory | undefined
  const category: FailureCategory = marker && Object.hasOwn(failureCategories, marker)
    ? marker as FailureCategory : 'unknown'
  const inspection = Boolean(marker) || message.includes('归档检测失败：')
  const persisted = record.failure?.schemaVersion === 1 ? record.failure : null
  const savedCategory = persisted && Object.hasOwn(failureCategories, persisted.category)
    ? persisted.category as FailureCategory : publication || (verification ? 'verification-failed' : sourceMarker ? sourceMarker[2] as FailureCategory : category)
  const stage = persisted && Object.hasOwn(stageLabels, persisted.stage)
    ? persisted.stage : publication ? 'Publishing' : verification ? 'Verifying' : sourceStage || (inspection ? 'inspection' : 'unknown')
  return {
    category: savedCategory,
    categoryLabel: failureCategories[savedCategory],
    stage,
    stageLabel: stageLabels[stage] + (persisted?.evidence === 'observed-stage' ? '（最后观测阶段）' : ''),
    evidence: persisted?.evidence || (marker || sourceStage || verification || publication ? 'recorded-marker' : 'legacy-message'),
    message,
  }
}

export function getTaskRecoveryGuidance(record: Pick<TaskHistoryRecord, 'status' | 'errorMessage' | 'failure'>) {
  if (describeTaskFailure(record)?.category !== 'rollback-incomplete') return null
  return {
    title: '回滚不完整：先保留文件，再核对',
    steps: [
      '保留源压缩包、当前输出目录和错误详情中的恢复目录；恢复目录可能是隐藏目录，不要删除其中的备份。',
      '导出本任务诊断报告，并按下方保存的来源、输出路径和原始错误核对；旧记录缺少恢复路径时不要猜测目录。',
      '检查输出是否混有旧文件和新文件，必要时先复制到另一个位置再人工处理；本软件尚不能自动判定每份残留文件应恢复到哪里。',
      '从历史创建的解压草稿是新任务，不会继续提交旧暂存或恢复旧输出。如需重新解压，请选择新的空目录，不要覆盖尚未核对的输出。',
    ],
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
    recoveryGuidance: getTaskRecoveryGuidance(record),
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
