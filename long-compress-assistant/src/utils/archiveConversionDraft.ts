import type { ConversionSettings } from '@/types/archiveConversion'
import { parentDirectory } from './resultLocation'
import { archiveSourceStem } from './archiveSource'
import { supportsNativePassword, supportsArchiveSplit, supportsFilenameEncryption } from './compressionFormat'

export function defaultConversionSettings(): ConversionSettings {
  return { format: '7z', level: 6, outputDirectory: '', outputPath: '', sourcePassword: '', password: '',
    splitMiB: 0, encryptFilenames: false, zipEncryption: 'aes256', zipPasswordEncoding: 'utf8', zipSplitMode: 'numeric' }
}
export function conversionOutputPath(source: string, settings: ConversionSettings, intent = 'convert'): string {
  if (settings.outputPath) return settings.outputPath
  const stem = archiveSourceStem(source)
  const directory = settings.outputDirectory || parentDirectory(source)
  return `${directory.replace(/[\\/]$/, '')}/${stem}_${intent === 'reconfigure' ? '调整' : '转换'}.${settings.format}`
}
export function normalizeConversionSettings(settings: ConversionSettings, patch: Partial<ConversionSettings>): ConversionSettings {
  const next = { ...settings, ...patch }
  if (patch.format && next.outputPath.toLowerCase().endsWith(`.${settings.format}`)) {
    next.outputPath = next.outputPath.slice(0, -settings.format.length) + patch.format
  }
  if (!supportsFilenameEncryption(next.format)) next.encryptFilenames = false
  if (!supportsNativePassword(next.format)) next.password = ''
  if (!supportsArchiveSplit(next.format)) next.splitMiB = 0
  return next
}
