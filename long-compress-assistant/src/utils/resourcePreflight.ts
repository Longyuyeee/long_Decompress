import type { Task } from '@/stores/task'
import type { ResourcePreflightReport } from '@/types/resourcePreflight'

export const formatResourceBytes = (value: number | null | undefined): string => {
  if (value === null || value === undefined) return '未知'
  if (value < 1024) return `${value} B`
  const units = ['KiB', 'MiB', 'GiB', 'TiB']
  let scaled = value / 1024
  let unit = units[0]
  for (let index = 1; index < units.length && scaled >= 1024; index++) {
    scaled /= 1024
    unit = units[index]
  }
  return `${scaled >= 10 ? scaled.toFixed(1) : scaled.toFixed(2)} ${unit}`
}

export const resourcePreflightLogMessage = (report: ResourcePreflightReport): string => {
  const target = [report.location, report.medium].filter(value => value !== 'unknown').join('/') || '存储介质未知'
  const available = formatResourceBytes(report.availableBytes)
  const estimated = formatResourceBytes(report.estimatedOutputBytes)
  return `资源预检：${report.summary}（${target}，可用 ${available}，预计输出 ${estimated}）`
}

export const attachResourcePreflight = (task: Task, report: ResourcePreflightReport): void => {
  task.resourcePreflight = report
  task.logs.push({
    task_id: task.id,
    timestamp: new Date().toISOString(),
    message: resourcePreflightLogMessage(report),
    severity: report.status === 'blocked' ? 'error' : report.status === 'warning' ? 'warning' : 'info',
  })
}

export const appendResourcePreflightFallback = (task: Task, error: unknown): void => {
  const message = error instanceof Error ? error.message : String(error)
  task.logs.push({
    task_id: task.id,
    timestamp: new Date().toISOString(),
    message: `资源预检暂不可用，将继续依赖运行时事务保护：${message}`,
    severity: 'warning',
  })
}
