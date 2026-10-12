import { expect, it } from 'vitest'
import { parentDirectory, resultLocation, uniqueResultDirectories } from '../resultLocation'

it('deduplicates recorded directories across files, extraction and Windows spelling', () => {
  expect(uniqueResultDirectories([
    { path: 'D:\\Output\\a.zip', directory: false }, { path: 'd:/output/b.mp4', directory: false },
    { path: 'D:/Output/', directory: true }, { path: 'E:/images/a.jpg', directory: false },
    { path: '', directory: true },
  ])).toEqual(['D:\\Output', 'E:/images'])
  expect(parentDirectory('C:\\archive.zip')).toBe('C:\\')
  expect(uniqueResultDirectories([{ path: '\\\\server\\share\\a.zip', directory: false }, { path: '//server/share/b.zip', directory: false }])).toHaveLength(1)
})

it('uses the published first volume and omits unfinished and skipped outputs', () => {
  expect(resultLocation({ status: 'running', type: 'compression', outputPath: 'D:/planned.zip' })).toBeNull()
  expect(resultLocation({ status: 'completed', type: 'compression', workloadKind: 'image', outputPath: 'D:/skipped.jpg' })).toBeNull()
  expect(resultLocation({ status: 'completed', taskType: 'decompression', outputPath: 'D:/unpacked' })).toEqual({ path: 'D:/unpacked', directory: true })
  expect(resultLocation({ status: 'completed', type: 'compression', outputPath: 'D:/planned.zip', archiveOutput: {
    primaryPath: 'D:/real.zip.001', files: [], split: true, totalBytes: 42,
  } })).toEqual({ path: 'D:/real.zip.001', directory: false })
})
