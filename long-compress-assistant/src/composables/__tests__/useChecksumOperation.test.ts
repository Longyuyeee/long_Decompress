import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { useChecksumOperation } from '../useChecksumOperation'
import type { ChecksumProgress } from '@/types/checksum'
const mocks = vi.hoisted(() => ({ listen: vi.fn(), invoke: vi.fn(), cleanup: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }))
vi.mock('@/composables/useTauriCommands', () => ({ useTauriCommands: () => ({ invoke: mocks.invoke }) }))
beforeEach(() => { vi.clearAllMocks(); mocks.listen.mockResolvedValue(mocks.cleanup) })
const setup = () => {
  let operation!: ReturnType<typeof useChecksumOperation>
  const wrapper = mount(defineComponent({ setup() { operation = useChecksumOperation(); return () => null } }))
  return { operation, wrapper }
}
const event = (id: string, bytes = 65536) => ({ payload: { operationId: id, currentFile: 'large.bin', fileBytes: bytes, fileSize: 200000, completedBytes: bytes, totalBytes: 200000, completedFiles: 0, totalFiles: 1 } as ChecksumProgress })

describe('checksum1522 operation lifecycle', () => {
  it('subscribes before native I/O, filters foreign events and stops this request only', async () => {
    const { operation, wrapper } = setup()
    let finish!: (value: string) => void
    mocks.invoke.mockImplementation((cmd: string) => cmd === 'calculate_checksum_batch' ? new Promise(resolve => { finish = resolve }) : Promise.resolve(true))
    const work = operation.run('calculate_checksum_batch', { paths: ['large.bin'] }); await flushPromises()
    const id = mocks.invoke.mock.calls[0][1].operationId, listener = mocks.listen.mock.calls[0][1]
    expect(mocks.listen.mock.invocationCallOrder[0]).toBeLessThan(mocks.invoke.mock.invocationCallOrder[0])
    listener(event('foreign')); expect(operation.progress.value).toBeNull()
    listener(event(id)); expect(operation.progress.value?.fileBytes).toBe(65536)
    await operation.stop(); expect(mocks.invoke).toHaveBeenLastCalledWith('cancel_checksum_operation', { operationId: id })
    finish('done'); await work; expect(mocks.cleanup).toHaveBeenCalledTimes(1)
    listener(event(id, 100000)); expect(operation.progress.value?.fileBytes).toBe(65536); wrapper.unmount()
  })
  it('leaving the workspace cancels active native reads and ignores late progress', async () => {
    const { operation, wrapper } = setup(); let finish!: (value: string) => void
    mocks.invoke.mockImplementation((cmd: string) => cmd === 'verify_checksum_file' ? new Promise(resolve => { finish = resolve }) : Promise.resolve(true))
    const work = operation.run('verify_checksum_file', {}); await flushPromises()
    const id = mocks.invoke.mock.calls[0][1].operationId
    wrapper.unmount(); expect(mocks.invoke).toHaveBeenLastCalledWith('cancel_checksum_operation', { operationId: id })
    mocks.listen.mock.calls[0][1](event(id)); expect(operation.progress.value).toBeNull()
    finish('done'); await work; expect(mocks.cleanup).toHaveBeenCalledTimes(1)
  })
  it('does not launch hashing if the progress subscription failed', async () => {
    const { operation, wrapper } = setup(); mocks.listen.mockRejectedValueOnce(Error('listener failed'))
    await expect(operation.run('verify_checksum_file', {})).rejects.toThrow('listener failed')
    expect(mocks.invoke).not.toHaveBeenCalled(); wrapper.unmount()
  })
})
