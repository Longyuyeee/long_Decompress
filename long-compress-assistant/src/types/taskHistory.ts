import type { Task } from '@/stores/task'
import {
  createMeasuredTaskMetricsV1,
  type TaskMetrics,
  type WorkloadKind,
} from '@/types/taskMetrics'

export type TaskHistoryStatus = 'completed' | 'failed' | 'cancelled'

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
  id: string
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
}

export const createTaskHistoryRecord = (task: Task): TaskHistoryRecord => {
  const completedAt = task.endTime || new Date()
  const startedAt = task.startTime
  const measuredMetrics = task.metrics || (
    task.type === 'compression'
    && task.outputBytes !== undefined
    && !task.outputBytesEstimated
      ? createMeasuredTaskMetricsV1(
          Math.max(0, task.totalBytes || task.processedBytes || 0),
          task.outputBytes,
        )
      : null
  )
  return {
    id: task.id,
    name: task.name,
    taskType: task.type,
    workloadKind: task.workloadKind || 'archive',
    metrics: measuredMetrics,
    status: task.status as TaskHistoryStatus,
    sourcePaths: [...task.sourceFiles],
    outputPath: task.outputPath,
    format: task.format || task.compressionOptions?.format || null,
    startedAt: startedAt?.toISOString() || null,
    completedAt: completedAt.toISOString(),
    durationMs: startedAt ? Math.max(0, completedAt.getTime() - startedAt.getTime()) : 0,
    processedBytes: Math.max(0, task.processedBytes || 0),
    totalBytes: Math.max(0, task.totalBytes || 0),
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
