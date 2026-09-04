import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import type { Task, TaskStatus } from '@/stores/task'
import CompressionExecutionPanel from '../CompressionExecutionPanel.vue'
import CompressionStatusCell from '../CompressionStatusCell.vue'
import CompressionToolbar from '../CompressionToolbar.vue'

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn().mockResolvedValue('{}') }))

const task = (status: TaskStatus = 'pending', overrides: Partial<Task> = {}): Task => ({
  id: 'task-1',
  name: 'sample.zip',
  type: 'compression',
  status,
  progress: 0,
  logs: [],
  sourceFiles: ['C:/input/sample.txt'],
  outputPath: 'C:/output/sample.zip',
  conflicts: [],
  ...overrides,
})

const mountWithPinia = (component: Parameters<typeof mount>[0], props: Record<string, unknown> = {}) =>
  mount(component, {
    props,
    global: { plugins: [createPinia()] },
  })

describe('compression presentation components', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('exposes all toolbar actions and reflects the busy state', async () => {
    const wrapper = mountWithPinia(CompressionToolbar, {
      hasFinished: true,
      activeCount: 1,
      pausableCount: 1,
      pausedCount: 0,
      pendingCount: 2,
      busy: true,
      configurationMode: 'global',
    })
    const start = wrapper.get('[data-testid="start-compression"]')
    expect(start.attributes('disabled')).toBeDefined()
    expect(start.find('i').classes()).toContain('pi-spinner')

    await wrapper.findAll('button')[0].trigger('click')
    await wrapper.findAll('button')[1].trigger('click')
    await wrapper.findAll('button')[2].trigger('click')
    await wrapper.get('[data-testid="compression-config-mode-individual"]').trigger('click')
    await wrapper.get('[data-testid="open-global-compression-settings"]').trigger('click')
    await start.trigger('click')

    expect(wrapper.emitted('clearFinished')).toHaveLength(1)
    expect(wrapper.emitted('pauseActive')).toHaveLength(1)
    expect(wrapper.emitted('cancelActive')).toHaveLength(1)
    expect(wrapper.emitted('updateConfigurationMode')?.[0]).toEqual(['individual'])
    expect(wrapper.emitted('openSettings')).toHaveLength(1)
    expect(wrapper.emitted('start')).toBeUndefined()
  })

  it('hides optional toolbar actions when there is no matching work', () => {
    const wrapper = mountWithPinia(CompressionToolbar, {
      hasFinished: false,
      activeCount: 0,
      pausableCount: 0,
      pausedCount: 0,
      pendingCount: 0,
      busy: false,
      configurationMode: 'global',
    })

    expect(wrapper.findAll('button')).toHaveLength(3)
    expect(wrapper.get('[data-testid="compression-batch-config-mode"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('全局设置')
  })

  it('renders pending, active, and terminal status states consistently', async () => {
    const wrapper = mountWithPinia(CompressionStatusCell)
    expect(wrapper.text()).toContain('等待中')
    expect(wrapper.text()).not.toContain('%')

    await wrapper.setProps({ task: task('compressing', { progress: 42 }) })
    expect(wrapper.text()).toContain('压缩中')
    expect(wrapper.text()).toContain('42.00%')
    expect(wrapper.find('[style="width: 42%;"]').exists()).toBe(true)

    await wrapper.setProps({ task: task('completed', { progress: 100 }) })
    expect(wrapper.text()).toContain('已完成')
    expect(wrapper.text()).not.toContain('100%')
  })

  it('renders execution progress, current file, logs, and empty states', async () => {
    const wrapper = mountWithPinia(CompressionExecutionPanel)
    expect(wrapper.text()).toContain('等待开始压缩')

    await wrapper.setProps({
      task: task('compressing', {
        progress: 42,
        stage: 'Finalizing',
        speed: '12 MB/s',
        processedBytes: 10 * 1024 * 1024,
        totalBytes: 20 * 1024 * 1024,
        outputBytes: 6 * 1024 * 1024,
        currentFile: 'C:/input/sample.txt',
        logs: [
          {
            task_id: 'task-1',
            timestamp: '2026-07-30T10:00:00.000Z',
            message: 'archive warning',
            severity: 'warning',
          },
          {
            task_id: 'task-1',
            timestamp: '2026-07-30T10:00:01.000Z',
            message: 'archive ready',
            severity: 'success',
          },
        ],
      }),
    })

    expect(wrapper.text()).toContain('正在收尾')
    expect(wrapper.text()).toContain('42.00%')
    expect(wrapper.text()).toContain('12 MB/s')
    expect(wrapper.get('[data-testid="compression-live-metrics"]').text()).toContain('10 MB')
    expect(wrapper.get('[data-testid="compression-live-metrics"]').text()).toContain('6 MB')
    expect(wrapper.get('[data-testid="compression-live-metrics"]').text()).toContain('60.00%')
    expect(wrapper.get('[data-testid="compression-live-metrics"]').text()).toContain('节省 40.00%')
    expect(wrapper.text()).toContain('C:/input/sample.txt')
    expect(wrapper.text()).toContain('archive warning')
    expect(wrapper.find('.text-yellow-400').exists()).toBe(true)
    expect(wrapper.find('.text-green-400').exists()).toBe(true)

    await wrapper.setProps({ task: task('compressing', { progress: 100, stage: 'Verifying' }) })
    expect(wrapper.text()).toContain('正在校验')

    await wrapper.setProps({ task: task('running') })
    expect(wrapper.text()).toContain('暂无执行日志')
  })
})
