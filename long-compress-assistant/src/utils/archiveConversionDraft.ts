import type { ConversionSettings } from '@/types/archiveConversion'
import { parentDirectory } from './resultLocation'
import { archiveSourceStem } from './archiveSource'
import { supportsNativePassword, supportsArchiveSplit, supportsFilenameEncryption } from './compressionFormat'

export function defaultConversionSettings(): ConversionSettings {
  return { format: '7z', level: 6, outputDirectory: '', outputPath: '', sourcePassword: '', password: '',
    splitMiB: 0, encryptFilenames: false, sevenZipFilter: 'none', sevenZipDeltaDistance: 1, sevenZipMethod: 'lzma2', zipMethod: 'deflate', zipEncryption: 'aes256', zipPasswordEncoding: 'utf8', zipSplitMode: 'numeric' }
}
export function conversionOutputPath(source: string, settings: ConversionSettings, intent = 'convert'): string {
  if (settings.outputPath) return settings.outputPath
  const stem = archiveSourceStem(source) + (settings.sourceZpaqSnapshot ? `_快照${settings.sourceZpaqSnapshot.version}` : '')
  const directory = settings.outputDirectory || parentDirectory(source)
  return `${directory.replace(/[\\/]$/, '')}/${stem}_${intent === 'zpaq-update' ? '更新' : intent === 'reconfigure' ? '调整' : '转换'}.${settings.format}`
}
export function normalizeConversionSettings(settings: ConversionSettings, patch: Partial<ConversionSettings>): ConversionSettings {
  const next = { ...settings, ...patch }
  if (next.format === 'swm') {
    if (!['store','xpress','lzx'].includes(next.wimMethod || '')) next.wimMethod = 'store'
    if (patch.format === 'swm' && !next.splitMiB) next.splitMiB = 1024
  }
  if (next.format === 'esd') next.wimMethod = 'solid'
  if (next.format !== '7z') { next.sevenZipFilter = settings.sevenZipFilter; next.sevenZipDeltaDistance = settings.sevenZipDeltaDistance }
  next.sevenZipMethod = next.format === '7z' ? next.sevenZipMethod ?? 'lzma2' : settings.sevenZipMethod ?? 'lzma2'
  next.zipMethod = next.format === 'zip' ? next.zipMethod ?? 'deflate' : settings.zipMethod ?? 'deflate'
  if (patch.format && next.outputPath.toLowerCase().endsWith(`.${settings.format}`)) {
    next.outputPath = next.outputPath.slice(0, -settings.format.length) + patch.format
  }
  if (!supportsFilenameEncryption(next.format)) next.encryptFilenames = false
  if (!supportsNativePassword(next.format)) next.password = ''
  if (!supportsArchiveSplit(next.format)) next.splitMiB = 0
  return next
}
