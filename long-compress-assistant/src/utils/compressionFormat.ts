export type CompressionFormatId =
  | 'zip'
  | '7z'
  | 'rar'
  | 'wim'
  | 'tar'
  | 'tar.gz'
  | 'tar.bz2'
  | 'tar.xz'
  | 'tar.zst'
  | 'tar.aes'
  | 'tar.gz.aes'
  | 'tar.bz2.aes'
  | 'tar.xz.aes'
  | 'tar.zst.aes'
  | 'gz'
  | 'bz2'
  | 'xz'
  | 'zst'
  | 'zstd'
  | 'gz.aes'
  | 'bz2.aes'
  | 'xz.aes'
  | 'zst.aes'
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
  engineFormat?: string
}

export const FORMAT_CAPABILITIES: CompressionFormatCapability[] = [
  { format: 'zip', displayName: 'ZIP', extensions: ['zip', 'zipx'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native' },
  { format: '7z', displayName: '7Z', extensions: ['7z'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native' },
  { format: 'rar', displayName: 'RAR', extensions: ['rar'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: true, fallbackEngine: 'winrar', knownLimitations: 'RAR creation, including encrypted RAR, requires user-installed WinRAR/RAR command line tools.' },
  { format: 'wim', displayName: 'WIM', extensions: ['wim'], canCompress: true, canExtract: true, supportsPasswordCompress: false, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: false, requires7za: true, requiresWinRar: false, fallbackEngine: '7za', engineFormat: 'wim', knownLimitations: 'WIM creation requires the bundled full 7-Zip engine and does not support password encryption.' },
  { format: 'tar', displayName: 'TAR', extensions: ['tar', 'ova'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.gz', displayName: 'TGZ', extensions: ['tar.gz', 'tgz', 'tpz'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.bz2', displayName: 'TBZ', extensions: ['tar.bz2', 'tbz', 'tbz2'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.xz', displayName: 'TXZ', extensions: ['tar.xz', 'txz'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.zst', displayName: 'TZST', extensions: ['tar.zst', 'tzst'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.aes', displayName: 'TAR.AES 🔒', extensions: ['tar.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'AES-256-GCM encrypted TAR archive. Password required.' },
  { format: 'tar.gz.aes', displayName: 'TGZ.AES 🔒', extensions: ['tar.gz.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'AES-256-GCM encrypted GZIP+TAR archive. Password required.' },
  { format: 'tar.bz2.aes', displayName: 'TBZ.AES 🔒', extensions: ['tar.bz2.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'AES-256-GCM encrypted BZ2+TAR archive. Password required.' },
  { format: 'tar.xz.aes', displayName: 'TXZ.AES 🔒', extensions: ['tar.xz.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'AES-256-GCM encrypted XZ+TAR archive. Password required.' },
  { format: 'tar.zst.aes', displayName: 'TZST.AES 🔒', extensions: ['tar.zst.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'AES-256-GCM encrypted Zstd+TAR archive. Password required.' },
  { format: 'gz', displayName: 'GZ', extensions: ['gz', 'gzip'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'bz2', displayName: 'BZ2', extensions: ['bz2', 'bzip2'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'xz', displayName: 'XZ', extensions: ['xz'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'zst', displayName: 'ZST', extensions: ['zst', 'zstd'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'gz.aes', displayName: 'GZ.AES 🔒', extensions: ['gz.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'AES-256-GCM encrypted GZIP file. Password required.' },
  { format: 'bz2.aes', displayName: 'BZ2.AES 🔒', extensions: ['bz2.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'AES-256-GCM encrypted BZ2 file. Password required.' },
  { format: 'xz.aes', displayName: 'XZ.AES 🔒', extensions: ['xz.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'AES-256-GCM encrypted XZ file. Password required.' },
  { format: 'zst.aes', displayName: 'ZST.AES 🔒', extensions: ['zst.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: false, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'AES-256-GCM encrypted Zstd file. Password required.' },
  { format: 'lzma', displayName: 'LZMA', extensions: ['lzma'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: false, requires7za: true, requiresWinRar: false, fallbackEngine: '7za' },
]

export interface ExtractOnlyFormatCapability {
  displayName: string
  extensions: string[]
  requires7za: boolean
}

export const EXTRACT_ONLY_FORMATS: ExtractOnlyFormatCapability[] = [
  { displayName: 'Application packages', extensions: ['jar', 'xpi', 'ipa', 'apk', 'appx'], requires7za: false },
  { displayName: 'Disk images', extensions: ['iso', 'img', 'dmg', 'vhd', 'vhdx', 'qcow', 'qcow2', 'qcow2c', 'vdi', 'vmdk'], requires7za: true },
  { displayName: 'Installers and packages', extensions: ['cab', 'deb', 'udeb', 'rpm', 'msi', 'msp', 'msm', 'nsis', 'ppkg'], requires7za: true },
  { displayName: 'Filesystems and firmware', extensions: ['apfs', 'apm', 'ext', 'ext2', 'ext3', 'ext4', 'gpt', 'mbr', 'uefif', 'scap', 'cramfs', 'udf', 'fat', 'ntfs', 'hfs', 'hfsx'], requires7za: true },
  { displayName: 'Legacy archives', extensions: ['ar', 'a', 'lzh', 'lha', 'arj', 'chm', 'squashfs', 'sfs', 'xar', 'cpio', 'ihex', 'z', 'taz'], requires7za: true },
]

export const COMPRESSIBLE_FORMATS = FORMAT_CAPABILITIES
  .filter(format => format.canCompress)
  .map(format => ({
    value: format.format,
    name: format.displayName,
    singleFileOnly: format.singleFileOnly,
    requires7za: format.requires7za,
    requiresWinRar: format.requiresWinRar,
    engineFormat: format.engineFormat,
    knownLimitations: format.knownLimitations,
  }))

export const DECOMPRESS_ARCHIVE_EXTENSIONS = [
  ...FORMAT_CAPABILITIES.flatMap(format => format.extensions),
  ...EXTRACT_ONLY_FORMATS.flatMap(format => format.extensions),
]

const DECOMPRESS_ARCHIVE_SUFFIXES = [...DECOMPRESS_ARCHIVE_EXTENSIONS]
  .sort((left, right) => right.length - left.length)
  .map(extension => `.${extension.toLowerCase()}`)

export const isDecompressArchivePath = (path: string) => {
  const normalizedPath = path.trim().toLowerCase()
  return DECOMPRESS_ARCHIVE_SUFFIXES.some(suffix => normalizedPath.endsWith(suffix))
}

export const DECOMPRESS_ARCHIVE_ACCEPT = DECOMPRESS_ARCHIVE_EXTENSIONS
  .map(extension => `.${extension}`)
  .join(',')

export const DECOMPRESS_ARCHIVE_HINT = 'ZIP · 7Z · RAR · TAR · GZ · BZ2 · XZ · Zstd · ISO · IMG · DMG · WIM · VHD · CAB · DEB · RPM · MSI · JAR · APK · IPA · LZH · ARJ · CHM · CPIO · XAR + 更多'

export const COMPRESSION_FORMAT_HINT = 'ZIP · 7Z · RAR · WIM · TAR · TAR.GZ · TAR.BZ2 · TAR.XZ · TAR.Zst · GZ · BZ2 · XZ · Zstd · LZMA · TAR.AES 🔒 · TGZ.AES 🔒 · TBZ.AES 🔒 · TXZ.AES 🔒 · TZST.AES 🔒 · GZ.AES 🔒 · BZ2.AES 🔒 · XZ.AES 🔒 · ZST.AES 🔒'

const TAR_FORMATS = new Set(
  FORMAT_CAPABILITIES
    .filter(format => format.format.startsWith('tar.'))
    .map(format => format.format)
)
// RAR stays in this set so a stale RAR+password configuration is rejected as
// unsupported instead of silently changing the requested output format to 7Z.
const NATIVE_PASSWORD_FORMATS = new Set(['zip', '7z', 'rar', 'tar.aes', 'tar.gz.aes', 'tar.bz2.aes', 'tar.xz.aes', 'tar.zst.aes', 'gz.aes', 'bz2.aes', 'xz.aes', 'zst.aes'])
const FORMAT_BY_ID = new Map(FORMAT_CAPABILITIES.map(format => [format.format, format]))

const normalizeFormatId = (format: string): CompressionFormatId | null => {
  if (format === 'zstd') return 'zst'
  return FORMAT_BY_ID.has(format as CompressionFormatId) ? format as CompressionFormatId : null
}

export const effectiveFormatForPassword = (format: string, password?: string) => {
  const normalized = normalizeFormatId(format) || format
  return password && !NATIVE_PASSWORD_FORMATS.has(normalized) ? '7z' : normalized
}

export const extensionForFormat = (format: string, password?: string) => {
  if (effectiveFormatForPassword(format, password) === '7z') return '7z'
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
