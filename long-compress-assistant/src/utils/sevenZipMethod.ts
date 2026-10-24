export type SevenZipMethod = 'lzma2' | 'lzma' | 'bzip2' | 'deflate'
export const SEVEN_ZIP_METHOD_OPTIONS = [
  { value: 'lzma2', label: 'LZMA2 · 默认', detail: '沿用当前 7Z 压缩方式' },
  { value: 'lzma', label: 'LZMA', detail: '经典 7Z 方法' },
  { value: 'bzip2', label: 'Bzip2', detail: '尝试不同压缩效果' },
  { value: 'deflate', label: 'Deflate', detail: '7Z 内的 Deflate 编码' },
]
export const isSevenZipMethod = (value: unknown): value is SevenZipMethod => value === 'lzma2' || value === 'lzma' || value === 'bzip2' || value === 'deflate'
export function normalizeSevenZipMethod(value?: unknown): SevenZipMethod {
  if (value === undefined) return 'lzma2'
  if (isSevenZipMethod(value)) return value
  throw new Error('请选择支持的 7Z 压缩方法：LZMA2、LZMA、Bzip2 或 Deflate。')
}
export function compressionAnalysisFormat(settings: { format: string; sevenZipMethod?: SevenZipMethod; zipMethod?: string }): string {
  if (settings.format === '7z') { const method = normalizeSevenZipMethod(settings.sevenZipMethod); return method === 'lzma2' ? '7z' : `7z-${method}` }
  if (settings.format === 'zipx') return 'xz'
  if (settings.format === 'zip') return ({ zstd: 'zst', bzip2: 'bz2', lzma: 'zip-lzma', ppmd: 'zip-ppmd' } as Record<string, string>)[settings.zipMethod || 'deflate'] || 'zip'
  return settings.format
}
