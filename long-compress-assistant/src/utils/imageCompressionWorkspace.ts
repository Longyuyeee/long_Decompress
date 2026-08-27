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
