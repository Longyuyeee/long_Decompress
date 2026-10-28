import { describe, expect, it } from 'vitest'
import { copyExecutionConfig, type ExecutionConfigV1 } from '../executionConfig'
import { createDefaultImageSettings } from '@/utils/imageCompressionWorkspace'
import { createDefaultVideoSettings } from '@/utils/videoCompressionWorkspace'
import { ZIP_PASSWORD_ENCODINGS, isZipPasswordEncoding } from '../zipPassword'

describe('actual execution snapshots', () => {
  it('selected1720 copies the bounded TAR selection without mutating history or accepting empty scope', () => {
    const config = {schemaVersion:1 as const,kind:'archive-extraction' as const,outputDirectory:'D:/out',extractToSubfolder:false,tarSelection:{paths:['alias.txt'],keepStructure:false,conflictPolicy:'skip' as const}}
    const copy = copyExecutionConfig(config)
    expect(copy).toEqual(config)
    config.tarSelection.paths.push('later.txt')
    expect(copy).toMatchObject({tarSelection:{paths:['alias.txt']}})
    expect(copyExecutionConfig({...config,tarSelection:{...config.tarSelection,paths:[]}})).toBeNull()
  })
  it.each(ZIP_PASSWORD_ENCODINGS)('keeps %s input separately from output encoding and without secrets', encoding => {
    const input: ExecutionConfigV1 = { schemaVersion: 1, kind: 'archive-extraction',
      outputDirectory: 'D:/out', extractToSubfolder: true, zipPasswordEncoding: encoding }
    expect(copyExecutionConfig(input)).toEqual(input)
    const conversion: ExecutionConfigV1 = { schemaVersion: 1, kind: 'archive-conversion',
      sourceZipPasswordEncoding: encoding, zipPasswordEncoding: 'utf8', format: 'zip', level: 6,
      outputPath: 'D:/new.zip', splitSize: 0, encryptFilenames: false, passwordRequired: true }
    expect(copyExecutionConfig(conversion)).toMatchObject({ sourceZipPasswordEncoding: encoding, zipPasswordEncoding: 'utf8' })
    expect(isZipPasswordEncoding(encoding)).toBe(true)
    for (const invalid of ['unknown', 'CP932', 932, true]) {
      expect(isZipPasswordEncoding(invalid)).toBe(false)
      expect(copyExecutionConfig({ ...input, zipPasswordEncoding: invalid } as unknown as ExecutionConfigV1)).toBeNull()
      expect(copyExecutionConfig({ ...conversion, sourceZipPasswordEncoding: invalid } as unknown as ExecutionConfigV1)).toBeNull()
    }
  })
  it('retains the explicit ZIP method when copying a conversion history', () => {
    const config = { schemaVersion: 1 as const, kind: 'archive-conversion' as const, format: 'zip', level: 6,
      outputPath: 'D:/new.zip', splitSize: 1048576, zipMethod: 'zstd' as const, encryptFilenames: false, passwordRequired: true }
    expect(copyExecutionConfig(config)).toMatchObject({ zipMethod: 'zstd', format: 'zip', splitSize: 1048576 })
  })
  const configs: ExecutionConfigV1[] = [
    { schemaVersion: 1, kind: 'archive-compression', outputDirectory: 'D:/打包', passwordRequired: true,
      settings: { format: 'zip', level: 8, filename: '照片', splitArchive: false, splitSize: '100', keepStructure: true,
        verifyAfter: true, createSolidArchive: false, fastPack: false, sevenZipFilter: 'none', sevenZipDeltaDistance: 1, zipMethod: 'zstd', zipPasswordEncoding: 'windows_local', zipSplitMode: 'standard' } },
    { schemaVersion: 1, kind: 'image-compression', settings: createDefaultImageSettings() },
    { schemaVersion: 1, kind: 'video-compression', outputDirectory: 'D:/视频', settings: createDefaultVideoSettings() },
    { schemaVersion: 1, kind: 'pdf-optimization', outputDirectory: '', mode: 'compatible-image-optimization', allowLargerOutput: true },
  ]
  it.each(configs)('copies $kind without later global mutations or transient secrets', (config) => {
    const polluted = { ...config, password: 'never-save', deleteAfter: true, confirmedLossyImageChanges: true,
      ...('settings' in config ? { settings: { ...config.settings, password: 'never-save', deleteAfter: true } } : {}) }
    const copy = copyExecutionConfig(polluted as ExecutionConfigV1)
    expect(copy).toEqual(config)
    expect(JSON.stringify(copy)).not.toContain('never-save')
    expect(JSON.stringify(copy)).not.toContain('deleteAfter')
    expect(JSON.stringify(copy)).not.toContain('confirmedLossyImageChanges')
    if ('settings' in polluted) {
      Object.assign(polluted.settings, { quality: 1, level: 1 })
      expect(copy).toEqual(config)
    }
  })
  it('disables retired fast packing without rewriting the original history', () => {
    const original = { ...configs[0], settings: { ...(configs[0] as any).settings, fastPack: true } } as ExecutionConfigV1
    const restored = copyExecutionConfig(original)
    expect((restored as any).settings.fastPack).toBe(false)
    expect((original as any).settings.fastPack).toBe(true)
  })
  it('does not pretend unknown versions were restored', () => {
    expect(copyExecutionConfig({ ...configs[0], schemaVersion: 2 } as unknown as ExecutionConfigV1)).toBeNull()
    expect(copyExecutionConfig()).toBeNull()
  })
})
