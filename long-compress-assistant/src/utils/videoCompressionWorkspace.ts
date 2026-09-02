import type { VideoCompressionSettings } from '@/types/video'

export interface VideoCandidate {
  name: string
  path: string
  size: number
  isDirectory: boolean
}

export const createDefaultVideoSettings = (): VideoCompressionSettings => ({
  preset: 'balanced',
  quality: 76,
  maxWidth: null,
  maxHeight: null,
})

export const cloneVideoSettings = (settings: VideoCompressionSettings): VideoCompressionSettings => ({ ...settings })

export const validateVideoCandidate = (candidate: VideoCandidate): { accepted: true } | { accepted: false, reason: string } => {
  if (!candidate.path.trim()) return { accepted: false, reason: '无法读取文件路径' }
  if (candidate.isDirectory) return { accepted: false, reason: '视频探测不接受目录' }
  return { accepted: true }
}
