import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useUpdateStore } from '../update'

const mocks = vi.hoisted(() => ({
  checkUpdate: vi.fn(),
  installUpdate: vi.fn(),
  onUpdaterEvent: vi.fn(),
  listen: vi.fn(),
}))

vi.mock('@tauri-apps/api/updater', () => ({
  checkUpdate: mocks.checkUpdate,
  installUpdate: mocks.installUpdate,
  onUpdaterEvent: mocks.onUpdaterEvent,
}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:(command:string,args?:unknown)=>command==='check_app_update'?mocks.checkUpdate():command==='install_app_update'?mocks.installUpdate(args):Promise.reject(Error('unexpected command'))}))
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen, TauriEvent: { DOWNLOAD_PROGRESS: 'tauri://update-download-progress' } }))

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
    mocks.listen.mockResolvedValue(vi.fn())
    mocks.installUpdate.mockResolvedValue(undefined)
  })

  afterEach(() => {
    vi.useRealTimers()
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
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-07-30T00:00:00Z'))
    mocks.checkUpdate.mockRejectedValue(new Error('network unavailable'))
    const store = useUpdateStore()

    await store.checkForUpdates(true)

    expect(store.status).toBe('error')
    expect(store.errorMessage).toContain('network unavailable')
    expect(store.lastAttemptAt).toBe(Date.now())
    expect(store.lastSuccessAt).toBeNull()
    expect(localStorage.getItem('updater-last-attempt-at')).toBe(String(Date.now()))
    expect(localStorage.getItem('updater-last-success-at')).toBeNull()
  })

  it('records successful checks separately from attempts', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-07-30T01:00:00Z'))
    mocks.checkUpdate.mockResolvedValue({ shouldUpdate: false })
    const store = useUpdateStore()

    await store.checkForUpdates(true)

    expect(store.status).toBe('up-to-date')
    expect(store.lastAttemptAt).toBe(Date.now())
    expect(store.lastSuccessAt).toBe(Date.now())
    expect(localStorage.getItem('updater-last-success-at')).toBe(String(Date.now()))
  })

  it('retries automatically when the previous attempt failed', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-07-30T02:00:00Z'))
    localStorage.setItem('updater-last-attempt-at', String(Date.now() - 1_000))
    mocks.checkUpdate.mockRejectedValue(new Error('temporary network failure'))
    const store = useUpdateStore()

    store.scheduleAutoCheck(true)
    await vi.advanceTimersByTimeAsync(2_500)

    expect(mocks.checkUpdate).toHaveBeenCalledOnce()
    expect(store.status).toBe('error')
    expect(store.lastSuccessAt).toBeNull()
  })

  it('does not repeat automatic checks within a day of success', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-07-30T03:00:00Z'))
    localStorage.setItem('updater-last-success-at', String(Date.now() - 1_000))
    const store = useUpdateStore()

    store.scheduleAutoCheck(true)
    await vi.advanceTimersByTimeAsync(3_000)

    expect(mocks.checkUpdate).not.toHaveBeenCalled()
  })

  it('registers progress before installation, accumulates chunks and uses the native downloaded phase', async () => {
    const store = useUpdateStore()
    store.manifest = manifest
    mocks.installUpdate.mockImplementation(async () => {
      const progress = mocks.listen.mock.calls[0][1]
      progress({ payload: { chunkLength: 25, contentLength: 100 } })
      progress({ payload: { chunkLength: 10, contentLength: 100 } })
      expect(store.downloadedBytes).toBe(35)
      expect(store.downloadPercent).toBe(35)
      expect(store.installPhase).toBe('downloading')
      mocks.onUpdaterEvent.mock.calls[0][0]({ status: 'DOWNLOADED' })
      expect(store.installPhase).toBe('installing')
      progress({ payload: { chunkLength: 99, contentLength: 100 } })
      expect(store.downloadedBytes).toBe(35)
    })
    await store.install(0)
    expect(store.installPhase).toBe('restarting')
    expect(mocks.listen.mock.invocationCallOrder[0]).toBeLessThan(mocks.installUpdate.mock.invocationCallOrder[0])
  })

  it('shows indeterminate progress without a content length and resets after a failed attempt', async () => {
    const store = useUpdateStore()
    store.manifest = manifest
    mocks.installUpdate.mockImplementationOnce(async () => {
      mocks.listen.mock.calls[0][1]({ payload: { chunkLength: 50, contentLength: null } })
      expect(store.downloadPercent).toBeNull()
      expect(store.downloadedBytes).toBe(50)
      store.remindLater()
      expect(store.dialogVisible).toBe(true)
      throw new Error('download interrupted')
    })
    await store.install(0)
    expect(store.status).toBe('error')
    mocks.installUpdate.mockImplementationOnce(async () => {
      expect(store.downloadedBytes).toBe(0)
      expect(store.totalBytes).toBeNull()
      expect(store.installPhase).toBe('preparing')
      mocks.listen.mock.calls[0][1]({ payload: { chunkLength: 120, contentLength: 100 } })
      expect(store.downloadPercent).toBeNull()
      expect(store.totalBytes).toBeNull()
      expect(store.downloadedBytes).toBe(120)
    })
    await store.install(0)
    expect(mocks.listen).toHaveBeenCalledOnce()
  })

  it('shares initialization and removes both listeners on cleanup', async () => {
    const stopStatus = vi.fn(), stopProgress = vi.fn()
    mocks.onUpdaterEvent.mockResolvedValue(stopStatus)
    mocks.listen.mockResolvedValue(stopProgress)
    const store = useUpdateStore()
    await Promise.all([store.initialize(), store.initialize()])
    expect(mocks.listen).toHaveBeenCalledOnce()
    store.cleanup()
    expect(stopStatus).toHaveBeenCalledOnce()
    expect(stopProgress).toHaveBeenCalledOnce()
  })
  it('keeps inconsistent content length indeterminate across later chunks instead of showing excess bytes over total', async () => {
    const store=useUpdateStore();store.manifest=manifest
    mocks.installUpdate.mockImplementation(async()=>{
      const progress=mocks.listen.mock.calls[0][1]
      progress({payload:{chunkLength:60,contentLength:100}})
      expect(store.downloadPercent).toBe(60)
      progress({payload:{chunkLength:60,contentLength:100}})
      expect(store.downloadedBytes).toBe(120);expect(store.totalBytes).toBeNull();expect(store.downloadPercent).toBeNull()
      progress({payload:{chunkLength:5,contentLength:100}})
      expect(store.downloadedBytes).toBe(125);expect(store.totalBytes).toBeNull()
      for(const chunkLength of [-1,NaN,Infinity,1.5])progress({payload:{chunkLength,contentLength:null}})
      expect(store.downloadedBytes).toBe(125)
    })
    await store.install(0)
  })
  it('ignores a second install even if its caller reports a running task', async () => {
    const store=useUpdateStore();store.manifest=manifest
    mocks.installUpdate.mockImplementation(async()=>{await store.install(1);expect(store.status).toBe('installing')})
    await store.install(0);expect(mocks.installUpdate).toHaveBeenCalledOnce();expect(mocks.installUpdate).toHaveBeenCalledWith({expectedVersion:manifest.version})
  })
  it('ignores callbacks belonging to disposed listeners', async () => {
    const store=useUpdateStore();await store.initialize();const previous=mocks.listen.mock.calls[0][1];store.cleanup()
    store.$patch({status:'installing',installPhase:'downloading',downloadedBytes:0,totalBytes:100})
    previous({payload:{chunkLength:10,contentLength:100}});expect(store.downloadedBytes).toBe(0)
    mocks.onUpdaterEvent.mock.calls[0][0]({status:'ERROR',error:'stale event'});expect(store.status).toBe('installing')
  })

})
