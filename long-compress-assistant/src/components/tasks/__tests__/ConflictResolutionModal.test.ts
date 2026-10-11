import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ConflictResolutionModal from '../ConflictResolutionModal.vue'
import { useTaskStore } from '@/stores/task'
import { useAppStore } from '@/stores/app'

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn(async () => undefined) }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))

describe('ConflictResolutionModal user choices', () => {
  beforeEach(() => { setActivePinia(createPinia()) })

  const setup = () => {
    const store = useTaskStore()
    store.addTask({ id: 'one', name: '照片.zip', type: 'decompression', sourceFiles: ['C:/照片.zip'], outputPath: 'C:/out' })
    store.tasks[0].conflicts.push({ taskId: 'one', fileName: '照片.jpg', sourcePath: 'C:/照片.zip', destPath: 'C:/out/照片.jpg', sourceSize: 2048, destSize: 1024, sourceModified: 0, destModified: 1700000000000 })
    const wrapper = mount(ConflictResolutionModal, {
      props: { taskId: 'one', visible: true },
      global: { stubs: { Modal: { template: '<div><slot /><slot name="footer" /></div>' } } },
    })
    return { wrapper, store }
  }

  it.each(['overwrite', 'skip', 'rename'] as const)('lets me apply %s to this task without silently choosing rename', async action => {
    const { wrapper, store } = setup()
    await wrapper.get('[data-testid="conflict-apply-remaining"]').setValue(true)
    expect(wrapper.emitted('resolve')).toBeUndefined()
    await wrapper.get(`[data-testid="conflict-${action}"]`).trigger('click')
    expect(wrapper.emitted('resolve')).toEqual([[action, true]])
    expect(store.tasks[0].conflicts).toHaveLength(1)
    expect(wrapper.emitted('update:visible')).toBeUndefined()
    wrapper.unmount()
  })

  it('defaults to one file, shows unknown timestamps honestly and blocks repeat clicks while saving', async () => {
    const { wrapper } = setup()
    expect(wrapper.text()).toContain(useAppStore().t('conflict.time_unknown'))
    expect(wrapper.text()).not.toContain('1970')
    await wrapper.get('[data-testid="conflict-skip"]').trigger('click')
    expect(wrapper.emitted('resolve')).toEqual([['skip', false]])
    await wrapper.setProps({ busy: true })
    await wrapper.get('[data-testid="conflict-overwrite"]').trigger('click')
    expect(wrapper.emitted('resolve')).toHaveLength(1)
    expect(wrapper.get('[role="status"]').text()).toContain(useAppStore().t('conflict.publishing'))
    wrapper.unmount()
  })

  it('does not carry the apply-to-remaining choice into another task or a reopened dialog', async () => {
    const { wrapper } = setup()
    await wrapper.get('[data-testid="conflict-apply-remaining"]').setValue(true)
    await wrapper.setProps({ visible: false })
    await wrapper.setProps({ visible: true })
    expect((wrapper.get('input').element as HTMLInputElement).checked).toBe(false)
    await wrapper.get('input').setValue(true)
    await wrapper.setProps({ taskId: 'other' })
    await wrapper.setProps({ taskId: 'one' })
    expect((wrapper.get('input').element as HTMLInputElement).checked).toBe(false)
    wrapper.unmount()
  })
})
