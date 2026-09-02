export type VideoFrameRateMode = 'variable' | 'constant-or-undetermined'

export interface VideoStreamFacts {
  index: number
  codec: string | null
  profile: string | null
  encodedWidth: number
  encodedHeight: number
  visibleWidth: number
  visibleHeight: number
  rotationDegrees: number
  pixelFormat: string | null
  colorTransfer: string | null
  hdr: boolean
  nominalFrameRate: string | null
  averageFrameRate: string | null
  averageFrameRateMilli: number | null
  frameRateMode: VideoFrameRateMode
  bitRate: number | null
  default: boolean
}

export interface AudioStreamFacts {
  index: number
  codec: string | null
  channels: number | null
  sampleRate: number | null
  bitRate: number | null
  language: string | null
  default: boolean
}

export interface SubtitleStreamFacts {
  index: number
  codec: string | null
  language: string | null
  default: boolean
  forced: boolean
}

export interface VideoFirstReleasePolicy {
  container: 'output-mp4'
  video: 'transcode-h264-mf-software'
  audio: 'preserve-primary-as-aac-when-present'
  additionalAudio: 'drop-with-explicit-warning'
  subtitles: 'drop-with-explicit-warning'
  chapters: 'drop-with-explicit-warning'
  attachedPictures: 'drop-with-explicit-warning'
  rotation: 'normalize-to-visible-pixel-orientation'
  variableFrameRate: 'preserve-input-timestamps'
  hdr: 'refuse-before-encoding'
}

export interface VideoProbeReport {
  source: string
  inputBytes: number
  container: string | null
  durationMs: number
  overallBitRate: number | null
  primaryVideo: VideoStreamFacts
  videoStreamCount: number
  audioStreams: AudioStreamFacts[]
  subtitleStreams: SubtitleStreamFacts[]
  chapterCount: number
  attachedPictureCount: number
  policy: VideoFirstReleasePolicy
  warnings: string[]
  blockingReasons: string[]
}

export type VideoCompressionPreset = 'clear' | 'balanced' | 'small'

export interface VideoCompressionSettings {
  preset: VideoCompressionPreset
  quality: number
  maxWidth: number | null
  maxHeight: number | null
}

export interface VideoCompressionPlanRequest extends VideoCompressionSettings {
  path: string
}

export interface VideoPresetFacts {
  preset: VideoCompressionPreset
  label: VideoCompressionPreset
  quality: number
  videoBitsPerPixelMilli: number
  minimumVideoBitRate: number
  maximumVideoBitRate: number
  audioBitRate: number
  defaultMaxWidth: number
  defaultMaxHeight: number
}

export interface VideoSizeEstimate {
  isEstimate: true
  lowBytes: number
  highBytes: number
  basis: 'duration-output-pixels-average-frame-rate-and-quality-bitrate-envelope'
  disclaimer: string
}

export interface VideoCompressionPlan {
  probe: VideoProbeReport
  preset: VideoPresetFacts
  effectiveMaxWidth: number
  effectiveMaxHeight: number
  outputWidth: number
  outputHeight: number
  willResize: boolean
  willUpscale: false
  aspectRatioPolicy: 'preserve-within-even-dimension-rounding'
  targetVideoBitRate: number
  targetAudioBitRate: number | null
  estimatedOutput: VideoSizeEstimate
  streamChanges: string[]
  requiresExplicitConfirmation: boolean
  canEncode: boolean
}

export interface VideoCompressionExecutionRequest {
  plan: VideoCompressionPlanRequest
  destination: string
  /** Must exactly match the latest backend plan when lossy stream changes exist. */
  confirmedStreamChanges: string[]
  preserveMarkOfWeb: boolean
}

export interface VideoCompressionDestinationPlan {
  destination: string
}

export interface VerifiedVideoOutput {
  encodedBytes: number
  container: 'mp4'
  durationMs: number
  durationDifferenceMs: number
  durationToleranceMs: number
  videoCodec: 'h264'
  audioCodec: 'aac' | null
  encodedWidth: number
  encodedHeight: number
  visibleWidth: number
  visibleHeight: number
  rotationDegrees: number
  decodedVideoFrames: number
}

export interface PublishedVideoOutput {
  path: string
  inputBytes: number
  outputBytes: number
  savingsRatio: number
  markOfTheWeb: 'applied' | 'not-present' | 'disabled'
  verified: VerifiedVideoOutput
}
