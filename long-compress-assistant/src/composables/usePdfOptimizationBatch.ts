import { useTauriCommands } from '@/composables/useTauriCommands'
import { useTaskStore, type Task, type TaskLog } from '@/stores/task'
import { createMeasuredTaskMetricsV1 } from '@/types/taskMetrics'
import type { PdfOptimizationMode, PublishedPdfOutput } from '@/types/pdf'

export interface PdfBatchSource {
  id: string
  name: string
  path: string
  mode: PdfOptimizationMode
  confirmedLossyImageChanges: boolean
  allowLargerOutput: boolean
}

export interface PdfBatchResult {
  itemId: string
  taskId: string
  status: 'published' | 'failed' | 'cancelled'
  destination: string
  outcome?: PublishedPdfOutput
  error?: string
}

const terminalStatuses = new Set(['completed', 'failed', 'cancelled'])

const appendTaskLog = (task: Task, message: string, severity: TaskLog['severity']) => {
  task.logs.push({ task_id: task.id, timestamp: new Date().toISOString(), message, severity })
}

export const applyPublishedPdfToTask = (
  taskStore: ReturnType<typeof useTaskStore>,
  taskId: string,
  outcome: PublishedPdfOutput,
) => {
  const task = taskStore.tasks.find(candidate => candidate.id === taskId)
  if (!task || terminalStatuses.has(task.status)) return
  task.outputPath = outcome.path
  task.format = 'pdf'
  task.processedBytes = outcome.inputBytes
  task.totalBytes = outcome.inputBytes
  task.outputBytes = outcome.outputBytes
  task.outputBytesEstimated = false
  task.outputToInputRatio = outcome.inputBytes > 0 ? outcome.outputBytes / outcome.inputBytes : undefined
  task.metrics = createMeasuredTaskMetricsV1(outcome.inputBytes, outcome.outputBytes, {
    pageCount: outcome.verified.outputFacts.pageCount,
  })
  task.progress = 100
  task.stage = undefined
  appendTaskLog(task, 'PDF 输出已完整验证并原子发布', 'success')
  taskStore.updateTaskStatus(task.id, 'completed')
}

export const usePdfOptimizationBatch = () => {
  const taskStore = useTaskStore()
  const commands = useTauriCommands()
  let activeTaskId: string | null = null
  let stopRequested = false

  const runPdfBatch = async (
    sources: PdfBatchSource[],
    outputDirectory: string | null,
    preserveMarkOfWeb: boolean,
    onTaskRegistered?: (itemId: string, taskId: string) => void,
  ): Promise<PdfBatchResult[]> => {
    if (activeTaskId) throw new Error('已有 PDF 任务正在运行')
    stopRequested = false
    const batchId = globalThis.crypto?.randomUUID?.() || `pdf-batch-${Date.now()}`
    const reservedDestinations: string[] = []
    const results: PdfBatchResult[] = []
    let historyPersistenceFailed = false

    for (const [index, source] of sources.entries()) {
      if (stopRequested) break
      const taskId = `pdf-${batchId}-${index}`
      taskStore.addTask({
        id: taskId,
        name: source.name,
        type: 'compression',
        workloadKind: 'pdf',
        sourceFiles: [source.path],
        outputPath: '',
        format: 'pdf',
      })
      taskStore.updateTaskStatus(taskId, 'preparing')
      onTaskRegistered?.(source.id, taskId)
      let destination = ''

      try {
        const plan = await commands.planPdfOptimizationDestination(
          source.path,
          source.mode,
          outputDirectory,
          [...reservedDestinations],
        )
        destination = plan.destination
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
        const outcome = await commands.compressPdfFile(taskId, {
          source: source.path,
          destination,
          mode: source.mode,
          confirmedLossyImageChanges: source.confirmedLossyImageChanges,
          preserveMarkOfWeb,
          allowLargerOutput: source.allowLargerOutput,
        })
        applyPublishedPdfToTask(taskStore, taskId, outcome)
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
    if (historyPersistenceFailed) throw new Error('一个或多个 PDF 任务历史未能持久化')
    return results
  }

  const cancelPdfBatch = async () => {
    stopRequested = true
    if (activeTaskId) await taskStore.cancelTask(activeTaskId)
  }

  return { runPdfBatch, cancelPdfBatch }
}
