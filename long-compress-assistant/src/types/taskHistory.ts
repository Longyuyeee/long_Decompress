import type { Task } from '@/stores/task'
import { copyExecutionConfig, type ExecutionConfigV1, type ExtractionConfigV1 } from './executionConfig'
export type { ExtractionConfigV1 } from './executionConfig'
import {
  createMeasuredTaskMetricsV1,
  type TaskMetrics,
  type WorkloadKind,
} from '@/types/taskMetrics'

export type TaskHistoryStatus = 'completed' | 'failed' | 'cancelled'

export function restoredExtractionConfig(record: TaskHistoryRecord): ExtractionConfigV1 | null {
  const config = record.executionConfig
  return record.taskType === 'decompression' && (!record.workloadKind || record.workloadKind === 'archive')
    && config?.schemaVersion === 1 && config.kind === 'archive-extraction'
    && typeof config.outputDirectory === 'string' && !!config.outputDirectory.trim()
    && typeof config.extractToSubfolder === 'boolean' ? config : null
}

export interface TaskFailureV1 {
  schemaVersion: 1
  category: string
  stage: string
  evidence: 'recorded-marker' | 'observed-stage' | 'unknown'
}

export interface TaskHistoryLog {
  timestamp: string
  message: string
  severity: string
}

export interface TaskHistoryRecord {
  archiveOutput?: import('./archiveOutput').PublishedArchiveOutput | null
  id: string
  retryOfTaskId?: string | null
  executionConfig?: ExecutionConfigV1 | null
  name: string
  taskType: 'compression' | 'decompression'
  workloadKind: WorkloadKind
  metrics?: TaskMetrics | null
  status: TaskHistoryStatus
  sourcePaths: string[]
  outputPath: string
  format?: string | null
  startedAt?: string | null
  completedAt: string
  durationMs: number
  processedBytes: number
  totalBytes: number
  errorMessage?: string | null
  failure?: TaskFailureV1 | null
  logs: TaskHistoryLog[]
  /** UI-only: a paged summary may defer its logs until opened. */
  logsLoaded?: boolean
}

export const createTaskHistoryRecord = (task: Task): TaskHistoryRecord => {
  const completedAt = task.endTime || new Date()
  const startedAt = task.startTime
  const sourceProgress = task.type === 'compression' ? task.compressionSourceProgress : undefined
  const processedBytes = sourceProgress?.processedBytes ?? task.processedBytes ?? 0
  const totalBytes = sourceProgress?.totalBytes ?? task.totalBytes ?? 0
  const discardedSplitOutput = Boolean(sourceProgress && task.status !== 'completed')
  const measuredMetrics = task.metrics || (
    task.type === 'compression'
    && !discardedSplitOutput
    && task.outputBytes !== undefined
    && !task.outputBytesEstimated
      ? createMeasuredTaskMetricsV1(
          Math.max(0, sourceProgress ? totalBytes : totalBytes || processedBytes),
          task.outputBytes,
        )
      : null
  )
  return {
    archiveOutput: task.status === 'completed' && task.type === 'compression' && task.archiveOutput
      ? { ...task.archiveOutput, files: task.archiveOutput.files.map(file => ({ ...file })) } : null,
    id: task.id,
    retryOfTaskId: task.retryOfTaskId || null,
    executionConfig: copyExecutionConfig(task.executionConfig),
    name: task.name,
    taskType: task.type,
    workloadKind: task.workloadKind || 'archive',
    metrics: measuredMetrics,
    status: task.status as TaskHistoryStatus,
    sourcePaths: [...task.sourceFiles],
    outputPath: task.archiveOutput?.primaryPath ?? task.outputPath,
    format: task.format || task.compressionOptions?.format || null,
    startedAt: startedAt?.toISOString() || null,
    completedAt: completedAt.toISOString(),
    durationMs: startedAt ? Math.max(0, completedAt.getTime() - startedAt.getTime()
      - (task.pausedDurationMs || 0)
      - (task.pausedAt ? Math.max(0, completedAt.getTime() - task.pausedAt.getTime()) : 0)) : 0,
    processedBytes: Math.max(0, processedBytes),
    totalBytes: Math.max(0, totalBytes),
    errorMessage: task.error || null,
    failure: task.status === 'failed' ? {
      schemaVersion: 1,
      category: 'unknown',
      stage: task.stage || 'unknown',
      evidence: task.stage ? 'observed-stage' : 'unknown',
    } : null,
    logs: task.logs.map(log => ({
      timestamp: log.timestamp,
      message: log.message,
      severity: log.severity,
    })),
  }
}
