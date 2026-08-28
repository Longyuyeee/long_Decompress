import { statSync } from 'node:fs'
import { resolve } from 'node:path'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useImageCompressionBatch } from '../useImageCompressionBatch'
import { useTaskStore } from '@/stores/task'
import { createDefaultImageSettings, type ImageCompressionFacts } from '@/utils/imageCompressionWorkspace'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  history: [] as unknown[],
}))

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/dialog', () => ({
  open: vi.fn(), save: vi.fn(), message: vi.fn(), ask: vi.fn(),
}))

const fixtureRoot = resolve(process.cwd(), 'test-results/media-fixture-audit/fixtures/images')
const pngPath = resolve(fixtureRoot, 'transparent.png')
const webpPath = resolve(fixtureRoot, 'photo.webp')

const facts = (
  format: 'jpeg' | 'png' | 'webp',
  encodedBytes: number,
  width: number,
  height: number,
  hasAlpha = false,
): ImageCompressionFacts => ({
  format,
  encodedBytes,
  encodedWidth: width,
  encodedHeight: height,
  visibleWidth: width,
  visibleHeight: height,
  orientation: 1,
  frameCount: 1,
  hasAlpha,
})

describe('useImageCompressionBatch', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    mocks.history = []
    mocks.invoke.mockReset()
    mocks.invoke.mockImplementation(async (command: string, payload: { record?: unknown }) => {
      if (command === 'save_task_history') {
        mocks.history.push(payload.record)
        return undefined
      }
      return undefined
    })
  })

  it('creates unified image tasks and persists real published facts plus a real failure', async () => {
    const inputBytes = statSync(pngPath).size
    const outputBytes = statSync(webpPath).size
    mocks.invoke.mockImplementation(async (command: string, payload: any) => {
      if (command === 'plan_image_compression_destination') {
        if (payload.source === webpPath) throw new Error('真实目标目录拒绝访问')
        return { status: 'ready', destination: 'C:/真实输出/transparent.compressed.png' }
      }
      if (command === 'compress_image_file') {
        return {
          status: 'published',
          input: facts('png', inputBytes, 256, 256, true),
          output: facts('png', outputBytes, 256, 256, true),
        }
      }
      if (command === 'save_task_history') {
        mocks.history.push(payload.record)
        return undefined
      }
      return undefined
    })
    const settings = { ...createDefaultImageSettings(), mode: 'lossless' as const, conflictPolicy: 'rename' as const }
    const batch = useImageCompressionBatch()

    const results = await batch.runImageBatch([
      { id: 'real-png', name: 'transparent.png', path: pngPath, inputFormat: 'png', settings },
      { id: 'real-failure', name: 'photo.webp', path: webpPath, inputFormat: 'webp', settings },
    ], undefined, 'history-batch')

    expect(results.map(result => result.status)).toEqual(['published', 'failed'])
    const tasks = useTaskStore().tasks
    expect(new Set(tasks.map(task => task.id)).size).toBe(2)
    expect(tasks.map(task => task.workloadKind)).toEqual(['image', 'image'])
    expect(tasks.map(task => task.status)).toEqual(['completed', 'failed'])
    expect(tasks[0].metrics).toEqual(expect.objectContaining({
      inputBytes,
      outputBytes,
      media: { image: expect.objectContaining({
        input: expect.objectContaining({ format: 'png', hasAlpha: true }),
        output: expect.objectContaining({ format: 'png', hasAlpha: true }),
      }) },
    }))
    expect(mocks.history).toHaveLength(2)
    expect(mocks.history).toEqual(expect.arrayContaining([
      expect.objectContaining({ status: 'completed', taskType: 'compression', workloadKind: 'image' }),
      expect.objectContaining({ status: 'failed', taskType: 'compression', workloadKind: 'image', errorMessage: expect.stringContaining('拒绝访问') }),
    ]))
    expect(JSON.stringify(mocks.history)).not.toContain('encodedBytes')
  })

  it('persists active and untouched items as cancelled without inventing image metrics', async () => {
    let rejectCompression!: (reason: unknown) => void
    mocks.invoke.mockImplementation((command: string, payload: any) => {
      if (command === 'plan_image_compression_destination') {
        return Promise.resolve({ status: 'ready', destination: `C:/真实输出/${payload.source.endsWith('photo.webp') ? 'photo' : 'transparent'}.compressed.webp` })
      }
      if (command === 'compress_image_file') {
        return new Promise((_resolve, reject) => { rejectCompression = reject })
      }
      if (command === 'cancel_compression') {
        rejectCompression(new Error('image compression was cancelled'))
        return Promise.resolve(undefined)
      }
      if (command === 'save_task_history') {
        mocks.history.push(payload.record)
        return Promise.resolve(undefined)
      }
      return Promise.resolve(undefined)
    })
    const settings = { ...createDefaultImageSettings(), outputFormat: 'webp' as const, conflictPolicy: 'rename' as const }
    const batch = useImageCompressionBatch()
    const pending = batch.runImageBatch([
      { id: 'active', name: 'photo.webp', path: webpPath, inputFormat: 'webp', settings },
      { id: 'untouched', name: 'transparent.png', path: pngPath, inputFormat: 'png', settings },
    ], undefined, 'cancel-history-batch')
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('compress_image_file', expect.anything()))

    await batch.cancelImageBatch()
    const results = await pending

    expect(results.map(result => result.status)).toEqual(['cancelled', 'cancelled'])
    expect(useTaskStore().tasks.map(task => task.status)).toEqual(['cancelled', 'cancelled'])
    expect(useTaskStore().tasks.every(task => task.metrics === undefined)).toBe(true)
    expect(mocks.history).toHaveLength(2)
    expect(mocks.history).toEqual(expect.arrayContaining([
      expect.objectContaining({ status: 'cancelled', workloadKind: 'image', metrics: null }),
    ]))
  })
})
