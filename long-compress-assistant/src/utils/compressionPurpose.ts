import type { CompressionOptions } from '@/stores/compression'
import type { ConversionPatch } from '@/types/archiveConversion'

export type CompressionPurposeId = 'share' | 'smaller' | 'encrypted' | 'split'
export const compressionPurposes = [
  { id: 'share', name: '兼容分享', icon: 'pi pi-send', parameters: 'ZIP · Deflate · 强度 6', hint: '便于分享。已有密码和分卷保持；接收方仍需支持相应的加密方式或卷型。' },
  { id: 'smaller', name: '体积优先', icon: 'pi pi-box', parameters: '7Z · 强度 9', hint: '适合归档，可能需要更多时间和内存；实际体积取决于内容，图片、视频未必明显变小。接收方需支持 7Z。' },
  { id: 'encrypted', name: '加密分享', icon: 'pi pi-lock', parameters: '7Z · 强度 6 · 密码与文件名加密', hint: '接收方需使用支持加密 7Z 的工具（如 Long解压或 7-Zip）。请另外告知密码。' },
  { id: 'split', name: '按大小分包', icon: 'pi pi-th-large', parameters: 'ZIP · Deflate · 强度 6 · 标准分卷', hint: '生成 .z01、.z02、…、.zip，接收方需集齐整组并使用支持标准 ZIP 分卷的工具。大小按 MiB 计算。' },
] as const

export type PurposePatch = Partial<Pick<CompressionOptions, 'format' | 'level' | 'sevenZipMethod' | 'zipMethod' | 'password' | 'encryptFilenames' | 'splitArchive' | 'splitSize' | 'zipSplitMode'>>

/** Only explicit choices produce a patch. No filenames, destinations, input passwords or deletion flags. */
export function compressionPurposePatch(id: CompressionPurposeId, options: { password?: string; reusePassword?: boolean; splitMiB?: number } = {}): PurposePatch {
  if (id === 'encrypted') {
    if (!options.password && !options.reusePassword) throw new Error('请填写新包密码，或沿用已有密码。')
    return { format: '7z', level: 6, encryptFilenames: true, ...(options.password ? { password: options.password } : {}) }
  }
  if (id === 'smaller') return { format: '7z', level: 9, sevenZipMethod: 'lzma2' }
  const zip: PurposePatch = { format: 'zip', zipMethod: 'deflate', level: 6, encryptFilenames: false }
  if (id === 'split') {
    const size = options.splitMiB ?? 1024
    if (!Number.isSafeInteger(size) || size < 1) throw new Error('分卷大小请输入大于等于 1 的整数 MiB。')
    return { ...zip, splitArchive: true, splitSize: String(size), zipSplitMode: 'standard' }
  }
  return zip
}

export function conversionPurposePatch(patch: PurposePatch): ConversionPatch {
  const { splitArchive, splitSize, ...common } = patch
  return { ...common, ...(splitArchive === true ? { splitMiB: Number(splitSize) } : {}) }
}
