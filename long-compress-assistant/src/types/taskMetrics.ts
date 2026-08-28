export type WorkloadKind = 'archive' | 'image' | 'video' | 'pdf'

export type ImageMetricFormat = 'jpeg' | 'png' | 'webp'

export interface ImageFileMetricsV1 {
  format: ImageMetricFormat
  /** Encoded pixel matrix before applying EXIF orientation. */
  encodedWidth: number
  encodedHeight: number
  /** Orientation-applied dimensions shown to the user. */
  visibleWidth: number
  visibleHeight: number
  orientation: number
  frameCount: number
  hasAlpha: boolean
}

export interface ImageMediaMetricsV1 {
  input: ImageFileMetricsV1
  output: ImageFileMetricsV1
}

export interface MediaMetricsV1 {
  /** Orientation-applied dimensions shown to the user, not the encoded pixel matrix. */
  width?: number
  height?: number
  frameCount?: number
  durationMs?: number
  pageCount?: number
  videoCodec?: string
  audioCodec?: string
  container?: string
  hasAlpha?: boolean
  /** Verified input/output image facts. Optional for pre-B04 history compatibility. */
  image?: ImageMediaMetricsV1
}

/**
 * Versioned, measured task metrics shared by the live queue and persisted history.
 * Values must come from the processing engine or the published output on disk.
 */
export interface TaskMetricsV1 {
  schemaVersion: 1
  inputBytes: number
  outputBytes: number
  savingsRatio: number
  media?: MediaMetricsV1
}

export type TaskMetrics = TaskMetricsV1

export const createImageMediaMetricsV1 = (
  input: ImageFileMetricsV1,
  output: ImageFileMetricsV1,
): MediaMetricsV1 => ({
  image: {
    input: { ...input },
    output: { ...output },
  },
})

const normalizeMeasuredBytes = (value: number): number => {
  if (!Number.isFinite(value) || value <= 0) return 0
  return Math.min(Number.MAX_SAFE_INTEGER, Math.round(value))
}

export const createMeasuredTaskMetricsV1 = (
  inputBytes: number,
  outputBytes: number,
  media?: MediaMetricsV1,
): TaskMetricsV1 => {
  const input = normalizeMeasuredBytes(inputBytes)
  const output = normalizeMeasuredBytes(outputBytes)
  return {
    schemaVersion: 1,
    inputBytes: input,
    outputBytes: output,
    savingsRatio: input > 0 ? (input - output) / input : 0,
    ...(media ? { media } : {}),
  }
}
