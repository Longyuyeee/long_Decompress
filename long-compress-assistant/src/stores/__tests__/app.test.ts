import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useAppStore } from '../app'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))

describe('App Store Explorer context menu synchronization', () => {
  let registered = false

  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
    registered = false
    mocks.invoke.mockReset()
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'is_context_menu_registered') return registered
      if (command === 'register_context_menu') registered = true
      if (command === 'unregister_context_menu') registered = false
      if (command === 'load_app_settings') return '{}'
      return undefined
    })
  })

  it('uses the persisted preference as startup source of truth', async () => {
    registered = true
    localStorage.setItem('app-settings', JSON.stringify({ contextMenuEnabled: false }))

    const store = useAppStore()
    await store.synchronizeContextMenu()

    expect(store.settings.contextMenuEnabled).toBe(false)
    expect(mocks.invoke).not.toHaveBeenCalledWith('register_context_menu')
    expect(mocks.invoke).toHaveBeenCalledWith('unregister_context_menu')
  })

  it('changes registration once and persists only after success', async () => {
    const store = useAppStore()
    await store.synchronizeContextMenu()
    mocks.invoke.mockClear()

    await store.setContextMenuEnabled(false)

    expect(store.settings.contextMenuEnabled).toBe(false)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'unregister_context_menu')).toHaveLength(1)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'register_context_menu')).toHaveLength(0)
  })

  it('keeps the saved preference when registration changes fail', async () => {
    const store = useAppStore()
    await store.synchronizeContextMenu()
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'is_context_menu_registered') return true
      if (command === 'unregister_context_menu') throw new Error('registry denied')
      return undefined
    })

    await expect(store.setContextMenuEnabled(false)).rejects.toThrow('registry denied')

    expect(store.settings.contextMenuEnabled).toBe(true)
  })

  it('enables Mark-of-the-Web propagation for new and existing settings', () => {
    localStorage.setItem('app-settings', JSON.stringify({ language: 'en-US' }))

    const store = useAppStore()

    expect(store.settings.preserveMarkOfWeb).toBe(true)
    store.settings.preserveMarkOfWeb = false
    store.resetSettings()
    expect(store.settings.preserveMarkOfWeb).toBe(true)
  })

  it('keeps legacy installations serial until concurrency is explicitly changed', () => {
    localStorage.setItem('app-settings', JSON.stringify({ maxConcurrentTasks: 4 }))

    const store = useAppStore()

    expect(store.settings.maxConcurrentTasks).toBe(1)
    expect(store.settings.archiveTaskConcurrencyVersion).toBe(1)
  })

  it('preserves an explicitly configured concurrency-v1 value', () => {
    localStorage.setItem('app-settings', JSON.stringify({
      maxConcurrentTasks: 3,
      archiveTaskConcurrencyVersion: 1,
    }))

    const store = useAppStore()

    expect(store.settings.maxConcurrentTasks).toBe(3)
  })

  it('disables legacy auto-start state without writing persistence during startup', () => {
    localStorage.setItem('app-settings', JSON.stringify({ autoStart: true }))

    const store = useAppStore()

    expect(store.settings.autoStart).toBe(false)
    expect(mocks.invoke).not.toHaveBeenCalledWith('set_auto_start', expect.anything())
    expect(mocks.invoke).not.toHaveBeenCalledWith('check_auto_start')
    expect(JSON.parse(localStorage.getItem('app-settings') || '{}').autoStart).toBe(false)
  })
})
