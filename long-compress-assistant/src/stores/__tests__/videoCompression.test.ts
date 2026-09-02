import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useCompressionStore } from '../compression'
import type { VideoCompressionPlan } from '@/types/video'

const plan = (): VideoCompressionPlan => ({
  probe: {
    source: 'C:/video.mp4', inputBytes: 100, container: 'mov,mp4', durationMs: 1_000, overallBitRate: null,
    primaryVideo: {
      index: 0, codec: 'h264', profile: null, encodedWidth: 640, encodedHeight: 360,
      visibleWidth: 640, visibleHeight: 360, rotationDegrees: 0, pixelFormat: 'yuv420p', colorTransfer: null,
      hdr: false, nominalFrameRate: '30/1', averageFrameRate: '30/1', averageFrameRateMilli: 30_000,
      frameRateMode: 'constant-or-undetermined', bitRate: null, default: true,
    },
    videoStreamCount: 1, audioStreams: [], subtitleStreams: [], chapterCount: 0, attachedPictureCount: 0,
    policy: {
      container: 'output-mp4', video: 'transcode-h264-mf-software', audio: 'preserve-primary-as-aac-when-present',
      additionalAudio: 'drop-with-explicit-warning', subtitles: 'drop-with-explicit-warning', chapters: 'drop-with-explicit-warning',
      attachedPictures: 'drop-with-explicit-warning', rotation: 'normalize-to-visible-pixel-orientation',
      variableFrameRate: 'preserve-input-timestamps', hdr: 'refuse-before-encoding',
    },
    warnings: [], blockingReasons: [],
  },
  preset: {
    preset: 'balanced', label: 'balanced', quality: 76, videoBitsPerPixelMilli: 75, minimumVideoBitRate: 800_000,
    maximumVideoBitRate: 8_000_000, audioBitRate: 128_000, defaultMaxWidth: 1_280, defaultMaxHeight: 720,
  },
  effectiveMaxWidth: 1_280, effectiveMaxHeight: 720, outputWidth: 640, outputHeight: 360,
  willResize: false, willUpscale: false, aspectRatioPolicy: 'preserve-within-even-dimension-rounding',
  targetVideoBitRate: 800_000, targetAudioBitRate: null,
  estimatedOutput: {
    isEstimate: true, lowBytes: 80_000, highBytes: 125_000,
    basis: 'duration-output-pixels-average-frame-rate-and-quality-bitrate-envelope', disclaimer: 'estimate only',
  },
  streamChanges: [], requiresExplicitConfirmation: false, canEncode: true,
})

describe('video compression draft state', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('keeps video drafts isolated and rejects directories without extension allowlists', () => {
    const store = useCompressionStore()
    store.addFile({ name: 'archive.txt', path: 'C:/archive.txt', size: 1, type: 'file', isDirectory: false })
    const result = store.addVideoCandidates([
      { name: 'unknown.data', path: 'C:/unknown.data', size: 100, isDirectory: false },
      { name: 'folder', path: 'C:/folder', size: 0, isDirectory: true },
    ])
    expect(result.accepted).toHaveLength(1)
    expect(result.rejected[0].reason).toContain('不接受目录')
    expect(store.videoItems).toHaveLength(1)
    expect(store.selectedFiles).toHaveLength(1)
    expect(store.imageItems).toHaveLength(0)
  })

  it('ignores stale plans and replans global and item overrides through one settings model', () => {
    const store = useCompressionStore()
    const item = store.addVideoCandidates([{ name: 'video.mp4', path: 'C:/video.mp4', size: 100, isDirectory: false }]).accepted[0]
    expect(item.planRevision).toBe(1)
    store.updateVideoGlobalSettings({ preset: 'small', quality: 42, maxWidth: 854, maxHeight: 480 })
    expect(item.planRevision).toBe(2)
    expect(store.completeVideoPlanning(item.id, 1, plan())).toBe(false)
    expect(store.completeVideoPlanning(item.id, 2, plan())).toBe(true)

    const readyPlan = item.plan
    const readyRevision = item.planRevision
    store.enableVideoItemOverride(item.id)
    expect(item.settings?.preset).toBe('small')
    expect(item.planRevision).toBe(readyRevision)
    expect(item.plan).toBe(readyPlan)
    const overrideRevision = item.planRevision
    store.updateVideoGlobalSettings({ preset: 'clear', quality: 92, maxWidth: null, maxHeight: null })
    expect(item.planRevision).toBe(overrideRevision)
    store.updateVideoItemSettings(item.id, { preset: 'balanced', quality: 76, maxWidth: null, maxHeight: null })
    expect(item.planRevision).toBe(overrideRevision + 1)
    expect(item.plan).toBe(readyPlan)
    expect(store.selectedFiles).toHaveLength(0)
    expect(store.imageItems).toHaveLength(0)
  })
})
