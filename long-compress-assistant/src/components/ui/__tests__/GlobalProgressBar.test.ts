import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import GlobalProgressBar from '../GlobalProgressBar.vue'
import { useAppStore } from '@/stores/app'
import { useTaskStore } from '@/stores/task'

const commandMocks = vi.hoisted(() => ({
  openInExplorer: vi.fn(),
  decompressFile: vi.fn(),
  compressFiles: vi.fn(),
}))
const tauriMocks = vi.hoisted(() => ({
  invoke: vi.fn(async (command: string) => command === 'load_app_settings' ? '{}' : undefined),
}))

vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => commandMocks,
}))
vi.mock('@tauri-apps/api/tauri', () => tauriMocks)

describe('GlobalProgressBar', () => {
  beforeEach(() => {
    commandMocks.openInExplorer.mockReset()
    commandMocks.decompressFile.mockReset()
    commandMocks.compressFiles.mockReset()
    tauriMocks.invoke.mockClear()
  })

  it('summarizes, expands, sorts, opens output, and clears completed tasks', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const appStore = useAppStore()
    const taskStore = useTaskStore()
    taskStore.addTask({
      id: 'running',
      name: 'running.7z',
      type: 'decompression',
      sourceFiles: ['running.7z'],
      outputPath: 'C:\\output\\running',
    })
    taskStore.addTask({
      id: 'done',
      name: 'done.zip',
      type: 'compression',
      sourceFiles: ['done.txt'],
      outputPath: 'C:\\output\\done.zip',
    })
    taskStore.updateTaskStatus('running', 'extracting')
    taskStore.updateTaskStatus('done', 'completed')
    const running = taskStore.tasks.find(task => task.id === 'running')!
    const done = taskStore.tasks.find(task => task.id === 'done')!
    running.progress = 25
    running.stage = 'Extracting'
    running.currentFile = 'C:\\input\\payload.txt'
    running.speed = '12 MB/s'
    done.progress = 100

    const wrapper = mount(GlobalProgressBar, {
      global: { plugins: [pinia] },
    })

    expect(wrapper.text()).toContain('63%')
    expect(wrapper.text()).toContain(`1 ${appStore.t('tasks.active')}`)
    expect(wrapper.text()).toContain('payload.txt')

    await wrapper.find('.progress-summary').trigger('click')
    expect(wrapper.text()).toContain('running.7z')
    expect(wrapper.text()).toContain('done.zip')
    expect(wrapper.text()).toContain('1/2')

    const openButtons = wrapper.findAll('button').filter(
      button => button.attributes('title') === appStore.t('tasks.open_folder'),
    )
    await openButtons[0].trigger('click')
    expect(commandMocks.openInExplorer).toHaveBeenCalledWith('C:\\output\\running')

    const clearButton = wrapper.findAll('button').find(
      button => button.text() === appStore.t('tasks.clear_done'),
    )
    await clearButton?.trigger('click')
    expect(taskStore.tasks.map(task => task.id)).toEqual(['running'])
  })

  it('minimizes to a status dot and restores the summary', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const appStore = useAppStore()
    const taskStore = useTaskStore()
    taskStore.addTask({
      id: 'pending',
      name: 'pending.zip',
      type: 'decompression',
      sourceFiles: ['pending.zip'],
      outputPath: '',
    })

    const wrapper = mount(GlobalProgressBar, {
      global: { plugins: [pinia] },
    })
    const minimize = wrapper.findAll('button').find(
      button => button.attributes('title') === appStore.t('common.minimize'),
    )
    await minimize?.trigger('click')
    await nextTick()

    expect(wrapper.find('.progress-summary').exists()).toBe(false)
    const restore = wrapper.find(
      `button[title="${appStore.t('tasks.show_progress')}"]`,
    )
    expect(restore.exists()).toBe(true)
    await restore.trigger('click')
    expect(wrapper.find('.progress-summary').exists()).toBe(true)
  })

  it('renders password verification at exactly zero extraction progress', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const taskStore = useTaskStore()
    taskStore.addTask({
      id: 'password-check',
      name: 'encrypted.rar',
      type: 'decompression',
      sourceFiles: ['encrypted.rar'],
      outputPath: 'C:\\output\\encrypted',
    })
    taskStore.updateTaskStatus('password-check', 'extracting')
    const task = taskStore.tasks[0]
    task.stage = 'password-attempt'
    task.progress = 0
    task.currentPassword = '保险箱候选'
    task.passwordAttemptCurrent = 2
    task.passwordAttemptTotal = 2

    const wrapper = mount(GlobalProgressBar, {
      global: { plugins: [pinia] },
    })
    await wrapper.find('.progress-summary').trigger('click')

    expect(wrapper.text()).toContain('验证解压密码')
    expect(wrapper.text()).toContain('0%')
    expect(wrapper.text()).toContain('2/2')
    expect(wrapper.find('.progress-bar-fill').attributes('style')).toContain('width: 0%')
  })
})
