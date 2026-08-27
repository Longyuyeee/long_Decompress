import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { useCompressionStore } from '../compression'
import { estimateImageOutputRange } from '@/utils/imageCompressionWorkspace'

const manifest = JSON.parse(readFileSync(resolve(process.cwd(), 'tests/fixtures/media/manifest.json'), 'utf8'))
const baseline = JSON.parse(readFileSync(resolve(process.cwd(), 'tests/fixtures/media/image-baseline.json'), 'utf8'))
const fixtureCandidate = (file: string) => {
  const entry = baseline.inputs.find((candidate: { file: string }) => candidate.file === file)
  return { name: file, path: `C:/real-fixtures/${file}`, size: entry?.bytes || 2048, type: 'file', isDirectory: false }
}

describe('image compression workspace store', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('accepts the real JPEG, PNG and WebP fixtures while rejecting real GIF and PDF inputs', () => {
    const store = useCompressionStore()
    const candidates = manifest.images.map((entry: { file: string }) => fixtureCandidate(entry.file))
    candidates.push(fixtureCandidate('text-vector.pdf'))

    const result = store.addImageCandidates(candidates)

    expect(result.accepted.map(item => item.name)).toEqual([
      'transparent.png',
      'exif-orientation.jpg',
      'photo.webp',
      'ultra-large.png',
    ])
    expect(result.rejected).toEqual(expect.arrayContaining([
      expect.objectContaining({ name: 'animated.gif', reason: expect.stringContaining('GIF') }),
      expect.objectContaining({ name: 'text-vector.pdf', reason: expect.stringContaining('不是受支持') }),
    ]))
  })

  it('keeps archive and image queues independent and uses one settings model for global and item overrides', () => {
    const store = useCompressionStore()
    store.addFile({ name: 'notes.txt', path: 'C:/real/notes.txt', size: 10, type: 'file', isDirectory: false })
    const { accepted } = store.addImageCandidates([fixtureCandidate('photo.webp')])

    expect(store.selectedFiles).toHaveLength(1)
    expect(store.imageItems).toHaveLength(1)
    store.imageGlobalSettings.quality = 76
    expect(store.getEffectiveImageSettings(accepted[0]).quality).toBe(76)

    store.enableImageItemOverride(accepted[0].id)
    store.updateImageItemSettings(accepted[0].id, { ...store.getEffectiveImageSettings(accepted[0]), quality: 91 })
    expect(store.imageGlobalSettings.quality).toBe(76)
    expect(store.getEffectiveImageSettings(accepted[0]).quality).toBe(91)
  })

  it('marks estimates as a range derived from the effective settings instead of an actual output', () => {
    const store = useCompressionStore()
    const lossy = estimateImageOutputRange(1_000_000, store.imageGlobalSettings)
    store.imageGlobalSettings.mode = 'lossless'
    const lossless = estimateImageOutputRange(1_000_000, store.imageGlobalSettings)

    expect(lossy).toEqual(expect.objectContaining({ minimum: expect.any(Number), maximum: expect.any(Number) }))
    expect(lossy!.minimum).toBeLessThan(lossy!.maximum)
    expect(lossless!.maximum).toBeGreaterThanOrEqual(lossy!.maximum)
  })
})
