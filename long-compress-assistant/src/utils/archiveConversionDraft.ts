import type { ConversionSettings } from '@/types/archiveConversion'
import { parentDirectory } from './resultLocation'

export function defaultConversionSettings(): ConversionSettings {
  return { format: '7z', level: 6, outputDirectory: '', outputPath: '', sourcePassword: '', password: '',
    splitMiB: 0, encryptFilenames: false, zipEncryption: 'aes256', zipPasswordEncoding: 'utf8' }
}
export function conversionOutputPath(source: string, settings: ConversionSettings, intent = 'convert'): string {
  if (settings.outputPath) return settings.outputPath
  const name = source.split(/[\\/]/).pop() || 'archive'
  const stem = name.replace(/(?:\.part\d+)?\.(?:tar\.(?:gz|bz2|xz|zst)|zip|7z|rar|tar|tgz|tbz2?|txz|tzst|wim)(?:\.\d+)?$/i, '')
  const directory = settings.outputDirectory || parentDirectory(source)
  return `${directory.replace(/[\\/]$/, '')}/${stem}_${intent === 'reconfigure' ? '调整' : '转换'}.${settings.format}`
}
export function normalizeConversionSettings(settings: ConversionSettings, patch: Partial<ConversionSettings>): ConversionSettings {
  const next = { ...settings, ...patch }
  if (patch.format && next.outputPath.toLowerCase().endsWith(`.${settings.format}`)) {
    next.outputPath = next.outputPath.slice(0, -settings.format.length) + patch.format
  }
  if (next.format !== '7z') next.encryptFilenames = false
  if (!['zip', '7z'].includes(next.format)) { next.password = ''; next.splitMiB = 0 }
  return next
}
