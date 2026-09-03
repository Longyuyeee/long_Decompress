import { describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import AeroTable from '../AeroTable.vue'
import { useTaskStore } from '@/stores/task'

vi.mock('@tauri-apps/api/dialog', () => ({ open: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: vi.fn(async (command: string) => command === 'load_app_settings' ? '{}' : undefined),
}))
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({
    listArchiveContents: vi.fn(async () => []),
    testArchiveIntegrity: vi.fn(async () => ''),
  }),
}))

describe('AeroTable', () => {
  it('keeps expanded password recovery controls inside a narrow config column', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const taskStore = useTaskStore()
    taskStore.addTask({
      id: 'password-required',
      name: 'encrypted.rar',
      type: 'decompression',
      sourceFiles: ['C:\\fixtures\\encrypted.rar'],
      outputPath: 'C:\\fixtures\\output',
      format: 'rar',
    })
    taskStore.updateTaskStatus('password-required', 'failed')
    taskStore.tasks[0].passwordRequired = true

    const wrapper = mount(AeroTable, {
      props: { taskType: 'decompression' },
      global: {
        plugins: [pinia],
        stubs: { Transition: false, TransitionGroup: false },
      },
    })

    await wrapper.get('[data-testid="task-row"]').trigger('click')
    const configPanel = wrapper.get('[data-testid="decompression-config-panel"]')
    const passwordInput = configPanel.get('input[type="password"]')
    const controls = passwordInput.element.parentElement!

    expect(controls.classList.contains('grid')).toBe(true)
    expect(controls.className).toContain('grid-cols-[minmax(0,1fr)_auto]')
    expect(passwordInput.classes()).toContain('col-span-2')
    expect(passwordInput.classes()).toContain('min-w-0')
    expect(configPanel.findAll('button').at(-1)?.text()).toContain('使用密码重试')
  })
})
