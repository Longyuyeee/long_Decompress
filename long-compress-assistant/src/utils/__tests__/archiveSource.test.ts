import { describe, expect, it } from 'vitest'
import { archiveGroupKey, archiveSourceFormat, archiveSourceStem } from '../archiveSource'
import { isPotentialSplitArchivePath } from '../compressionFormat'

describe('EGG native member names', () => {
  it.each(['资料.vol1.egg','资料.vol2.egg','资料.VOL103.EGG'])('shares a distinct native group for %s', name => {
    expect(archiveGroupKey(`F:/下载/${name}`)).toBe('f:/下载/资料.vol1.egg')
    expect(archiveSourceFormat(name)).toBe('egg')
    expect(archiveSourceStem(name)).toBe('资料')
    expect(isPotentialSplitArchivePath(name)).toBe(true)
    expect(archiveGroupKey(name)).not.toBe(archiveGroupKey('资料.egg'))
  })
})

describe('ALZ native member names', () => {
  it.each(['资料.alz', '资料.a00', '资料.A01', '资料.b00'])('shares the original group and clean output name for %s', name => {
    expect(archiveGroupKey(`F:/下载/${name}`)).toBe('f:/下载/资料.alz')
    expect(archiveSourceFormat(name)).toBe('alz')
    expect(archiveSourceStem(name)).toBe('资料')
    if (!name.endsWith('.alz')) expect(isPotentialSplitArchivePath(name)).toBe(true)
  })
})

describe('standard ZIPX sources', () => {
  it.each(['资料.zx01', '资料.ZX02', '资料.zx100', '资料.zipx'])('uses a clean name and distinct group for %s', name => {
    expect(archiveGroupKey(`F:/下载/${name}`)).toBe('f:/下载/资料.zipx')
    expect(archiveSourceFormat(name)).toBe('zipx')
    expect(archiveSourceStem(name)).toBe('资料')
    expect(archiveGroupKey(name)).not.toBe(archiveGroupKey('资料.zip'))
    expect(archiveGroupKey(name)).not.toBe(archiveGroupKey('资料.zipx.001'))
    if (!name.endsWith('.zipx')) expect(isPotentialSplitArchivePath(name)).toBe(true)
  })
})

describe('HaoZip forum ZIP sources', () => {
  it('normalizes any member without merging a same-basename ordinary ZIP', () => {
    const first = 'F:\\下载\\资料.haozip01.zip'
    const last = 'F:\\下载\\资料.HAOZIP03.ZIP'
    expect(archiveGroupKey(first)).toBe(archiveGroupKey(last))
    expect(archiveGroupKey(first)).not.toBe(archiveGroupKey('F:\\下载\\资料.zip'))
    expect(archiveSourceFormat(last)).toBe('zip')
    expect(archiveSourceStem(last)).toBe('资料')
    expect(archiveSourceStem('F:\\下载\\haozip02.zip')).toBe('archive')
    expect(isPotentialSplitArchivePath(last)).toBe(true)
    expect(isPotentialSplitArchivePath('F:\\下载\\haozip02.zip')).toBe(true)
    expect(isPotentialSplitArchivePath('F:\\下载\\not-haozip02.zip')).toBe(false)
  })
})

describe('HaoZip forum 7Z source names', () => {
  it.each([
    ['旅行.haozip01.7z', '旅行'],
    ['旅行.HAOZIP02.7Z', '旅行'],
    ['旅行.haozip003.7z', '旅行'],
    ['haozip01.7z', 'archive'],
    ['旅行-not-haozip02.7z', '旅行-not-haozip02'],
    ['旅行.haozip1.7z', '旅行.haozip1'],
    ['旅行.haozip02.zip', '旅行'],
  ])('keeps the same logical name as the backend for %s', (source, expected) => {
    expect(archiveSourceStem(`downloads/${source}`)).toBe(expected)
  })

  it('groups members without conflating ordinary 7Z or a forum ZIP with the same stem', () => {
    const first = 'F:\\下载\\资料.haozip01.7z'
    const middle = 'F:/下载/资料.HAOZIP002.7Z'
    expect(archiveGroupKey(middle)).toBe(archiveGroupKey(first))
    expect(archiveGroupKey(first)).not.toBe(archiveGroupKey('F:/下载/资料.7z'))
    expect(archiveGroupKey(first)).not.toBe(archiveGroupKey('F:/下载/资料.haozip01.zip'))
    expect(archiveSourceFormat(middle)).toBe('7z')
    expect(archiveSourceStem(middle)).toBe('资料')
    expect(archiveSourceStem('F:/下载/HAOZIP02.7Z')).toBe('archive')
    expect(isPotentialSplitArchivePath(first)).toBe(true)
    expect(isPotentialSplitArchivePath('F:/下载/HAOZIP02.7Z')).toBe(true)
    expect(isPotentialSplitArchivePath('F:/下载/not-haozip02.7z')).toBe(false)
  })
})
