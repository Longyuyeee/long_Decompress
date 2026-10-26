import {describe,expect,it} from 'vitest'
import {DECOMPRESS_ARCHIVE_EXTENSIONS,DECOMPRESS_ARCHIVE_PICKER_EXTENSIONS,EXTRACT_ONLY_FORMATS,FORMAT_CAPABILITIES,isDecompressArchivePath,supportsArchiveSplit,isPasswordSupportedFormat,isPotentialSplitArchivePath} from '../compressionFormat'
describe('LZOP numeric 1.6.59 import integration',()=>{
 it('shares existing import and picker paths for the compound suffix',()=>{
  expect(DECOMPRESS_ARCHIVE_EXTENSIONS).toContain('tar.lzo')
  expect(DECOMPRESS_ARCHIVE_PICKER_EXTENSIONS).toContain('tar.lzo')
  expect(isDecompressArchivePath('C:\\资料\\BACKUP.TAR.LZO')).toBe(true)
  for (const name of ['backup.tar.lzo.001','photo.png.lzo.003','backup.tar.lzo.0002']) expect(isPotentialSplitArchivePath(name)).toBe(true)
  expect(isDecompressArchivePath('backup.tar.lzo.partial')).toBe(false)
  expect(EXTRACT_ONLY_FORMATS.find(f=>f.extensions.includes('tar.lzo'))?.requires7za).toBe(false)
 })
 it('does not invent creation, native passwords or native split targets',()=>{
  expect(FORMAT_CAPABILITIES.map(f=>f.format)).not.toContain('tar.lzo')
  expect(supportsArchiveSplit('tar.lzo')).toBe(false)
  expect(isPasswordSupportedFormat('tar.lzo')).toBe(false)
 })
})
