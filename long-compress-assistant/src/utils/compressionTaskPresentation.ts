import type { Task, TaskStatus } from '@/stores/task'

const FINISHED_STATUSES: TaskStatus[] = ['completed', 'failed', 'cancelled']
const PROGRESS_STATUSES: TaskStatus[] = [
  'running',
  'compressing',
  'preparing',
  'finalizing',
  'cancelling',
]

export const isFinishedCompressionStatus = (status?: TaskStatus) =>
  Boolean(status && FINISHED_STATUSES.includes(status))

export const isActiveCompressionStatus = (status?: TaskStatus) =>
  Boolean(status && !FINISHED_STATUSES.includes(status))

export const compressionStatusTranslationKey = (
  status: TaskStatus = 'pending',
) => `compress.status.${status}`

export const compressionStageTranslationKey = (task?: Task) => {
  if (!task?.stage) {
    return compressionStatusTranslationKey(task?.status || 'pending')
  }

  const normalized = String(task.stage).toLowerCase()
  if (normalized.includes('verif')) return 'compress.status.verifying'
  if (normalized.includes('final')) return 'compress.status.finalizing'
  if (normalized.includes('compress') || normalized.includes('writ')) {
    return 'compress.status.compressing'
  }
  if (normalized.includes('pre') || normalized.includes('check')) {
    return 'compress.status.preparing'
  }
  return compressionStatusTranslationKey(task.status)
}

export const compressionStatusClass = (status?: TaskStatus) => {
  switch (status) {
    case 'completed':
      return 'text-green-500'
    case 'failed':
      return 'text-red-500'
    case 'cancelled':
      return 'text-orange-500'
    case 'cancelling':
      return 'text-orange-400'
    case 'compressing':
    case 'preparing':
    case 'running':
    case 'finalizing':
      return 'text-primary'
    default:
      return 'text-muted'
  }
}

export const compressionStatusIcon = (status?: TaskStatus) => {
  switch (status) {
    case 'completed':
      return 'pi-check-circle'
    case 'failed':
      return 'pi-exclamation-circle'
    case 'cancelled':
      return 'pi-ban'
    case 'cancelling':
    case 'compressing':
    case 'preparing':
    case 'running':
    case 'finalizing':
      return 'pi-spin pi-spinner'
    default:
      return 'pi-clock'
  }
}

export const showsCompressionProgress = (status?: TaskStatus) =>
  Boolean(status && PROGRESS_STATUSES.includes(status))

export const compressionLogSeverityClass = (severity: string) => {
  switch (severity) {
    case 'error':
      return 'text-red-400'
    case 'warning':
      return 'text-yellow-400'
    case 'success':
      return 'text-green-400'
    default:
      return 'text-muted'
  }
}

export const emptyCompressionLogTranslationKey = (task?: Task) =>
  !task || task.status === 'pending'
    ? 'compress.pending_log'
    : 'compress.no_logs'
