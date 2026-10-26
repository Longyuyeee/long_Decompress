import {describe,expect,it} from 'vitest'
import {DECOMPRESS_ARCHIVE_EXTENSIONS,DECOMPRESS_ARCHIVE_PICKER_EXTENSIONS,EXTRACT_ONLY_FORMATS,FORMAT_CAPABILITIES,isDecompressArchivePath,supportsArchiveSplit,isPasswordSupportedFormat} from '../compressionFormat'
describe('TAR.LZO 1.6.58 read-only integration',()=>{
 it('shares existing import and picker paths for the compound suffix',()=>{
  expect(DECOMPRESS_ARCHIVE_EXTENSIONS).toContain('tar.lzo')
  expect(DECOMPRESS_ARCHIVE_PICKER_EXTENSIONS).toContain('tar.lzo')
  expect(isDecompressArchivePath('C:\\资料\\BACKUP.TAR.LZO')).toBe(true)
  expect(isDecompressArchivePath('backup.tar.lzo.partial')).toBe(false)
  expect(FORMAT_CAPABILITIES.find(f=>f.format==='tar.lzo')?.requires7za).toBe(false)
 })
 it('allows TAR.LZO creation without claiming native passwords or split targets',()=>{
  expect(FORMAT_CAPABILITIES.find(f=>f.format==='tar.lzo')).toMatchObject({canCompress:true,singleFileOnly:false})
  expect(supportsArchiveSplit('tar.lzo')).toBe(false)
  expect(isPasswordSupportedFormat('tar.lzo')).toBe(false)
 })
})
