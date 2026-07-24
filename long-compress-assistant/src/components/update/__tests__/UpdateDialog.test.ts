import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import UpdateDialog from '../UpdateDialog.vue'
import { useAppStore } from '@/stores/app'
import { useTaskStore } from '@/stores/task'
import { useUpdateStore } from '@/stores/update'

const updaterMocks = vi.hoisted(() => ({
  checkUpdate: vi.fn(),
  installUpdate: vi.fn(),
  onUpdaterEvent: vi.fn(),
}))
const tauriMocks = vi.hoisted(() => ({
  invoke: vi.fn(async (command: string) => command === 'load_app_settings' ? '{}' : undefined),
}))

vi.mock('@tauri-apps/api/updater', () => updaterMocks)
vi.mock('@tauri-apps/api/tauri', () => tauriMocks)

const modalStub = {
  props: ['visible', 'title', 'description'],
  emits: ['update:visible'],
  template: `
    <section v-if="visible">
      <h2>{{ title }}</h2>
      <p v-if="description">{{ description }}</p>
      <slot />
      <footer><slot name="footer" /></footer>
      <button class="modal-close" @click="$emit('update:visible', false)">close</button>
    </section>
  `,
}

describe('UpdateDialog', () => {
  beforeEach(() => {
    localStorage.clear()
    updaterMocks.checkUpdate.mockReset()
    updaterMocks.installUpdate.mockReset()
    updaterMocks.onUpdaterEvent.mockReset()
    tauriMocks.invoke.mockClear()
    setActivePinia(createPinia())
  })

  it('blocks installation while a task is active and enables it after completion', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const mountedTaskStore = useTaskStore()
    const mountedUpdateStore = useUpdateStore()
    const mountedAppStore = useAppStore()
    mountedTaskStore.addTask({
      id: 'task-1',
      name: 'archive.zip',
      type: 'decompression',
      sourceFiles: ['archive.zip'],
      outputPath: 'output',
    })
    mountedUpdateStore.manifest = {
      version: '1.0.13',
      date: '2026-07-25',
      body: 'Reliability update',
    } as any
    mountedUpdateStore.status = 'available'
    mountedUpdateStore.dialogVisible = true
    const mounted = mount(UpdateDialog, {
      global: { plugins: [pinia], stubs: { Modal: modalStub } },
    })

    const installText = mountedAppStore.t('update.install')
    const installButton = mounted.findAll('button').find(button => button.text() === installText)
    expect(installButton?.attributes('disabled')).toBeDefined()
    expect(mounted.text()).toContain(
      mountedAppStore.t('update.active_tasks').replace('{0}', '1'),
    )

    mountedTaskStore.updateTaskStatus('task-1', 'completed')
    await nextTick()
    expect(installButton?.attributes('disabled')).toBeUndefined()

    await installButton?.trigger('click')
    await flushPromises()
    expect(updaterMocks.installUpdate).toHaveBeenCalledOnce()
    expect(mountedUpdateStore.status).toBe('installing')

    mounted.unmount()
  })

  it('supports skip, remind-later, and retry actions', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const appStore = useAppStore()
    const updateStore = useUpdateStore()
    updateStore.manifest = { version: '1.0.13', body: '', date: '' } as any
    updateStore.status = 'available'
    updateStore.dialogVisible = true
    const skipSpy = vi.spyOn(updateStore, 'skipCurrentVersion')
    const laterSpy = vi.spyOn(updateStore, 'remindLater')

    const wrapper = mount(UpdateDialog, {
      global: { plugins: [pinia], stubs: { Modal: modalStub } },
    })

    await wrapper.findAll('button')
      .find(button => button.text() === appStore.t('update.skip'))
      ?.trigger('click')
    expect(skipSpy).toHaveBeenCalledOnce()

    updateStore.status = 'error'
    updateStore.dialogVisible = true
    updateStore.errorMessage = 'network unavailable'
    const retrySpy = vi.spyOn(updateStore, 'checkForUpdates').mockResolvedValue()
    await nextTick()
    expect(wrapper.text()).toContain('network unavailable')
    await wrapper.findAll('button')
      .find(button => button.text() === appStore.t('update.retry'))
      ?.trigger('click')
    expect(retrySpy).toHaveBeenCalledWith(true)

    updateStore.status = 'up-to-date'
    await nextTick()
    await wrapper.findAll('button')
      .find(button => button.text() === appStore.t('update.ok'))
      ?.trigger('click')
    expect(laterSpy).toHaveBeenCalled()
  })

  it('cannot be dismissed while installation is running', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const updateStore = useUpdateStore()
    updateStore.manifest = { version: '1.0.13' } as any
    updateStore.status = 'installing'
    updateStore.dialogVisible = true
    const laterSpy = vi.spyOn(updateStore, 'remindLater')

    const wrapper = mount(UpdateDialog, {
      global: { plugins: [pinia], stubs: { Modal: modalStub } },
    })
    await wrapper.find('.modal-close').trigger('click')

    expect(laterSpy).not.toHaveBeenCalled()
    expect(updateStore.status).toBe('installing')
  })
})
