export type CompressionFormatId =
  | 'zip'
  | '7z'
  | 'rar'
  | 'tar'
  | 'tar.gz'
  | 'tar.bz2'
  | 'tar.xz'
  | 'tar.zst'
  | 'gz'
  | 'bz2'
  | 'xz'
  | 'zst'
  | 'zstd'
  | 'lzma'

export interface CompressionFormatCapability {
  format: CompressionFormatId
  displayName: string
  extensions: string[]
  canCompress: boolean
  canExtract: boolean
  supportsPasswordCompress: boolean
  supportsPasswordExtract: boolean
  singleFileOnly: boolean
  supportsSplit: boolean
  requires7za: boolean
  requiresWinRar: boolean
  fallbackEngine: 'native' | '7za' | 'winrar' | 'container-7z'
  knownLimitations?: string
}

export const FORMAT_CAPABILITIES: CompressionFormatCapability[] = [
  { format: 'zip', displayName: 'ZIP', extensions: ['zip', 'zipx'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native' },
  { format: '7z', displayName: '7Z', extensions: ['7z'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native' },
  { format: 'rar', displayName: 'RAR', extensions: ['rar'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: true, fallbackEngine: 'winrar', knownLimitations: 'RAR creation requires WinRAR/RAR command line tools.' },
  { format: 'tar', displayName: 'TAR', extensions: ['tar', 'ova'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.gz', displayName: 'TGZ', extensions: ['tar.gz', 'tgz', 'tpz'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.bz2', displayName: 'TBZ', extensions: ['tar.bz2', 'tbz', 'tbz2'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.xz', displayName: 'TXZ', extensions: ['tar.xz', 'txz'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.zst', displayName: 'TZST', extensions: ['tar.zst', 'tzst'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'gz', displayName: 'GZ', extensions: ['gz', 'gzip'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'bz2', displayName: 'BZ2', extensions: ['bz2', 'bzip2'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'xz', displayName: 'XZ', extensions: ['xz'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'zst', displayName: 'ZST', extensions: ['zst', 'zstd'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'lzma', displayName: 'LZMA', extensions: ['lzma'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: false, requires7za: true, requiresWinRar: false, fallbackEngine: '7za' },
]

export interface ExtractOnlyFormatCapability {
  displayName: string
  extensions: string[]
  requires7za: boolean
}

export const EXTRACT_ONLY_FORMATS: ExtractOnlyFormatCapability[] = [
  { displayName: 'ZIP containers', extensions: ['jar', 'xpi', 'odt', 'ods', 'docx', 'xlsx', 'pptx', 'epub', 'ipa', 'apk', 'appx'], requires7za: false },
  { displayName: 'Disk images', extensions: ['iso', 'img', 'dmg', 'wim', 'vhd', 'vhdx'], requires7za: true },
  { displayName: 'Installers and packages', extensions: ['cab', 'deb', 'rpm', 'msi', 'nsis'], requires7za: true },
  { displayName: 'Legacy and filesystem archives', extensions: ['lzh', 'lha', 'arj', 'chm', 'squashfs', 'sfs', 'xar', 'cpio', 'udf', 'fat', 'ntfs', 'hfs'], requires7za: true },
]

export const COMPRESSIBLE_FORMATS = FORMAT_CAPABILITIES
  .filter(format => format.canCompress)
  .map(format => ({
    value: format.format,
    name: format.displayName,
    singleFileOnly: format.singleFileOnly,
    requires7za: format.requires7za,
    requiresWinRar: format.requiresWinRar,
    knownLimitations: format.knownLimitations,
  }))

export const DECOMPRESS_ARCHIVE_EXTENSIONS = [
  ...FORMAT_CAPABILITIES.flatMap(format => format.extensions),
  ...EXTRACT_ONLY_FORMATS.flatMap(format => format.extensions),
]

export const DECOMPRESS_ARCHIVE_ACCEPT = DECOMPRESS_ARCHIVE_EXTENSIONS
  .map(extension => `.${extension}`)
  .join(',')

export const DECOMPRESS_ARCHIVE_HINT = 'ZIP, 7Z, RAR, TAR, Zstd, ISO, CAB, DEB, RPM, DMG, MSI + 30 more'

const TAR_FORMATS = new Set(
  FORMAT_CAPABILITIES
    .filter(format => format.format.startsWith('tar.'))
    .map(format => format.format)
)
const NATIVE_PASSWORD_FORMATS = new Set(['zip', '7z', 'rar'])
const FORMAT_BY_ID = new Map(FORMAT_CAPABILITIES.map(format => [format.format, format]))

const normalizeFormatId = (format: string): CompressionFormatId | null => {
  if (format === 'zstd') return 'zst'
  return FORMAT_BY_ID.has(format as CompressionFormatId) ? format as CompressionFormatId : null
}

export const extensionForFormat = (format: string, password?: string) => {
  if (password && !NATIVE_PASSWORD_FORMATS.has(format)) return '7z'
  if (TAR_FORMATS.has(format as CompressionFormatId)) return format
  if (format === 'zstd') return 'zst'
  return format
}

export const isPasswordSupportedFormat = (format: string) => {
  const normalized = normalizeFormatId(format)
  return normalized ? FORMAT_BY_ID.get(normalized)?.supportsPasswordCompress ?? false : false
}

export const isSingleFileStreamFormat = (format: string) => {
  const normalized = normalizeFormatId(format)
  return normalized ? FORMAT_BY_ID.get(normalized)?.singleFileOnly ?? false : false
}
