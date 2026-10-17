import { describe, expect, it } from 'vitest'
import { supportsZipPasswordEncoding, zipPasswordEncodingForSource } from '../zipPasswordSource'

describe('ZIP password input scope', () => {
  it.each(['a.zip', 'a.zipx', 'a.zip.001', 'a.z01', 'a.haozip01.zip'])('keeps the explicit input choice for %s', path => {
    expect(zipPasswordEncodingForSource(path, 'cp932')).toBe('cp932')
  })
  it('uses actual content over an extension and allows an unknown restored input', () => {
    expect(zipPasswordEncodingForSource('renamed.7z', 'cp950', 'ZIP')).toBe('cp950')
    expect(zipPasswordEncodingForSource('renamed.zip', 'cp950', '7Z')).toBeUndefined()
    expect(zipPasswordEncodingForSource('data.rar.001', 'cp936')).toBeUndefined()
    expect(supportsZipPasswordEncoding('archive.exe', 'RAR')).toBe(false)
    expect(supportsZipPasswordEncoding('archive.exe', 'ZIP')).toBe(true)
    expect(supportsZipPasswordEncoding('unknown')).toBe(true)
    expect(zipPasswordEncodingForSource('a.zip')).toBeUndefined()
  })
})
