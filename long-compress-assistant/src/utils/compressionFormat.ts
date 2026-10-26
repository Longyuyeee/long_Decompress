export type CompressionFormatId =
  | 'tar.lzma'
  | 'lzma.aes'
  | 'tar.lzma.aes'
  | 'zpaq'
  | 'zip'
  | 'zipx'
  | '7z'
  | 'rar'
  | 'swm'
  | 'wim'
  | 'esd'
  | 'tar'
  | 'tar.gz'
  | 'tar.bz2'
  | 'tar.xz'
  | 'tar.zst'
  | 'lz4.aes'
  | 'br.aes'
  | 'lz.aes'
  | 'tar.lz4.aes'
  | 'tar.br.aes'
  | 'tar.lz.aes'
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
  | 'lz4'
  | 'br'
  | 'tar.lz4'
  | 'tar.br'
  | 'lz'
  | 'tar.lz'

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
  { format: 'zpaq', displayName: 'ZPAQ', extensions: ['zpaq'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', engineFormat: 'ZPAQ', knownLimitations: '创建新的日志式归档，支持原生密码；密码同时保护目录和内容。压缩强度 1～9 对应引擎 1～5 级，使用 16 MiB 块和两个线程。支持数字切片分卷，解压需集齐全部成员；不是 ZPAQ 原生多卷。不提供增量更新或历史恢复。' },
  { format: 'zip', displayName: 'ZIP', extensions: ['zip'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native' },
  { format: 'zipx', knownLimitations: '使用 XZ 方法；支持数字卷与 .zx01/…/.zipx 标准卷，接收方需使用支持该卷型及方法的软件。', displayName: 'ZIPX（XZ）', extensions: ['zipx'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native' },
  { format: '7z', displayName: '7Z', extensions: ['7z'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native' },
  { format: 'rar', displayName: 'RAR', extensions: ['rar'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: true, fallbackEngine: 'winrar', knownLimitations: 'RAR creation, including encrypted RAR, requires user-installed WinRAR/RAR command line tools.' },
  { format: 'esd', displayName: 'ESD（固实）', extensions: ['esd'], canCompress: true, canExtract: true, supportsPasswordCompress: false, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: true, requiresWinRar: false, fallbackEngine: '7za', engineFormat: 'wim', knownLimitations: '使用 LZMS 固实内容归档，单线程；更耗内存和时间，选择性提取也可能需要读取整个固实块。数字切卷需整组保留，不是原生 SWM；不含原生密码，不保证系统部署或 Windows 权限元数据。' },
  { format: 'swm', displayName: 'SWM（原生分包）', extensions: ['swm'], canCompress: true, canExtract: true, supportsPasswordCompress: false, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: true, requiresWinRar: false, fallbackEngine: '7za', engineFormat: 'wim', knownLimitations: '创建仅存储、XPRESS、LZX 非固实原生资源分包，按目标卷大小生成 name.swm、name2.swm 等；单个资源可能超过目标大小。需整组保留，没有原生密码；只承诺内容归档，不保证 Windows 部署元数据。' },
  { format: 'wim', displayName: 'WIM', extensions: ['wim'], canCompress: true, canExtract: true, supportsPasswordCompress: false, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: true, requiresWinRar: false, fallbackEngine: '7za', engineFormat: 'wim', knownLimitations: '创建可选仅存储、XPRESS、LZX、LZMS 或 LZMS 固实压缩，均支持去重，不使用强度滑块；数字切卷不是原生 SWM，需要集齐整组；WIM 不支持原生密码。多镜像按编号目录浏览。' },
  { format: 'tar', displayName: 'TAR', extensions: ['tar', 'ova'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.gz', displayName: 'TGZ', extensions: ['tar.gz', 'tgz', 'tpz'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.bz2', displayName: 'TBZ', extensions: ['tar.bz2', 'tbz', 'tbz2'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.xz', displayName: 'TXZ', extensions: ['tar.xz', 'txz'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.zst', displayName: 'TZST', extensions: ['tar.zst', 'tzst'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.lz4.aes', displayName: 'TAR.LZ4.AES 🔒', extensions: ['tar.lz4.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持数字分卷。接收方请使用 Long解压，不是内层格式的原生密码标准。' },
  { format: 'tar.br.aes', displayName: 'TAR.BR.AES 🔒', extensions: ['tar.br.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持数字分卷。接收方请使用 Long解压，不是内层格式的原生密码标准。' },
  { format: 'tar.lz.aes', displayName: 'TAR.LZIP.AES 🔒', extensions: ['tar.lz.aes', 'tlz.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持数字分卷。接收方请使用 Long解压，不是内层格式的原生密码标准。' },
  { format: 'lz4.aes', displayName: 'LZ4.AES 🔒', extensions: ['lz4.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持数字分卷。接收方请使用 Long解压，不是内层格式的原生密码标准。' },
  { format: 'br.aes', displayName: 'BR.AES 🔒', extensions: ['br.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持数字分卷。接收方请使用 Long解压，不是内层格式的原生密码标准。' },
  { format: 'lz.aes', displayName: 'LZIP.AES 🔒', extensions: ['lz.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持数字分卷。接收方请使用 Long解压，不是内层格式的原生密码标准。' },
  { format: 'tar.aes', displayName: 'TAR.AES 🔒', extensions: ['tar.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持 .aes.001 数字分卷。接收方请使用 Long解压，不是通用 TAR 密码格式。' },
  { format: 'tar.gz.aes', displayName: 'TGZ.AES 🔒', extensions: ['tar.gz.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持 .aes.001 数字分卷。接收方请使用 Long解压，不是通用 TAR 密码格式。' },
  { format: 'tar.bz2.aes', displayName: 'TBZ.AES 🔒', extensions: ['tar.bz2.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持 .aes.001 数字分卷。接收方请使用 Long解压，不是通用 TAR 密码格式。' },
  { format: 'tar.xz.aes', displayName: 'TXZ.AES 🔒', extensions: ['tar.xz.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持 .aes.001 数字分卷。接收方请使用 Long解压，不是通用 TAR 密码格式。' },
  { format: 'tar.zst.aes', displayName: 'TZST.AES 🔒', extensions: ['tar.zst.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持 .aes.001 数字分卷。接收方请使用 Long解压，不是通用 TAR 密码格式。' },
  { format: 'gz', displayName: 'GZ', extensions: ['gz', 'gzip'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'bz2', displayName: 'BZ2', extensions: ['bz2', 'bzip2'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'xz', displayName: 'XZ', extensions: ['xz'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'zst', displayName: 'ZST', extensions: ['zst', 'zstd'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'gz.aes', displayName: 'GZ.AES 🔒', extensions: ['gz.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持 .aes.001 数字分卷。接收方请使用 Long解压，不是通用 TAR 密码格式。' },
  { format: 'bz2.aes', displayName: 'BZ2.AES 🔒', extensions: ['bz2.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持 .aes.001 数字分卷。接收方请使用 Long解压，不是通用 TAR 密码格式。' },
  { format: 'xz.aes', displayName: 'XZ.AES 🔒', extensions: ['xz.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持 .aes.001 数字分卷。接收方请使用 Long解压，不是通用 TAR 密码格式。' },
  { format: 'zst.aes', displayName: 'ZST.AES 🔒', extensions: ['zst.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'Long 自有 AES 容器，需要密码；支持 .aes.001 数字分卷。接收方请使用 Long解压，不是通用 TAR 密码格式。' },
  { format: 'lzma', displayName: 'LZMA', extensions: ['lzma'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.lzma', displayName: 'TAR.LZMA', extensions: ['tar.lzma'], canCompress: true, canExtract: true, supportsPasswordCompress: false, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: '适合目录或多个文件；支持数字分卷。需要密码时请选择 TAR.LZMA.AES；普通 TAR.LZMA 没有原生密码。' },
  { format: 'lzma.aes', displayName: 'LZMA.AES 🔒', extensions: ['lzma.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: '单文件 Long 自有加密格式；目录或多文件请选择 TAR.LZMA.AES。接收方需使用 Long解压 v1.5.24 或更新版本，数字分卷需集齐全部卷。' },
  { format: 'tar.lzma.aes', displayName: 'TAR.LZMA.AES 🔒', extensions: ['tar.lzma.aes'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: true, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: '适合目录或多文件的 Long 自有加密格式；接收方需使用 Long解压 v1.5.24 或更新版本。支持数字分卷，不是通用 LZMA 原生密码标准。' },
  { format: 'tar.lz4', displayName: 'TAR.LZ4', extensions: ['tar.lz4'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.br', displayName: 'TAR.BR', extensions: ['tar.br'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'lz4', displayName: 'LZ4', extensions: ['lz4'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'br', displayName: 'Brotli', extensions: ['br'], canCompress: true, canExtract: true, supportsPasswordCompress: true, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'container-7z' },
  { format: 'tar.lz', displayName: 'TAR.LZIP', extensions: ['tar.lz', 'tlz'], canCompress: true, canExtract: true, supportsPasswordCompress: false, supportsPasswordExtract: false, singleFileOnly: false, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: 'LZIP 无密码加密；支持 .lz.001 数字切卷。全部卷需放在同一目录，数字切卷不等于 LZIP 原生成员。' },
  { format: 'lz', displayName: 'LZIP', extensions: ['lz'], canCompress: true, canExtract: true, supportsPasswordCompress: false, supportsPasswordExtract: false, singleFileOnly: true, supportsSplit: true, requires7za: false, requiresWinRar: false, fallbackEngine: 'native', knownLimitations: '单文件格式；目录或多文件请选择 TAR.LZIP。无密码，支持 .lz.001 数字切卷，解压需集齐全部卷。' },
]

export interface ExtractOnlyFormatCapability {
  displayName: string
  extensions: string[]
  requires7za: boolean
}

export const EXTRACT_ONLY_FORMATS: ExtractOnlyFormatCapability[] = [
  { displayName: 'LZOP（单文件）', extensions: ['lzo'], requires7za: false },
  { displayName: 'ALZ', extensions: ['alz'], requires7za: false },
  { displayName: 'EGG', extensions: ['egg'], requires7za: false },
  { displayName: 'Application packages', extensions: ['jar', 'xpi', 'ipa', 'apk', 'appx'], requires7za: false },
  { displayName: 'Disk images', extensions: ['iso', 'img', 'udf', 'dmg', 'vhd', 'vhdx', 'qcow', 'qcow2', 'qcow2c', 'vdi', 'vmdk'], requires7za: true },
  { displayName: 'Installers and packages', extensions: ['cab', 'deb', 'udeb', 'rpm', 'msi', 'msp', 'msm', 'nsis'], requires7za: true },
  { displayName: 'Filesystems and firmware', extensions: ['apfs', 'ext', 'ext2', 'ext3', 'ext4', 'gpt', 'mbr', 'uefif', 'cramfs', 'fat', 'ntfs', 'hfs', 'hfsx'], requires7za: true },
  { displayName: 'Legacy archives', extensions: ['ar', 'a', 'arj', 'z', 'lzh', 'lha', 'squashfs', 'sfs', 'xar', 'cpio', 'ihex'], requires7za: true },
]

export const COMPRESSIBLE_FORMATS = FORMAT_CAPABILITIES
  .filter(format => format.canCompress)
  .map(format => ({
    value: format.format,
    name: format.displayName,
    singleFileOnly: format.singleFileOnly,
    supportsSplit: format.supportsSplit,
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
    || /\.[a-j]\d{2}$/.test(normalizedPath)
    || /\.zx\d{2,5}$/.test(normalizedPath)
}

// Candidates only: native code verifies the embedded ZIP/7Z/RAR before import.
// Do not add EXE to format capabilities or system file associations.
export const isPotentialSfxPath = (path: string) => /\.exe$/i.test(path.trim())

// These names cannot be represented reliably by a short native file-dialog
// extension filter. They are only candidates: the Rust detector verifies the
// sibling volumes before a task is created.
export const isPotentialSplitArchivePath = (path: string) => {
  const normalizedPath = path.trim().toLowerCase()
  return /(?:^|[\\/.])haozip\d{2,5}\.(zip|7z)$/.test(normalizedPath)
    || /\.vol\d+\.egg$/.test(normalizedPath)
    || /\.[a-j]\d{2}$/.test(normalizedPath)
    || /(?:\.7z)?\.\d{3,}$/.test(normalizedPath)
    || /\.zx?\d{2,5}$/.test(normalizedPath)
    || /\.r\d{2,3}$/.test(normalizedPath)
    || /\.part\d+(?:\.rar)?$/.test(normalizedPath)
}

// Native pickers cannot express the full split-volume patterns used by the
// runtime detector. Include the conventional first-volume suffixes so users
// can still select a split archive while unrelated files remain hidden.
export const DECOMPRESS_ARCHIVE_PICKER_EXTENSIONS = [
  ...DECOMPRESS_ARCHIVE_EXTENSIONS,
  '001',
  'z01',
  'zx01',
  'r00',
  'a00',
  'exe',
]

export const DECOMPRESS_ARCHIVE_ACCEPT = DECOMPRESS_ARCHIVE_PICKER_EXTENSIONS
  .map(extension => `.${extension}`)
  .join(',')

export const DECOMPRESS_ARCHIVE_HINT = 'ZIP · 7Z · RAR · TAR · GZ · BZ2 · XZ · Zstd · LZ4 · Brotli · LZIP · TAR.LZIP · ISO · IMG · DMG · WIM · SWM · VHD · CAB · DEB · RPM · MSI · JAR · APK · IPA · LZH · CPIO · XAR + 更多'

export const COMPRESSION_FORMAT_HINT = 'ZIP · ZIPX（XZ） · 7Z · RAR · WIM · ESD · TAR · TAR.GZ · TAR.BZ2 · TAR.XZ · TAR.Zst · GZ · BZ2 · XZ · Zstd · LZ4 · Brotli · TAR.LZ4 · TAR.BR · LZIP · TAR.LZIP · LZMA · TAR.LZMA · LZMA.AES 🔒 · TAR.LZMA.AES 🔒 · TAR.AES 🔒 · TGZ.AES 🔒 · TBZ.AES 🔒 · TXZ.AES 🔒 · TZST.AES 🔒 · GZ.AES 🔒 · BZ2.AES 🔒 · XZ.AES 🔒 · ZST.AES 🔒'

const TAR_FORMATS = new Set(
  FORMAT_CAPABILITIES
    .filter(format => format.format.startsWith('tar.'))
    .map(format => format.format)
)
// RAR stays in this set so a stale RAR+password configuration is rejected as
// unsupported instead of silently changing the requested output format to 7Z.
const NATIVE_PASSWORD_FORMATS = new Set(['zip', 'zipx', '7z', 'rar', 'zpaq', 'lzma.aes', 'tar.lzma.aes', 'lz4.aes', 'br.aes', 'lz.aes', 'tar.lz4.aes', 'tar.br.aes', 'tar.lz.aes', 'tar.aes', 'tar.gz.aes', 'tar.bz2.aes', 'tar.xz.aes', 'tar.zst.aes', 'gz.aes', 'bz2.aes', 'xz.aes', 'zst.aes'])
const FORMAT_BY_ID = new Map(FORMAT_CAPABILITIES.map(format => [format.format, format]))

const normalizeFormatId = (format: string): CompressionFormatId | null => {
  if (format === 'zstd') return 'zst'
  if (format === 'tlz') return 'tar.lz'
  if (format === 'tlz.aes') return 'tar.lz.aes'
  return FORMAT_BY_ID.has(format as CompressionFormatId) ? format as CompressionFormatId : null
}

export const effectiveFormatForPassword = (format: string, password?: string) => {
  const normalized = normalizeFormatId(format) || format
  if (FORMAT_BY_ID.get(normalized as CompressionFormatId)?.supportsPasswordCompress === false) return normalized
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

export const supportsNativePassword = (format: string) => NATIVE_PASSWORD_FORMATS.has(normalizeFormatId(format) || format)
export const supportsArchiveSplit = (format: string) => FORMAT_BY_ID.get(normalizeFormatId(format)!)?.supportsSplit ?? false
export const supportsFilenameEncryption = (format: string) => ['7z', 'rar'].includes(format)
export const supportsSolidCompression = (format: string) => ['7z', 'rar'].includes(format)

export const isLz4Format = (format: string) => /(?:^|\.)lz4(?:\.aes)?$/.test(format)
