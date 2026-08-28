import { useTauriCommands } from '@/composables/useTauriCommands'
import { useTaskStore, type Task, type TaskLog } from '@/stores/task'
import { createMeasuredTaskMetricsV1 } from '@/types/taskMetrics'
import type {
  PublishedVideoOutput,
  VideoCompressionPlan,
  VideoCompressionSettings,
} from '@/types/video'

export interface VideoBatchSource {
  id: string
  name: string
  path: string
  plan: VideoCompressionPlan
  settings: VideoCompressionSettings
}

export interface VideoBatchResult {
  itemId: string
  taskId: string
  status: 'published' | 'failed' | 'cancelled'
  destination: string
  outcome?: PublishedVideoOutput
  error?: string
}

const terminalStatuses = new Set(['completed', 'failed', 'cancelled'])

const appendTaskLog = (task: Task, message: string, severity: TaskLog['severity']) => {
  task.logs.push({ task_id: task.id, timestamp: new Date().toISOString(), message, severity })
}

export const applyPublishedVideoToTask = (
  taskStore: ReturnType<typeof useTaskStore>,
  taskId: string,
  outcome: PublishedVideoOutput,
) => {
  const task = taskStore.tasks.find(candidate => candidate.id === taskId)
  if (!task || terminalStatuses.has(task.status)) return
  task.outputPath = outcome.path
  task.format = outcome.verified.container
  task.processedBytes = outcome.inputBytes
  task.totalBytes = outcome.inputBytes
  task.outputBytes = outcome.outputBytes
  task.outputBytesEstimated = false
  task.outputToInputRatio = outcome.inputBytes > 0 ? outcome.outputBytes / outcome.inputBytes : undefined
  task.metrics = createMeasuredTaskMetricsV1(outcome.inputBytes, outcome.outputBytes, {
    width: outcome.verified.visibleWidth,
    height: outcome.verified.visibleHeight,
    durationMs: outcome.verified.durationMs,
    videoCodec: outcome.verified.videoCodec,
    ...(outcome.verified.audioCodec ? { audioCodec: outcome.verified.audioCodec } : {}),
    container: outcome.verified.container,
  })
  task.progress = 100
  task.heartbeatSecondsSinceProgress = undefined
  task.heartbeatAt = undefined
  appendTaskLog(task, '视频输出已完整验证并原子发布', 'success')
  taskStore.updateTaskStatus(task.id, 'completed')
}

export const useVideoCompressionBatch = () => {
  const taskStore = useTaskStore()
  const commands = useTauriCommands()
  let activeTaskId: string | null = null
  let stopRequested = false

  const runVideoBatch = async (
    sources: VideoBatchSource[],
    outputDirectory: string | null,
    preserveMarkOfWeb: boolean,
    onTaskRegistered?: (itemId: string, taskId: string) => void,
  ): Promise<VideoBatchResult[]> => {
    if (activeTaskId) throw new Error('已有视频任务正在运行')
    stopRequested = false
    const batchId = globalThis.crypto?.randomUUID?.() || `video-batch-${Date.now()}`
    const reservedDestinations: string[] = []
    const results: VideoBatchResult[] = []
    let historyPersistenceFailed = false

    for (const [index, source] of sources.entries()) {
      if (stopRequested) break
      const taskId = `video-${batchId}-${index}`
      taskStore.addTask({
        id: taskId,
        name: source.name,
        type: 'compression',
        workloadKind: 'video',
        sourceFiles: [source.path],
        outputPath: '',
        format: 'mp4',
      })
      taskStore.updateTaskStatus(taskId, 'preparing')
      onTaskRegistered?.(source.id, taskId)
      let destination = ''

      try {
        const destinationPlan = await commands.planVideoCompressionDestination(
          source.path,
          outputDirectory,
          [...reservedDestinations],
        )
        destination = destinationPlan.destination
        reservedDestinations.push(destination)
        const task = taskStore.tasks.find(candidate => candidate.id === taskId)
        if (task) task.outputPath = destination
        if (stopRequested) {
          taskStore.updateTaskStatus(taskId, 'cancelled')
          results.push({ itemId: source.id, taskId, status: 'cancelled', destination })
          continue
        }
        activeTaskId = taskId
        taskStore.updateTaskStatus(taskId, 'compressing')
        const outcome = await commands.compressVideoFile(taskId, {
          plan: { path: source.path, ...source.settings },
          destination,
          confirmedStreamChanges: [...source.plan.streamChanges],
          preserveMarkOfWeb,
        })
        applyPublishedVideoToTask(taskStore, taskId, outcome)
        results.push({ itemId: source.id, taskId, status: 'published', destination, outcome })
      } catch (error) {
        const message = String(error)
        const task = taskStore.tasks.find(candidate => candidate.id === taskId)
        if (task?.status === 'cancelled' || message.includes('CANCELLED')) {
          if (task && task.status !== 'cancelled') taskStore.updateTaskStatus(taskId, 'cancelled')
          results.push({ itemId: source.id, taskId, status: 'cancelled', destination, error: message })
        } else {
          if (task) {
            task.error = message
            appendTaskLog(task, message, 'error')
          }
          taskStore.updateTaskStatus(taskId, 'failed')
          results.push({ itemId: source.id, taskId, status: 'failed', destination, error: message })
        }
      } finally {
        activeTaskId = null
        const persisted = await taskStore.waitForHistoryPersistence(taskId)
        if (!persisted) historyPersistenceFailed = true
      }
    }
    if (historyPersistenceFailed) throw new Error('一个或多个视频任务历史未能持久化')
    return results
  }

  const cancelVideoBatch = async () => {
    stopRequested = true
    if (activeTaskId) await taskStore.cancelTask(activeTaskId)
  }

  return { runVideoBatch, cancelVideoBatch }
}
