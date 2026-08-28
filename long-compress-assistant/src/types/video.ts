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
