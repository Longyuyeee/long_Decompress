import {describe, expect, it} from 'vitest'
import {DECOMPRESS_ARCHIVE_ACCEPT, DECOMPRESS_ARCHIVE_EXTENSIONS, DECOMPRESS_ARCHIVE_PICKER_EXTENSIONS, EXTRACT_ONLY_FORMATS, FORMAT_CAPABILITIES, isDecompressArchivePath, supportsArchiveSplit, isPasswordSupportedFormat} from '../compressionFormat'

describe('LZOP 1.6.57 read-only integration', () => {
  it('imports .lzo through shared picker/drop/association extensions', () => {
    expect(DECOMPRESS_ARCHIVE_EXTENSIONS).toContain('lzo')
    expect(DECOMPRESS_ARCHIVE_PICKER_EXTENSIONS).toContain('lzo')
    expect(DECOMPRESS_ARCHIVE_ACCEPT).toContain('.lzo')
    expect(isDecompressArchivePath('C:\\资料\\notes.TXT.LZO')).toBe(true)
    expect(isDecompressArchivePath('notes.lzo.backup')).toBe(false)
  })
  it('supports native regular-file member creation without passwords or creating volumes', () => {
    expect(EXTRACT_ONLY_FORMATS.some(c => c.extensions.includes('lzo'))).toBe(false)
    expect(FORMAT_CAPABILITIES.find(c => c.format === 'lzo')).toMatchObject({canCompress:true,singleFileOnly:false,requires7za:false})
    expect(supportsArchiveSplit('lzo')).toBe(false)
    expect(isPasswordSupportedFormat('lzo')).toBe(false)
  })
})
