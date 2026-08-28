import { readFileSync, statSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it, vi } from 'vitest'
import {
  ImageCompressionBatchRunner,
  createDefaultImageSettings,
  createImageCompressionRequest,
  type ImageBatchCommands,
  type ImageBatchSource,
  type ImageCompressionFacts,
} from '../imageCompressionWorkspace'

const fixtureRoot = resolve(process.cwd(), 'test-results/media-fixture-audit/fixtures/images')
const baseline = JSON.parse(readFileSync(resolve(process.cwd(), 'tests/fixtures/media/image-baseline.json'), 'utf8'))

const facts = (format: 'jpeg' | 'png' | 'webp', encodedBytes: number): ImageCompressionFacts => ({
  format,
  encodedBytes,
  encodedWidth: 10,
  encodedHeight: 10,
  visibleWidth: 10,
  visibleHeight: 10,
  orientation: 1,
  frameCount: 1,
  hasAlpha: format === 'png',
})

describe('image compression orchestration', () => {
  it('maps real JPEG and PNG sources into truthful backend requests and count-based progress', async () => {
    const jpegPath = resolve(fixtureRoot, 'exif-orientation.jpg')
    const pngPath = resolve(fixtureRoot, 'transparent.png')
    expect(statSync(jpegPath).size).toBe(baseline.inputs.find((item: { file: string }) => item.file === 'exif-orientation.jpg').bytes)
    expect(statSync(pngPath).size).toBe(baseline.inputs.find((item: { file: string }) => item.file === 'transparent.png').bytes)

    const jpegSettings = {
      ...createDefaultImageSettings(),
      outputFormat: 'webp' as const,
      resizeMode: 'limit' as const,
      maxWidth: 320,
      maxHeight: 180,
      outputDirectory: 'C:/真实输出',
      conflictPolicy: 'rename' as const,
    }
    const pngSettings = {
      ...createDefaultImageSettings(),
      mode: 'lossless' as const,
      outputFormat: 'keep' as const,
      conflictPolicy: 'skip' as const,
    }
    const items: ImageBatchSource[] = [
      { id: 'oriented', name: 'exif-orientation.jpg', path: jpegPath, inputFormat: 'jpeg', settings: jpegSettings },
      { id: 'alpha', name: 'transparent.png', path: pngPath, inputFormat: 'png', settings: pngSettings },
    ]
    const plans: ImageBatchCommands['planDestination'] = vi.fn(async request => request.source === jpegPath
      ? { status: 'ready', destination: 'C:/真实输出/exif-orientation.compressed.webp' }
      : { status: 'skipped', destination: `${pngPath}.compressed.png`, reason: '真实目标已存在' })
    const compress: ImageBatchCommands['compress'] = vi.fn(async (_taskId, request) => ({
      status: 'published',
      input: facts('jpeg', statSync(jpegPath).size),
      output: facts('webp', 1200),
    }))
    const progress: number[] = []
    const runner = new ImageCompressionBatchRunner(
      { planDestination: plans, compress, cancel: vi.fn() },
      (item, index) => `real-image-task-${index}-${item.id}`,
    )

    const results = await runner.run(items, value => progress.push(value.percentage))

    expect(progress).toEqual([50, 100])
    expect(results.map(result => result.status)).toEqual(['published', 'skipped'])
    expect(plans).toHaveBeenNthCalledWith(2, expect.objectContaining({
      targetFormat: 'png',
      conflictPolicy: 'skip',
      reservedDestinations: ['C:/真实输出/exif-orientation.compressed.webp'],
    }))
    expect(compress).toHaveBeenCalledWith('real-image-task-0-oriented', {
      source: jpegPath,
      destination: 'C:/真实输出/exif-orientation.compressed.webp',
      mode: 'lossy',
      quality: 82,
      targetFormat: 'webp',
      maxDimensions: { width: 320, height: 180 },
      preserveMetadata: true,
      onlyIfSmaller: false,
    })
  })

  it('maps replace-if-smaller to the audited size policy without inventing a resize', () => {
    const request = createImageCompressionRequest(
      'C:/输入/photo.webp',
      'C:/输出/photo.compressed.webp',
      'webp',
      createDefaultImageSettings(),
    )

    expect(request).toMatchObject({
      targetFormat: 'webp',
      maxDimensions: null,
      onlyIfSmaller: true,
    })
  })

  it('cancels the active unique task and truthfully settles untouched items as cancelled', async () => {
    let rejectActive!: (reason: unknown) => void
    const compress = vi.fn(() => new Promise<never>((_resolve, reject) => { rejectActive = reject }))
    const cancel = vi.fn(async () => rejectActive(new Error('image compression was cancelled')))
    const settings = { ...createDefaultImageSettings(), conflictPolicy: 'rename' as const }
    const items: ImageBatchSource[] = ['one', 'two', 'three'].map(name => ({
      id: name,
      name: `${name}.webp`,
      path: resolve(fixtureRoot, 'photo.webp'),
      inputFormat: 'webp',
      settings,
    }))
    const runner = new ImageCompressionBatchRunner(
      {
        planDestination: async request => ({ status: 'ready', destination: `${request.source}.${request.reservedDestinations.length}.webp` }),
        compress,
        cancel,
      },
      (item, index) => `cancel-batch-${index}-${item.id}`,
    )

    const pending = runner.run(items)
    await vi.waitFor(() => expect(compress).toHaveBeenCalledTimes(1))
    await runner.cancel()
    const results = await pending

    expect(cancel).toHaveBeenCalledWith('cancel-batch-0-one')
    expect(results).toEqual([
      { status: 'cancelled', itemId: 'one', taskId: 'cancel-batch-0-one' },
      { status: 'cancelled', itemId: 'two', taskId: 'cancel-batch-1-two' },
      { status: 'cancelled', itemId: 'three', taskId: 'cancel-batch-2-three' },
    ])
  })

  it('does not start encoding when cancellation arrives while the destination is being planned', async () => {
    let finishPlan!: () => void
    const planDestination = vi.fn(() => new Promise<{ status: 'ready', destination: string }>(resolvePlan => {
      finishPlan = () => resolvePlan({ status: 'ready', destination: 'C:/output/photo.compressed.webp' })
    }))
    const compress = vi.fn()
    const runner = new ImageCompressionBatchRunner(
      { planDestination, compress, cancel: vi.fn() },
      () => 'planning-task',
    )
    const pending = runner.run([{
      id: 'photo',
      name: 'photo.webp',
      path: resolve(fixtureRoot, 'photo.webp'),
      inputFormat: 'webp',
      settings: createDefaultImageSettings(),
    }])
    await vi.waitFor(() => expect(planDestination).toHaveBeenCalledTimes(1))

    await runner.cancel()
    finishPlan()

    await expect(pending).resolves.toEqual([
      { status: 'cancelled', itemId: 'photo', taskId: 'planning-task' },
    ])
    expect(compress).not.toHaveBeenCalled()
  })
})
