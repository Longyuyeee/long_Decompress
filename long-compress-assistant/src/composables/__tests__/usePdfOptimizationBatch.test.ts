import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { usePdfOptimizationBatch } from '../usePdfOptimizationBatch'
import { useTaskStore } from '@/stores/task'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  history: [] as unknown[],
}))

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/dialog', () => ({
  open: vi.fn(), save: vi.fn(), message: vi.fn(), ask: vi.fn(),
}))

const publishedPdf = (path: string) => ({
  path, inputBytes: 8_589, outputBytes: 7_000, savingsRatio: 0.185,
  outputSha256: 'a'.repeat(64), markOfTheWeb: 'not-present',
  verified: {
    outputBytes: 7_000, outputSha256: 'a'.repeat(64),
    sourceFacts: { pageCount: 1, encrypted: false, pageMediaBoxes: [], formFields: [], annotations: [], outlines: [], attachments: [] },
    outputFacts: { pageCount: 1, encrypted: false, pageMediaBoxes: [], formFields: [], annotations: [], outlines: [], attachments: [] },
  },
})

describe('usePdfOptimizationBatch', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    mocks.history = []
    mocks.invoke.mockReset()
  })

  it('persists verified PDF facts and a real task failure through the unified history route', async () => {
    mocks.invoke.mockImplementation(async (command: string, payload: any) => {
      if (command === 'plan_pdf_optimization_destination') {
        if (payload.source.endsWith('blocked.pdf')) throw new Error('PDF_DESTINATION_DIRECTORY_UNAVAILABLE')
        return { destination: 'C:/output/form.organized.pdf' }
      }
      if (command === 'compress_pdf_file') return publishedPdf(payload.request.destination)
      if (command === 'save_task_history') mocks.history.push(payload.record)
      return undefined
    })
    const batch = usePdfOptimizationBatch()
    const results = await batch.runPdfBatch([
      { id: 'form', name: 'form.pdf', path: 'C:/input/form.pdf', mode: 'lossless-organization', confirmedLossyImageChanges: false, allowLargerOutput: false },
      { id: 'blocked', name: 'blocked.pdf', path: 'C:/input/blocked.pdf', mode: 'lossless-organization', confirmedLossyImageChanges: false, allowLargerOutput: false },
    ], null, true)

    expect(results.map(result => result.status)).toEqual(['published', 'failed'])
    expect(useTaskStore().tasks.map(task => task.status)).toEqual(['completed', 'failed'])
    expect(useTaskStore().tasks[0].metrics).toEqual(expect.objectContaining({
      inputBytes: 8_589, outputBytes: 7_000, media: { pageCount: 1 },
    }))
    expect(mocks.history).toEqual(expect.arrayContaining([
      expect.objectContaining({ status: 'completed', workloadKind: 'pdf' }),
      expect.objectContaining({ status: 'failed', workloadKind: 'pdf', errorMessage: expect.stringContaining('DIRECTORY_UNAVAILABLE') }),
    ]))
  })

  it('waits for backend cancellation cleanup and persists no invented metrics', async () => {
    let rejectCompression!: (reason: unknown) => void
    mocks.invoke.mockImplementation((command: string, payload: any) => {
      if (command === 'plan_pdf_optimization_destination') {
        return Promise.resolve({ destination: 'C:/output/large.organized.pdf' })
      }
      if (command === 'compress_pdf_file') {
        return new Promise((_resolve, reject) => { rejectCompression = reject })
      }
      if (command === 'cancel_compression') {
        rejectCompression(new Error('PDF_PUBLISH_CANCELLED'))
        return Promise.resolve(undefined)
      }
      if (command === 'save_task_history') {
        mocks.history.push(payload.record)
        return Promise.resolve(undefined)
      }
      return Promise.resolve(undefined)
    })
    const batch = usePdfOptimizationBatch()
    const pending = batch.runPdfBatch([
      { id: 'large', name: 'large.pdf', path: 'C:/input/large.pdf', mode: 'lossless-organization', confirmedLossyImageChanges: false, allowLargerOutput: false },
    ], null, true)
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('compress_pdf_file', expect.anything()))

    await batch.cancelPdfBatch()
    const results = await pending

    expect(results[0].status).toBe('cancelled')
    expect(useTaskStore().tasks[0]).toMatchObject({ status: 'cancelled', workloadKind: 'pdf' })
    expect(useTaskStore().tasks[0].metrics).toBeUndefined()
    expect(mocks.history).toEqual([
      expect.objectContaining({ status: 'cancelled', workloadKind: 'pdf', metrics: null }),
    ])
  })
})
