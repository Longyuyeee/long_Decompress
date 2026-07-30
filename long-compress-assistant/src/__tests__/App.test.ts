import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import App from '../App.vue'
import { useAppStore } from '@/stores/app'
import { useCompressionStore } from '@/stores/compression'
import { useTaskStore } from '@/stores/task'

const mocks = vi.hoisted(() => {
  const listeners = new Map<string, (event: any) => void>()
  return {
    listeners,
    invoke: vi.fn(),
    listen: vi.fn(async (event: string, callback: (payload: any) => void) => {
      listeners.set(event, callback)
      return vi.fn()
    }),
    routerPush: vi.fn(),
    hide: vi.fn(),
    setPosition: vi.fn(),
    setSize: vi.fn(),
    outerPosition: vi.fn(async () => ({ x: 20, y: 30 })),
    outerSize: vi.fn(async () => ({ width: 920, height: 620 })),
    onResized: vi.fn(async () => vi.fn()),
    initAccessibility: vi.fn(),
    setupWatchers: vi.fn(),
    cleanupSystemWatcher: vi.fn(),
    updaterCleanup: vi.fn(),
    contextActions: [] as Array<{ action: string; files: string[] }>,
  }
})

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: mocks.routerPush }),
}))

vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/window', () => ({
  appWindow: {
    hide: mocks.hide,
    setPosition: mocks.setPosition,
    setSize: mocks.setSize,
    outerPosition: mocks.outerPosition,
    outerSize: mocks.outerSize,
    onResized: mocks.onResized,
  },
  LogicalPosition: class {
    constructor(public x: number, public y: number) {}
  },
  LogicalSize: class {
    constructor(public width: number, public height: number) {}
  },
}))

vi.mock('@tauri-apps/api/updater', () => ({
  checkUpdate: vi.fn(),
  installUpdate: vi.fn(),
  onUpdaterEvent: vi.fn(),
}))

vi.mock('@/composables/useAccessibility', () => ({
  useAccessibility: () => ({
    initAccessibility: mocks.initAccessibility,
    setupWatchers: mocks.setupWatchers,
    watchSystemPreferences: () => mocks.cleanupSystemWatcher,
  }),
}))

const modalStub = {
  props: ['visible', 'title', 'description'],
  template: `
    <section v-if="visible" class="exit-modal">
      <h2>{{ title }}</h2>
      <p>{{ description }}</p>
      <slot />
      <footer><slot name="footer" /></footer>
    </section>
  `,
}

describe('App orchestration', () => {
  beforeEach(() => {
    localStorage.clear()
    mocks.listeners.clear()
    mocks.invoke.mockReset()
    mocks.listen.mockClear()
    mocks.routerPush.mockReset()
    mocks.hide.mockReset()
    mocks.setPosition.mockReset()
    mocks.setSize.mockReset()
    mocks.outerPosition.mockClear()
    mocks.outerSize.mockClear()
    mocks.onResized.mockClear()
    mocks.initAccessibility.mockReset()
    mocks.setupWatchers.mockReset()
    mocks.cleanupSystemWatcher.mockReset()

    mocks.contextActions = [{
      action: 'context-quick-pack',
      files: ['C:\\input\\alpha.txt', 'C:\\input\\beta.txt'],
    }]
    let contextActionsRead = false
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'take_pending_context_actions') {
        if (contextActionsRead) return []
        contextActionsRead = true
        return mocks.contextActions
      }
      if (command === 'get_file_info') return { size: 10, is_dir: false }
      if (command === 'path_exists') return false
      return undefined
    })
  })

  it('drains context actions, handles shortcuts, and safely resolves exit requests', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const appStore = useAppStore()
    const taskStore = useTaskStore()
    taskStore.addTask({
      id: 'active-task',
      name: 'active.zip',
      type: 'decompression',
      sourceFiles: ['active.zip'],
      outputPath: 'C:\\output',
    })

    const wrapper = mount(App, {
      attachTo: document.body,
      global: {
        plugins: [pinia],
        stubs: {
          MainLayout: true,
          ToastContainer: true,
          UpdateDialog: true,
          Modal: modalStub,
        },
      },
    })
    await flushPromises()

    expect(mocks.initAccessibility).toHaveBeenCalledOnce()
    expect(mocks.setupWatchers).toHaveBeenCalledOnce()
    expect(mocks.routerPush).toHaveBeenCalledWith('/compress')
    expect(appStore.successMessage).toContain('2')

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'o', ctrlKey: true }))
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', ctrlKey: true }))
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'i', ctrlKey: true }))
    window.dispatchEvent(new KeyboardEvent('keydown', { key: ',', ctrlKey: true }))
    expect(mocks.routerPush).toHaveBeenCalledWith('/decompress')
    expect(mocks.routerPush).toHaveBeenCalledWith('/integrity')
    expect(mocks.routerPush).toHaveBeenCalledWith('/settings')

    mocks.listeners.get('exit-confirmation-requested')?.({ payload: null })
    await nextTick()
    expect(wrapper.find('.exit-modal').exists()).toBe(true)
    const backgroundButton = wrapper.findAll('button').find(
      button => button.text() === appStore.t('exit.confirm.background'),
    )
    await backgroundButton?.trigger('click')
    await flushPromises()
    expect(mocks.hide).toHaveBeenCalledOnce()

    mocks.listeners.get('exit-confirmation-requested')?.({ payload: null })
    await nextTick()
    const exitButton = wrapper.findAll('button').find(
      button => button.text() === appStore.t('exit.confirm.stop_and_exit'),
    )
    await exitButton?.trigger('click')
    await flushPromises()

    expect(mocks.invoke).toHaveBeenCalledWith('cancel_tasks_and_wait', {
      taskIds: ['active-task'],
    })
    expect(mocks.invoke).toHaveBeenCalledWith('exit_app')
    expect(taskStore.tasks[0].status).toBe('cancelled')

    wrapper.unmount()
    expect(mocks.cleanupSystemWatcher).toHaveBeenCalledOnce()
  })

  it('replaces a finished source row when Explorer requests another compression format', async () => {
    mocks.contextActions = [{
      action: 'context-compress-7z',
      files: ['C:\\input\\alpha.txt'],
    }]
    const pinia = createPinia()
    setActivePinia(pinia)
    const compressionStore = useCompressionStore()
    const taskStore = useTaskStore()
    compressionStore.addFile({
      name: 'alpha.txt',
      path: 'C:\\input\\alpha.txt',
      size: 10,
      type: 'file',
      isDirectory: false,
    })
    compressionStore.bindJobTask(
      'C:\\input\\alpha.txt',
      'finished-zip',
      { ...compressionStore.globalSettings, format: 'zip' },
      'C:\\input\\alpha.zip',
    )
    taskStore.addTask({
      id: 'finished-zip',
      name: 'alpha.txt',
      type: 'compression',
      sourceFiles: ['C:\\input\\alpha.txt'],
      outputPath: 'C:\\input\\alpha.zip',
      format: 'zip',
    })
    taskStore.updateTaskStatus('finished-zip', 'completed')

    const wrapper = mount(App, {
      global: {
        plugins: [pinia],
        stubs: {
          MainLayout: true,
          ToastContainer: true,
          UpdateDialog: true,
          Modal: modalStub,
        },
      },
    })
    await flushPromises()

    expect(taskStore.tasks).toHaveLength(0)
    expect(compressionStore.selectedFiles).toHaveLength(1)
    expect(compressionStore.selectedFiles[0].path).toBe('C:\\input\\alpha.txt')
    expect(compressionStore.selectedFiles[0]).not.toHaveProperty('taskId')
    expect(compressionStore.globalSettings.format).toBe('7z')
    expect(compressionStore.autoStartRequested).toBe(true)
    expect(mocks.routerPush).toHaveBeenCalledWith('/compress')

    wrapper.unmount()
  })
})
