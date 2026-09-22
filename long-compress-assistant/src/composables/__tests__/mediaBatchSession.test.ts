import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { usePdfOptimizationBatch, type PdfBatchSource } from '../usePdfOptimizationBatch'
import { useVideoCompressionBatch, type VideoBatchSource } from '../useVideoCompressionBatch'
import { useTaskStore } from '@/stores/task'
import { useMediaBatchSession } from '../useMediaBatchSession'
import { usePdfWorkspaceStore } from '@/stores/pdfWorkspace'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/dialog', () => ({ open: vi.fn(), save: vi.fn(), message: vi.fn(), ask: vi.fn() }))

const source = { id: 'one', name: 'one.pdf', path: 'C:/one.pdf', mode: 'lossless-organization', confirmedLossyImageChanges: false, allowLargerOutput: false } satisfies PdfBatchSource
const video = { id: 'one', name: 'one.mp4', path: 'C:/one.mp4', settings: {}, plan: { streamChanges: [] } } as VideoBatchSource
const cases = [
  { kind: 'pdf', plan: 'plan_pdf_optimization_destination', compress: 'compress_pdf_file', open: () => {
    const batch = usePdfOptimizationBatch()
    return { run: () => batch.runPdfBatch([source, { ...source, id: 'two' }], null, true), cancel: batch.cancelPdfBatch }
  } },
  { kind: 'video', plan: 'plan_video_compression_destination', compress: 'compress_video_file', open: () => {
    const batch = useVideoCompressionBatch()
    return { run: () => batch.runVideoBatch([video, { ...video, id: 'two' }], null, true), cancel: batch.cancelVideoBatch }
  } },
]

describe.each(cases)('$kind：离开页面再回来仍控制原批次', ({ open, plan, compress, kind }) => {
  beforeEach(() => { setActivePinia(createPinia()); mocks.invoke.mockReset() })

  it('我还在等输出路径规划，回来不能再次启动，并且取消后不应开始编码', async () => {
    let resolvePlan!: (value: unknown) => void
    mocks.invoke.mockImplementation((command: string) => command === plan
      ? new Promise(resolve => { resolvePlan = resolve }) : Promise.resolve())
    const first = open()
    const pending = first.run()
    await vi.waitFor(() => expect(resolvePlan).toBeTypeOf('function'))
    const reopened = open()
    await expect(reopened.run()).rejects.toThrow('正在运行')
    await reopened.cancel()
    resolvePlan({ destination: 'C:/out/result' })
    const results = await pending
    expect(results.map(result => result.status)).toEqual(['cancelled'])
    expect(mocks.invoke.mock.calls.filter(([command]) => command === compress)).toHaveLength(0)
    expect(useTaskStore().tasks).toHaveLength(1)
  })

  it('我切页回来取消，必须取消正在编码的原任务，且不要启动下一项', async () => {
    let rejectCompression!: (reason: unknown) => void
    mocks.invoke.mockImplementation((command: string) => {
      if (command === plan) return Promise.resolve({ destination: 'C:/out/result' })
      if (command === compress) return new Promise((_resolve, reject) => { rejectCompression = reject })
      if (command === 'cancel_compression') rejectCompression(new Error('CANCELLED'))
      return Promise.resolve()
    })
    const pending = open().run()
    await vi.waitFor(() => expect(rejectCompression).toBeTypeOf('function'))
    const reopened = open()
    await reopened.cancel()
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'cancel_compression')).toHaveLength(1)
    const results = await pending
    expect(results.map(result => result.status)).toEqual(['cancelled'])
    expect(useTaskStore().tasks).toHaveLength(1)
    expect(useTaskStore().tasks[0]).toMatchObject({ status: 'cancelled', workloadKind: kind })
    expect(useMediaBatchSession(kind as 'pdf' | 'video').isRunning.value).toBe(false)
  })

  it('取消后的历史还在保存时，不允许新批次覆盖控制器；保存结束后可以重试', async () => {
    let finishHistory!: (saved: boolean) => void
    vi.spyOn(useTaskStore(), 'waitForHistoryPersistence').mockImplementation(() => new Promise(resolve => { finishHistory = resolve }))
    mocks.invoke.mockImplementation((command: string) => {
      if (command === plan) return Promise.reject(new Error('destination unavailable'))
      return Promise.resolve()
    })
    const pending = open().run()
    await vi.waitFor(() => expect(finishHistory).toBeTypeOf('function'))
    await expect(open().run()).rejects.toThrow('正在运行')
    await open().cancel()
    finishHistory(false)
    await expect(pending).rejects.toThrow('历史未能持久化')
    expect(useMediaBatchSession(kind as 'pdf' | 'video').isRunning.value).toBe(false)
  })
})

it('不同工作区和应用实例互不取消，也不共享 PDF 草稿', () => {
  setActivePinia(createPinia())
  const pdf = useMediaBatchSession('pdf')
  pdf.isRunning.value = true
  expect(useMediaBatchSession('video').isRunning.value).toBe(false)
  const workspace = usePdfWorkspaceStore()
  workspace.outputDirectory = 'C:/chosen'
  expect(usePdfWorkspaceStore().outputDirectory).toBe('C:/chosen')
  setActivePinia(createPinia())
  expect(useMediaBatchSession('pdf').isRunning.value).toBe(false)
  expect(usePdfWorkspaceStore().outputDirectory).toBe('')
})
