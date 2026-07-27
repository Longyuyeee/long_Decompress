import { beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { useAccessibility } from '@/composables/useAccessibility'
import { useAppStore } from '@/stores/app'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))

const createMediaQuery = (matches = false) => ({
  matches,
  media: '',
  onchange: null,
  addListener: vi.fn(),
  removeListener: vi.fn(),
  addEventListener: vi.fn(),
  removeEventListener: vi.fn(),
  dispatchEvent: vi.fn(),
})

describe('useAccessibility', () => {
  beforeEach(() => {
    document.documentElement.className = ''
    document.documentElement.style.cssText = ''
    setActivePinia(createPinia())
    vi.clearAllMocks()
    mocks.invoke.mockResolvedValue(undefined)
    vi.mocked(window.matchMedia).mockImplementation(() => createMediaQuery() as MediaQueryList)
  })

  it('applies every configured accessibility preference', () => {
    const appStore = useAppStore()
    appStore.settings.uiScale = 120
    appStore.settings.accessibility = {
      fontSize: 'x-large',
      highContrast: true,
      colorBlindMode: 'deuteranopia',
      reduceMotion: true,
      focusIndicator: false,
    }

    const accessibility = useAccessibility()
    accessibility.initAccessibility()

    const root = document.documentElement
    expect(root.style.fontSize).toBe('150%')
    expect(root.classList.contains('high-contrast')).toBe(true)
    expect(root.classList.contains('deuteranopia')).toBe(true)
    expect(root.classList.contains('reduce-motion')).toBe(true)
    expect(root.classList.contains('enhanced-focus')).toBe(false)

    accessibility.applyHighContrast(false)
    accessibility.applyColorBlindMode('none')
    accessibility.applyReduceMotion(false)
    accessibility.applyFocusIndicator(true)
    expect(root.classList.contains('high-contrast')).toBe(false)
    expect(root.classList.contains('deuteranopia')).toBe(false)
    expect(root.classList.contains('reduce-motion')).toBe(false)
    expect(root.classList.contains('enhanced-focus')).toBe(true)
  })

  it('reacts to accessibility setting changes after watchers are installed', async () => {
    const appStore = useAppStore()
    const accessibility = useAccessibility()
    accessibility.setupWatchers()

    appStore.settings.accessibility = {
      fontSize: 'large',
      highContrast: true,
      colorBlindMode: 'protanopia',
      reduceMotion: true,
      focusIndicator: false,
    }
    await nextTick()

    const root = document.documentElement
    expect(root.style.fontSize).toBe('112.5%')
    expect(root.classList.contains('high-contrast')).toBe(true)
    expect(root.classList.contains('protanopia')).toBe(true)
    expect(root.classList.contains('reduce-motion')).toBe(true)
    expect(root.classList.contains('enhanced-focus')).toBe(false)
  })

  it('observes system preferences and removes both listeners during cleanup', () => {
    const motionQuery = createMediaQuery(true)
    const contrastQuery = createMediaQuery(true)
    vi.mocked(window.matchMedia).mockImplementation((query: string) => {
      if (query.includes('reduced-motion')) return motionQuery as MediaQueryList
      if (query.includes('contrast')) return contrastQuery as MediaQueryList
      return createMediaQuery() as MediaQueryList
    })
    const log = vi.spyOn(console, 'log').mockImplementation(() => undefined)

    const cleanup = useAccessibility().watchSystemPreferences()

    expect(motionQuery.addEventListener).toHaveBeenCalledWith('change', expect.any(Function))
    expect(contrastQuery.addEventListener).toHaveBeenCalledWith('change', expect.any(Function))
    expect(log).toHaveBeenCalledWith('System prefers reduced motion')
    expect(log).toHaveBeenCalledWith('System prefers high contrast')

    cleanup()
    expect(motionQuery.removeEventListener).toHaveBeenCalledWith('change', expect.any(Function))
    expect(contrastQuery.removeEventListener).toHaveBeenCalledWith('change', expect.any(Function))
  })
})
