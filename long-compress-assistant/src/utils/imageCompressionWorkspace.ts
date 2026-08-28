import type { ImageFileMetricsV1, ImageMetricFormat } from '@/types/taskMetrics'

export type ImageOutputFormat = 'keep' | 'jpeg' | 'png' | 'webp'
export type ImageCompressionMode = 'lossy' | 'lossless'
export type ImageResizeMode = 'keep' | 'limit'
export type ImageConflictPolicy = 'rename' | 'skip' | 'replace-if-smaller'

export interface ImageCompressionSettings {
  mode: ImageCompressionMode
  quality: number
  resizeMode: ImageResizeMode
  maxWidth: number
  maxHeight: number
  outputFormat: ImageOutputFormat
  preserveMetadata: boolean
  outputDirectory: string
  conflictPolicy: ImageConflictPolicy
}

export interface ImageCandidate {
  name: string
  path: string
  size: number
  type?: string
  isDirectory?: boolean
}

export interface ImageCompressionFacts extends ImageFileMetricsV1 {
  encodedBytes: number
}

export type ImageCompressionOutcome =
  | {
      status: 'published'
      input: ImageCompressionFacts
      output: ImageCompressionFacts
    }
  | {
      status: 'kept-source-because-output-was-not-smaller'
      input: ImageCompressionFacts
      candidate: ImageCompressionFacts
    }

export interface ImageCompressionRequest {
  source: string
  destination: string
  mode: ImageCompressionMode
  quality: number
  targetFormat: ImageMetricFormat
  maxDimensions: { width: number, height: number } | null
  preserveMetadata: boolean
  onlyIfSmaller: boolean
}

export type ImageDestinationPlan =
  | { status: 'ready', destination: string }
  | { status: 'skipped', destination: string, reason: string }

export interface ImageDestinationPlanRequest {
  source: string
  outputDirectory: string | null
  targetFormat: ImageMetricFormat
  conflictPolicy: ImageConflictPolicy
  reservedDestinations: string[]
}

export interface ImageBatchSource {
  id: string
  name: string
  path: string
  inputFormat: string
  settings: ImageCompressionSettings
}

export type ImageBatchItemResult =
  | { status: 'published' | 'kept-source-because-output-was-not-smaller', itemId: string, taskId: string, destination: string, outcome: ImageCompressionOutcome }
  | { status: 'skipped', itemId: string, taskId: string, destination: string, reason: string }
  | { status: 'failed', itemId: string, taskId: string, error: string }
  | { status: 'cancelled', itemId: string, taskId: string }

export interface ImageBatchProgress {
  settled: number
  total: number
  percentage: number
  itemId: string
  taskId: string
  status: ImageBatchItemResult['status']
}

export interface ImageBatchCommands {
  planDestination(request: ImageDestinationPlanRequest): Promise<ImageDestinationPlan>
  compress(taskId: string, request: ImageCompressionRequest): Promise<ImageCompressionOutcome>
  cancel(taskId: string): Promise<void>
}

const supportedExtensions = new Set(['jpg', 'jpeg', 'png', 'webp'])
const explicitlyUnsupportedExtensions = new Map([
  ['gif', '动图 GIF 暂不在首期图片压缩范围内'],
  ['bmp', 'BMP 暂不在首期图片压缩范围内'],
  ['tif', 'TIFF 暂不在首期图片压缩范围内'],
  ['tiff', 'TIFF 暂不在首期图片压缩范围内'],
])

export const imageExtension = (name: string) => {
  const match = name.toLocaleLowerCase().match(/\.([^.]+)$/)
  return match?.[1] || ''
}

export const validateImageCandidate = (candidate: ImageCandidate) => {
  if (candidate.isDirectory) return { accepted: false, reason: '图片模式不接受文件夹，请选择图片文件' }
  if (!candidate.path) return { accepted: false, reason: '无法读取该文件的本地路径' }
  const extension = imageExtension(candidate.name || candidate.path)
  if (supportedExtensions.has(extension)) return { accepted: true, reason: '' }
  return {
    accepted: false,
    reason: explicitlyUnsupportedExtensions.get(extension) || '不是受支持的 JPG、PNG 或 WebP 图片',
  }
}

export const inferImageFormat = (name: string) => {
  const extension = imageExtension(name)
  return extension === 'jpg' ? 'jpeg' : extension || 'unknown'
}

export const createDefaultImageSettings = (): ImageCompressionSettings => ({
  mode: 'lossy',
  quality: 82,
  resizeMode: 'keep',
  maxWidth: 2560,
  maxHeight: 2560,
  outputFormat: 'keep',
  preserveMetadata: true,
  outputDirectory: '',
  conflictPolicy: 'replace-if-smaller',
})

const requireImageMetricFormat = (format: string): ImageMetricFormat => {
  if (format === 'jpeg' || format === 'png' || format === 'webp') return format
  throw new Error(`无法为不受支持的图片格式建立压缩请求：${format}`)
}

export const resolveImageTargetFormat = (inputFormat: string, settings: ImageCompressionSettings) =>
  requireImageMetricFormat(settings.outputFormat === 'keep' ? inputFormat : settings.outputFormat)

export const createImageCompressionRequest = (
  source: string,
  destination: string,
  inputFormat: string,
  settings: ImageCompressionSettings,
): ImageCompressionRequest => ({
  source,
  destination,
  mode: settings.mode,
  quality: settings.quality,
  targetFormat: resolveImageTargetFormat(inputFormat, settings),
  maxDimensions: settings.resizeMode === 'limit'
    ? { width: settings.maxWidth, height: settings.maxHeight }
    : null,
  preserveMetadata: settings.preserveMetadata,
  onlyIfSmaller: settings.conflictPolicy === 'replace-if-smaller',
})

export const createImageTaskId = (batchId: string, itemId: string, index: number) => {
  const nonce = globalThis.crypto?.randomUUID?.() || `${Date.now()}-${index}`
  return `image-${batchId}-${itemId}-${nonce}`
}

export class ImageCompressionBatchRunner {
  private activeTaskId: string | null = null
  private cancellationRequested = false
  private running = false

  constructor(
    private readonly commands: ImageBatchCommands,
    private readonly taskIdFactory: (item: ImageBatchSource, index: number) => string,
  ) {}

  async run(
    items: ImageBatchSource[],
    onProgress?: (progress: ImageBatchProgress) => void,
  ): Promise<ImageBatchItemResult[]> {
    if (this.running) throw new Error('图片批量任务已在运行')
    this.running = true
    const jobs = items.map((item, index) => ({ item, taskId: this.taskIdFactory(item, index) }))
    if (new Set(jobs.map(job => job.taskId)).size !== jobs.length) {
      this.running = false
      throw new Error('图片批量任务 ID 必须逐项唯一')
    }
    const results: ImageBatchItemResult[] = []
    const reservedDestinations: string[] = []
    const report = (result: ImageBatchItemResult) => {
      results.push(result)
      onProgress?.({
        settled: results.length,
        total: jobs.length,
        percentage: jobs.length === 0 ? 100 : Number((results.length / jobs.length * 100).toFixed(2)),
        itemId: result.itemId,
        taskId: result.taskId,
        status: result.status,
      })
    }

    try {
      for (const { item, taskId } of jobs) {
        if (this.cancellationRequested) {
          report({ status: 'cancelled', itemId: item.id, taskId })
          continue
        }
        const targetFormat = resolveImageTargetFormat(item.inputFormat, item.settings)
        let plan: ImageDestinationPlan
        try {
          plan = await this.commands.planDestination({
            source: item.path,
            outputDirectory: item.settings.outputDirectory || null,
            targetFormat,
            conflictPolicy: item.settings.conflictPolicy,
            reservedDestinations: [...reservedDestinations],
          })
        } catch (error) {
          if (this.cancellationRequested) report({ status: 'cancelled', itemId: item.id, taskId })
          else report({ status: 'failed', itemId: item.id, taskId, error: String(error) })
          continue
        }
        if (this.cancellationRequested) {
          report({ status: 'cancelled', itemId: item.id, taskId })
          continue
        }
        if (plan.status === 'skipped') {
          report({ status: 'skipped', itemId: item.id, taskId, destination: plan.destination, reason: plan.reason })
          continue
        }

        reservedDestinations.push(plan.destination)
        this.activeTaskId = taskId
        try {
          const outcome = await this.commands.compress(
            taskId,
            createImageCompressionRequest(item.path, plan.destination, item.inputFormat, item.settings),
          )
          report({ status: outcome.status, itemId: item.id, taskId, destination: plan.destination, outcome })
        } catch (error) {
          if (this.cancellationRequested || String(error).toLocaleLowerCase().includes('cancel')) {
            report({ status: 'cancelled', itemId: item.id, taskId })
          } else {
            report({ status: 'failed', itemId: item.id, taskId, error: String(error) })
          }
        } finally {
          this.activeTaskId = null
        }
      }
      return results
    } finally {
      this.running = false
    }
  }

  async cancel(): Promise<void> {
    this.cancellationRequested = true
    if (this.activeTaskId) await this.commands.cancel(this.activeTaskId)
  }
}

export const estimateImageOutputRange = (inputBytes: number, settings: ImageCompressionSettings) => {
  if (inputBytes <= 0) return null
  if (settings.mode === 'lossless') {
    return { minimum: Math.round(inputBytes * 0.65), maximum: Math.round(inputBytes * 1.02) }
  }
  const quality = Math.min(100, Math.max(1, settings.quality))
  const centre = 0.2 + quality / 160
  return {
    minimum: Math.round(inputBytes * Math.max(0.12, centre - 0.16)),
    maximum: Math.round(inputBytes * Math.min(1.05, centre + 0.18)),
  }
}
