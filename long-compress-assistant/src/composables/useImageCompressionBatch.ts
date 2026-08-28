import { useTauriCommands } from '@/composables/useTauriCommands'
import { useTaskStore, type Task, type TaskLog } from '@/stores/task'
import { createVerifiedImageTaskMetricsV1 } from '@/types/taskMetrics'
import {
  ImageCompressionBatchRunner,
  createImageTaskId,
  resolveImageTargetFormat,
  type ImageBatchItemResult,
  type ImageBatchProgress,
  type ImageBatchSource,
} from '@/utils/imageCompressionWorkspace'

const terminalStatuses = new Set(['completed', 'failed', 'cancelled'])

const appendTaskLog = (task: Task, message: string, severity: TaskLog['severity']) => {
  task.logs.push({
    task_id: task.id,
    timestamp: new Date().toISOString(),
    message,
    severity,
  })
}

export const applyImageBatchResultToTask = (
  taskStore: ReturnType<typeof useTaskStore>,
  result: ImageBatchItemResult,
) => {
  const task = taskStore.tasks.find(candidate => candidate.id === result.taskId)
  if (!task || terminalStatuses.has(task.status)) return

  if (result.status === 'published') {
    const outcome = result.outcome
    if (outcome.status !== 'published') throw new Error('图片发布终态与后端事实不一致')
    task.outputPath = result.destination
    task.format = outcome.output.format
    task.processedBytes = outcome.input.encodedBytes
    task.totalBytes = outcome.input.encodedBytes
    task.outputBytes = outcome.output.encodedBytes
    task.outputBytesEstimated = false
    task.metrics = createVerifiedImageTaskMetricsV1(outcome.input, outcome.output)
    task.progress = 100
    appendTaskLog(task, '图片输出已验证并发布', 'success')
    taskStore.updateTaskStatus(task.id, 'completed')
    return
  }

  if (result.status === 'kept-source-because-output-was-not-smaller') {
    const outcome = result.outcome
    if (outcome.status !== 'kept-source-because-output-was-not-smaller') {
      throw new Error('图片保留源文件终态与后端事实不一致')
    }
    task.outputPath = task.sourceFiles[0] || result.destination
    task.format = outcome.input.format
    task.processedBytes = outcome.input.encodedBytes
    task.totalBytes = outcome.input.encodedBytes
    task.outputBytes = outcome.input.encodedBytes
    task.outputBytesEstimated = false
    task.metrics = createVerifiedImageTaskMetricsV1(outcome.input, outcome.input)
    task.progress = 100
    appendTaskLog(
      task,
      `候选输出 ${outcome.candidate.encodedBytes} B 未小于源文件，已保留源文件`,
      'success',
    )
    taskStore.updateTaskStatus(task.id, 'completed')
    return
  }

  if (result.status === 'skipped') {
    task.outputPath = result.destination
    task.progress = 100
    appendTaskLog(task, result.reason, 'success')
    taskStore.updateTaskStatus(task.id, 'completed')
    return
  }

  if (result.status === 'failed') {
    task.error = result.error
    appendTaskLog(task, result.error, 'error')
    taskStore.updateTaskStatus(task.id, 'failed')
    return
  }

  appendTaskLog(task, '图片任务已取消，未发布输出', 'warning')
  taskStore.updateTaskStatus(task.id, 'cancelled')
}

export const useImageCompressionBatch = () => {
  const taskStore = useTaskStore()
  const commands = useTauriCommands()
  let activeRunner: ImageCompressionBatchRunner | null = null

  const runImageBatch = async (
    sources: ImageBatchSource[],
    onProgress?: (progress: ImageBatchProgress) => void,
    requestedBatchId?: string,
  ) => {
    if (activeRunner) throw new Error('已有图片批量任务正在运行')
    const batchId = requestedBatchId || globalThis.crypto?.randomUUID?.() || `batch-${Date.now()}`
    const taskIds = sources.map((source, index) => createImageTaskId(batchId, source.id, index))
    for (const [index, source] of sources.entries()) {
      const taskId = taskIds[index]
      taskStore.addTask({
        id: taskId,
        name: source.name,
        type: 'compression',
        workloadKind: 'image',
        sourceFiles: [source.path],
        outputPath: '',
        format: resolveImageTargetFormat(source.inputFormat, source.settings),
      })
      taskStore.updateTaskStatus(taskId, 'preparing')
    }

    const runner = new ImageCompressionBatchRunner({
      planDestination: commands.planImageCompressionDestination,
      compress: async (taskId, request) => {
        const task = taskStore.tasks.find(candidate => candidate.id === taskId)
        if (task) {
          task.outputPath = request.destination
          task.format = request.targetFormat
        }
        taskStore.updateTaskStatus(taskId, 'compressing')
        return commands.compressImageFile(taskId, request)
      },
      cancel: async taskId => {
        const cancelled = await taskStore.cancelTask(taskId)
        if (!cancelled) throw new Error(`无法取消图片任务：${taskId}`)
      },
    }, (_source, index) => taskIds[index])
    activeRunner = runner

    try {
      const results = await runner.run(sources, progress => {
        applyImageBatchResultToTask(taskStore, progress.result)
        onProgress?.(progress)
      })
      const persisted = await Promise.all(results.map(result => taskStore.waitForHistoryPersistence(result.taskId)))
      if (persisted.some(value => !value)) throw new Error('一个或多个图片任务历史未能持久化')
      return results
    } finally {
      activeRunner = null
    }
  }

  const cancelImageBatch = async () => {
    if (activeRunner) await activeRunner.cancel()
  }

  return { runImageBatch, cancelImageBatch }
}
