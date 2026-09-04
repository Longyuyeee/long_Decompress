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
  it('sorts imported tasks by natural name and exposes full truncated names', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const taskStore = useTaskStore()
    for (const [id, name] of [['20', '福建兄弟视频-20.rar'], ['2', '福建兄弟视频-2.rar'], ['10', '福建兄弟视频-10.rar']] as const) {
      taskStore.addTask({ id, name, type: 'decompression', sourceFiles: [`C:\\fixtures\\${name}`], outputPath: 'C:\\fixtures\\output', format: 'rar' })
    }

    const wrapper = mount(AeroTable, {
      props: { taskType: 'decompression' },
      global: { plugins: [pinia], stubs: { Transition: false, TransitionGroup: false } },
    })

    const nameCells = wrapper.findAll('.task-name-cell > .overflow-tooltip-anchor')
    const names = nameCells.map(item => item.text())
    expect(names).toEqual(['福建兄弟视频-2.rar', '福建兄弟视频-10.rar', '福建兄弟视频-20.rar'])
    expect(nameCells.every(item => item.attributes('title') === undefined)).toBe(true)
    expect(wrapper.findAll('[data-testid="task-row"]').every(row => row.classes().includes('grid'))).toBe(true)
  })

  it('centers terminal status while reserving aligned runtime columns for active tasks', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const taskStore = useTaskStore()
    taskStore.addTask({ id: 'done', name: 'done.rar', type: 'decompression', sourceFiles: ['done.rar'], outputPath: '', format: 'rar' })
    taskStore.addTask({ id: 'active', name: 'active.rar', type: 'decompression', sourceFiles: ['active.rar'], outputPath: '', format: 'rar' })
    taskStore.updateTaskStatus('done', 'completed')
    taskStore.updateTaskStatus('active', 'extracting')

    const wrapper = mount(AeroTable, {
      props: { taskType: 'decompression' },
      global: { plugins: [pinia], stubs: { Transition: false, TransitionGroup: false } },
    })
    const rows = wrapper.findAll('[data-testid="task-row"]')
    expect(rows[0].get('.task-status-cell').classes()).not.toContain('is-terminal')
    expect(rows[0].find('.task-status-runtime').exists()).toBe(true)
    expect(rows[1].get('.task-status-cell').classes()).toContain('is-terminal')
    expect(rows[1].find('.task-status-runtime').exists()).toBe(false)
  })

  it('keeps row controls ordered in one dedicated action cell', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const taskStore = useTaskStore()
    taskStore.addTask({
      id: 'long-pending-name',
      name: 'digital-rural-platform.zip (2 个超长名称测试副本).zip',
      type: 'decompression',
      sourceFiles: ['C:\\fixtures\\digital-rural-platform.zip'],
      outputPath: 'C:\\fixtures\\output',
      format: 'zip',
    })

    const wrapper = mount(AeroTable, {
      props: { taskType: 'decompression' },
      global: { plugins: [pinia], stubs: { Transition: false, TransitionGroup: false } },
    })

    const actionCell = wrapper.get('[data-testid="task-action-cell"]')
    const controls = actionCell.findAll('button')
    expect(controls).toHaveLength(2)
    expect(controls[0].attributes('data-testid')).toBe('remove-archive-task-long-pending-name')
    expect(controls[1].attributes('data-testid')).toBe('toggle-archive-task-long-pending-name')
    expect(actionCell.find('.task-action-divider').exists()).toBe(true)

    await controls[1].trigger('click')
    expect(controls[1].attributes('aria-expanded')).toBe('true')
  })

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
