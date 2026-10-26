export type SevenZipMethod = 'lzma2' | 'lzma' | 'bzip2' | 'deflate' | 'ppmd' | 'copy'
export const SEVEN_ZIP_METHOD_OPTIONS = [
  { value: 'lzma2', label: 'LZMA2 · 默认', detail: '沿用当前 7Z 压缩方式' },
  { value: 'lzma', label: 'LZMA', detail: '经典 7Z 方法' },
  { value: 'bzip2', label: 'Bzip2', detail: '尝试不同压缩效果' },
  { value: 'deflate', label: 'Deflate', detail: '7Z 内的 Deflate 编码' },
  { value: 'ppmd', label: 'PPMd', detail: 'PPMdH · 文本归档' },
  { value: 'copy', label: '仅归档（Copy）', detail: '快速打包 · 不再压缩数据' },
]
export const isSevenZipMethod = (value: unknown): value is SevenZipMethod => value === 'lzma2' || value === 'lzma' || value === 'bzip2' || value === 'deflate' || value === 'ppmd' || value === 'copy'
export function normalizeSevenZipMethod(value?: unknown): SevenZipMethod {
  if (value === undefined) return 'lzma2'
  if (isSevenZipMethod(value)) return value
  throw new Error('请选择支持的 7Z 压缩方法：LZMA2、LZMA、Bzip2、Deflate、PPMd 或仅归档（Copy）。')
}
export function compressionAnalysisFormat(settings: { format: string; sevenZipMethod?: SevenZipMethod; sevenZipFilter?: 'none' | 'delta' | 'bcj_x86' | 'bcj_arm32' | 'bcj_arm_thumb'; sevenZipDeltaDistance?: number; zipMethod?: string; level?: number }): string {
  if (settings.format === '7z') { const method = normalizeSevenZipMethod(settings.sevenZipMethod); if (settings.sevenZipFilter === 'delta' && ['lzma2','lzma'].includes(method)) return `7z-${method}-delta-${settings.sevenZipDeltaDistance ?? 1}`; if (settings.sevenZipFilter === 'bcj_x86' && ['lzma2','lzma'].includes(method)) return `7z-${method}-bcj-x86`; if (settings.sevenZipFilter === 'bcj_arm32' && ['lzma2','lzma'].includes(method)) return `7z-${method}-bcj-arm32`; if (settings.sevenZipFilter === 'bcj_arm_thumb' && ['lzma2','lzma'].includes(method)) return `7z-${method}-bcj-arm-thumb`; return method === 'lzma2' ? '7z' : `7z-${method}` }
  if (settings.format === 'zipx') return 'xz'
  if (settings.format === 'zip' && (settings.zipMethod === 'store' || (!settings.zipMethod || settings.zipMethod === 'deflate') && settings.level === 0)) return 'zip-store'
  if (settings.format === 'zip') return ({ zstd: 'zip-zstd', bzip2: 'zip-bzip2', lzma: 'zip-lzma', ppmd: 'zip-ppmd' } as Record<string, string>)[settings.zipMethod || 'deflate'] || 'zip'
  return settings.format
}
