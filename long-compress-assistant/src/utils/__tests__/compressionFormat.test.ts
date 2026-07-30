import { describe, expect, it } from 'vitest'
import {
  COMPRESSIBLE_FORMATS,
  DECOMPRESS_ARCHIVE_ACCEPT,
  FORMAT_CAPABILITIES,
  effectiveFormatForPassword,
  extensionForFormat,
  isDecompressArchivePath,
  isPasswordSupportedFormat,
  isSingleFileStreamFormat,
} from '../compressionFormat'

describe('compression format helpers', () => {
  it('keeps native password formats unchanged', () => {
    expect(extensionForFormat('zip', 'secret')).toBe('zip')
    expect(extensionForFormat('7z', 'secret')).toBe('7z')
    expect(extensionForFormat('rar', 'secret')).toBe('rar')
  })

  it('routes non-native password formats to encrypted 7z containers', () => {
    expect(extensionForFormat('tar.gz', 'secret')).toBe('7z')
    expect(extensionForFormat('tar.zst', 'secret')).toBe('7z')
    expect(extensionForFormat('zst', 'secret')).toBe('7z')
    expect(extensionForFormat('lzma', 'secret')).toBe('7z')
    expect(effectiveFormatForPassword('tar.gz', 'secret')).toBe('7z')
    expect(effectiveFormatForPassword('zstd', 'secret')).toBe('7z')
    expect(effectiveFormatForPassword('tar.gz')).toBe('tar.gz')
  })

  it('uses natural extensions when password is empty', () => {
    expect(extensionForFormat('tar.gz')).toBe('tar.gz')
    expect(extensionForFormat('zstd')).toBe('zst')
    expect(extensionForFormat('xz')).toBe('xz')
  })

  it('reports supported password and single-file stream formats', () => {
    expect(isPasswordSupportedFormat('tar.zst')).toBe(true)
    expect(isPasswordSupportedFormat('rar')).toBe(true)
    expect(isPasswordSupportedFormat('wim')).toBe(false)
    expect(isPasswordSupportedFormat('exe')).toBe(false)
    expect(isSingleFileStreamFormat('zstd')).toBe(true)
    expect(isSingleFileStreamFormat('tar.zst')).toBe(false)
  })

  it('drives compressible formats from the capability matrix', () => {
    const compressible = FORMAT_CAPABILITIES.filter(format => format.canCompress)

    expect(COMPRESSIBLE_FORMATS.map(format => format.value)).toEqual(
      compressible.map(format => format.format)
    )
    expect(COMPRESSIBLE_FORMATS.some(format => format.value === 'tar.zst')).toBe(true)
    expect(COMPRESSIBLE_FORMATS.some(format => format.value === 'lzma' && format.requires7za)).toBe(true)
    expect(COMPRESSIBLE_FORMATS.some(format => format.value === 'wim' && format.engineFormat === 'wim')).toBe(true)
  })

  it('builds decompression accept list from supported extensions', () => {
    expect(DECOMPRESS_ARCHIVE_ACCEPT).toContain('.zip')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).toContain('.tar.zst')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).not.toContain('.docx')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).not.toContain('.xlsx')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).toContain('.iso')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).toContain('.rpm')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).toContain('.qcow2')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).toContain('.vmdk')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).toContain('.apfs')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).toContain('.hfsx')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).toContain('.cramfs')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).not.toContain('.ppkg')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).not.toContain('.arj')
  })

  it('accepts archives case-insensitively and rejects document containers', () => {
    expect(isDecompressArchivePath('C:/downloads/BACKUP.TAR.GZ')).toBe(true)
    expect(isDecompressArchivePath('C:/downloads/app.apk')).toBe(true)
    expect(isDecompressArchivePath('C:/documents/report.docx')).toBe(false)
    expect(isDecompressArchivePath('C:/documents/budget.xlsx')).toBe(false)
  })
})
