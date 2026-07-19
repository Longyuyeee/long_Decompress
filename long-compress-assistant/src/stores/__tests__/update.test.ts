import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useUpdateStore } from '../update'

const mocks = vi.hoisted(() => ({
  checkUpdate: vi.fn(),
  installUpdate: vi.fn(),
  onUpdaterEvent: vi.fn(),
}))

vi.mock('@tauri-apps/api/updater', () => ({
  checkUpdate: mocks.checkUpdate,
  installUpdate: mocks.installUpdate,
  onUpdaterEvent: mocks.onUpdaterEvent,
}))

const manifest = {
  version: '1.0.8',
  date: '2026-07-20T00:00:00Z',
  body: 'Signed updater bootstrap',
}

describe('update store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    vi.clearAllMocks()
    Object.defineProperty(window, '__TAURI_IPC__', {
      value: vi.fn(),
      configurable: true,
    })
    mocks.onUpdaterEvent.mockResolvedValue(vi.fn())
    mocks.installUpdate.mockResolvedValue(undefined)
  })

  it('shows a signed update returned by the updater endpoint', async () => {
    mocks.checkUpdate.mockResolvedValue({ shouldUpdate: true, manifest })
    const store = useUpdateStore()

    await store.checkForUpdates(true)

    expect(store.status).toBe('available')
    expect(store.availableVersion).toBe('1.0.8')
    expect(store.dialogVisible).toBe(true)
  })

  it('respects a skipped version during automatic checks but not manual checks', async () => {
    mocks.checkUpdate.mockResolvedValue({ shouldUpdate: true, manifest })
    const store = useUpdateStore()

    await store.checkForUpdates(true)
    store.skipCurrentVersion()
    await store.checkForUpdates(false)
    expect(store.dialogVisible).toBe(false)

    await store.checkForUpdates(true)
    expect(store.dialogVisible).toBe(true)
  })

  it('blocks installation while archive tasks are active', async () => {
    mocks.checkUpdate.mockResolvedValue({ shouldUpdate: true, manifest })
    const store = useUpdateStore()
    await store.checkForUpdates(true)

    await store.install(2)

    expect(store.status).toBe('error')
    expect(store.errorMessage).toContain('2 个任务')
    expect(mocks.installUpdate).not.toHaveBeenCalled()
  })

  it('hands the verified update to Tauri when no task is active', async () => {
    mocks.checkUpdate.mockResolvedValue({ shouldUpdate: true, manifest })
    const store = useUpdateStore()
    await store.checkForUpdates(true)

    await store.install(0)

    expect(mocks.installUpdate).toHaveBeenCalledOnce()
    expect(store.status).toBe('installing')
  })

  it('keeps the current version usable when update checks fail', async () => {
    mocks.checkUpdate.mockRejectedValue(new Error('network unavailable'))
    const store = useUpdateStore()

    await store.checkForUpdates(true)

    expect(store.status).toBe('error')
    expect(store.errorMessage).toContain('network unavailable')
  })
})
