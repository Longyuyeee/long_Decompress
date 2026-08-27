export type WorkloadKind = 'archive' | 'image' | 'video' | 'pdf'

export interface MediaMetricsV1 {
  width?: number
  height?: number
  frameCount?: number
  durationMs?: number
  pageCount?: number
  videoCodec?: string
  audioCodec?: string
  container?: string
  hasAlpha?: boolean
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

export const createMeasuredTaskMetricsV1 = (
  inputBytes: number,
  outputBytes: number,
  media?: MediaMetricsV1,
): TaskMetricsV1 => {
  const input = Math.max(0, Math.round(inputBytes))
  const output = Math.max(0, Math.round(outputBytes))
  return {
    schemaVersion: 1,
    inputBytes: input,
    outputBytes: output,
    savingsRatio: input > 0 ? (input - output) / input : 0,
    ...(media ? { media } : {}),
  }
}
