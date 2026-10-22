import { beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { useAppStore } from '../app'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))

describe('App Store Explorer context menu synchronization', () => {
  it('consumes archive input credentials once and does not carry them into an unrelated open', () => {
    const store=useAppStore()
    store.openArchiveInBrowser('C:/repaired.zip',{password:'session-only-secret',zipPasswordEncoding:'cp932'})
    expect(store.takeArchiveBrowserOptions()).toEqual({password:'session-only-secret',zipPasswordEncoding:'cp932'})
    expect(store.takeArchiveBrowserPath()).toBe('C:/repaired.zip')
    store.openArchiveInBrowser('C:/other.zip')
    expect(store.takeArchiveBrowserOptions()).toEqual({})
    expect(localStorage.getItem('app-settings') || '').not.toContain('session-only-secret')
  })
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

  it('persists validated archive task defaults across computers', () => {
    localStorage.setItem('app-settings', JSON.stringify({
      archiveTaskConcurrencyVersion: 1,
      defaultCompressionFormat: '7z',
      defaultCompressionLevel: 8,
      defaultExtractToSubfolder: true,
    }))

    const store = useAppStore()
    expect(store.settings).toMatchObject({
      defaultCompressionFormat: '7z',
      defaultCompressionLevel: 8,
      defaultExtractToSubfolder: true,
    })
  })

  it('keeps structural directory selection opt-in for old settings and after reset', () => {
    localStorage.setItem('app-settings', JSON.stringify({ defaultExtractToSubfolder: true }))
    const store = useAppStore()
    expect(store.settings.smartExtractionDirectory).toBe(false)
    store.updateSettings({ smartExtractionDirectory: true })
    setActivePinia(createPinia())
    const restored = useAppStore()
    expect(restored.settings.smartExtractionDirectory).toBe(true)
    restored.resetSettings()
    expect(restored.settings.smartExtractionDirectory).toBe(false)
  })

  it('keeps ZIP parallelism opt-in on old settings, reload and reset', () => {
    localStorage.setItem('app-settings', JSON.stringify({ parallelZipExtraction: 'true' }))
    const store = useAppStore()
    expect(store.settings.parallelZipExtraction).toBe(false)
    store.updateSettings({ parallelZipExtraction: true })
    setActivePinia(createPinia())
    const restored = useAppStore()
    expect(restored.settings.parallelZipExtraction).toBe(true)
    restored.resetSettings()
    expect(restored.settings.parallelZipExtraction).toBe(false)
  })

  it('loads auto-start display state without touching Windows persistence', () => {
    localStorage.setItem('app-settings', JSON.stringify({ autoStart: true }))

    const store = useAppStore()

    expect(store.settings.autoStart).toBe(true)
    expect(mocks.invoke).not.toHaveBeenCalledWith('set_auto_start', expect.anything())
    expect(mocks.invoke).not.toHaveBeenCalledWith('check_auto_start')
    expect(JSON.parse(localStorage.getItem('app-settings') || '{}').autoStart).toBe(true)
  })

  it('restores a signature theme from portable settings and switches without leftover classes', async () => {
    localStorage.setItem('app-settings', JSON.stringify({ theme: 'forest', accentColor: '#6bd4a1' }))
    const store = useAppStore()
    await nextTick()
    expect(store.theme).toBe('forest')
    expect(document.documentElement.classList.contains('mode-forest')).toBe(true)
    store.theme = 'glacier'
    await nextTick()
    expect(document.documentElement.classList.contains('mode-forest')).toBe(false)
    expect(document.documentElement.dataset.colorScheme).toBe('light')
    store.saveSettingsToStorage()
    expect(JSON.parse(localStorage.getItem('app-settings')!).theme).toBe('glacier')
    store.resetSettings()
    expect(store.theme).toBe('auto')
    expect(store.accentColor).toBe('#0ea5e9')
  })

  it('restores theme and accent when only backend settings survive an update', async () => {
    const fallback = mocks.invoke.getMockImplementation()!
    mocks.invoke.mockImplementation((command: string) => command === 'load_app_settings'
      ? Promise.resolve(JSON.stringify({ theme: 'obsidian', accentColor: '#d9b76f' })) : fallback(command))
    const store = useAppStore()
    await new Promise(resolve => setTimeout(resolve, 0))
    expect(store.theme).toBe('obsidian')
    expect(store.accentColor).toBe('#d9b76f')
    expect(localStorage.getItem('app-theme')).toBe('obsidian')
  })
})
