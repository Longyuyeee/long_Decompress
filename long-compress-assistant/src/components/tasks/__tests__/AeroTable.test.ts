import { describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import AeroTable from '../AeroTable.vue'
import { useTaskStore } from '@/stores/task'

vi.mock('@tauri-apps/api/dialog', () => ({ open: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: vi.fn(async (command: string) => command === 'load_app_settings' ? '{}' : command === 'preview_extraction_output' ? 'C:/output' : undefined),
}))
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({
    listArchiveContents: vi.fn(async () => []),
    testArchiveIntegrity: vi.fn(async () => ''),
  }),
}))

describe('AeroTable', () => {
  it('comparison1555 restored encrypted tasks can refill the password inline without changing the saved scope',async()=>{
    const pinia=createPinia();setActivePinia(pinia);const store=useTaskStore();store.addTask({id:'restored-compare',name:'selected ZIP',type:'decompression',sourceFiles:['C:/group.zip.001'],outputPath:'D:/out',draftPasswordRequired:true,comparisonSelection:{source:{path:'C:/group.zip.001',kind:'zip',revision:'a'.repeat(64),encrypted:true},paths:['one.txt'],keepStructure:true,conflictPolicy:'rename',zipPasswordEncoding:'cp936'}});const view=mount(AeroTable,{props:{taskType:'decompression'},global:{plugins:[pinia],stubs:{Teleport:true,ExtractionOutputPreview:true}}});await view.get('[data-testid="toggle-archive-task-restored-compare"]').trigger('click');await flushPromises();await view.get('[data-testid="comparison-task-password-restored-compare"]').setValue('fresh-password');expect(store.tasks[0]).toMatchObject({password:'fresh-password',draftPasswordRequired:false,comparisonSelection:{paths:['one.txt'],zipPasswordEncoding:'cp936'}});view.unmount()
  })

  it('edits only the pending ZIP input choice and leaves shared output inheritance intact', async () => {
    const pinia = createPinia(); setActivePinia(pinia)
    const store = useTaskStore()
    store.addTask({ id: 'zip-choice', name: 'a.zip', type: 'decompression', sourceFiles: ['C:/a.zip'], outputPath: 'D:/out', configurationMode: 'global' })
    const wrapper = mount(AeroTable, { props: { taskType: 'decompression' }, global: { plugins: [pinia], stubs: { Teleport: true, ExtractionOutputPreview: true } } })
    await wrapper.get('[data-testid="toggle-archive-task-zip-choice"]').trigger('click'); await flushPromises()
    const field = wrapper.findComponent({ name: 'ZipPasswordEncodingField' })
    field.vm.$emit('update:modelValue', 'cp936')
    expect(store.tasks[0]).toMatchObject({ zipPasswordEncoding: 'cp936', outputPath: 'D:/out', configurationMode: 'global' })
    store.tasks[0].status = 'extracting'; await flushPromises()
    expect(wrapper.find('[data-testid="task-zip-password-encoding"]').exists()).toBe(false)
    wrapper.unmount()
  })
  it('displays a readable final reason without changing the original log evidence', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const store = useTaskStore()
    store.addTask({ id: 'missing-after-import', name: '资料.zip', type: 'decompression', sourceFiles: ['C:/资料.zip'], outputPath: 'C:/out' })
    const original = '最终失败原因：[archive-inspection:incomplete] 缺少分卷：资料.z02'
    store.tasks[0].status = 'failed'
    store.tasks[0].logs.push({ task_id: 'missing-after-import', message: original, severity: 'error', timestamp: new Date().toISOString() })
    const wrapper = mount(AeroTable, { props: { taskType: 'decompression' }, global: { plugins: [pinia], stubs: { Teleport: true, ExtractionOutputPreview: true, FailedExtractionRetry: true } } })
    await wrapper.get('[data-testid="toggle-archive-task-missing-after-import"]').trigger('click')
    await flushPromises()
    const log = wrapper.get('[data-testid="decompression-log-viewport"] .task-log-message')
    expect(log.text()).toBe('最终失败原因：缺少分卷：资料.z02')
    expect(log.attributes('title')).toBe(log.text())
    expect(store.tasks[0].logs[0].message).toBe(original)
    wrapper.unmount()
  })
  it('settles detail progress on terminal status without waiting for animation', async () => {
    vi.stubGlobal('requestAnimationFrame', vi.fn(() => 1))
    vi.stubGlobal('cancelAnimationFrame', vi.fn())
    const pinia = createPinia()
    setActivePinia(pinia)
    const store = useTaskStore()
    store.addTask({ id: 'terminal-progress', name: '资料.zip', type: 'decompression', sourceFiles: ['C:/资料.zip'], outputPath: 'C:/out' })
    store.tasks[0].status = 'extracting'
    store.tasks[0].progress = 20
    const wrapper = mount(AeroTable, { props: { taskType: 'decompression' }, global: { plugins: [pinia], stubs: { Teleport: true, ExtractionOutputPreview: true } } })
    try {
      await wrapper.get('[data-testid="task-row"]').trigger('click')
      store.tasks[0].progress = 66
      await flushPromises()
      expect(wrapper.get('[data-testid="decompression-detail-progress"]').text()).toBe('20.00%')
      store.tasks[0].status = 'failed'
      await flushPromises()
      expect(wrapper.get('[data-testid="decompression-detail-progress"]').text()).toBe('66.00%')
      store.tasks[0].status = 'completed'
      store.tasks[0].progress = 100
      await flushPromises()
      expect(wrapper.get('[data-testid="decompression-detail-progress"]').text()).toBe('100%')
    } finally {
      wrapper.unmount()
      vi.unstubAllGlobals()
    }
  })
  it('does not intercept spaces or Enter from row controls while preserving row keyboard expansion', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const store = useTaskStore()
    store.addTask({ id: 'password-space', name: '资料.zip.001', type: 'decompression', sourceFiles: ['C:/资料.zip.001'], outputPath: 'C:/out' })
    store.tasks[0].status = 'failed'
    store.tasks[0].passwordRequired = true
    const wrapper = mount(AeroTable, { props: { taskType: 'decompression' }, global: { plugins: [pinia], stubs: { Teleport: true, ExtractionOutputPreview: true } } })
    const row = wrapper.get('[data-testid="task-row"]')
    for (const control of [wrapper.get('.task-password-cell input'), wrapper.get('.task-password-cell button')]) {
      for (const key of [' ', 'Enter']) {
        const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true })
        control.element.dispatchEvent(event)
        await flushPromises()
        expect(event.defaultPrevented).toBe(false)
        expect(row.attributes('aria-expanded')).toBe('false')
      }
    }
    await row.trigger('keydown', { key: ' ' })
    expect(row.attributes('aria-expanded')).toBe('true')
    await row.trigger('keydown', { key: 'Enter' })
    expect(row.attributes('aria-expanded')).toBe('false')
    wrapper.unmount()
  })
  it('expands in the list and automatically preserves settings only after editing', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const store = useTaskStore()
    store.addTask({ id: 'inline', name: 'inline.zip', type: 'decompression', sourceFiles: ['C:/inline.zip'], outputPath: 'C:/output', configurationMode: 'global', extractToSubfolder: false })
    const wrapper = mount(AeroTable, { props: { taskType: 'decompression' }, global: { plugins: [pinia], stubs: { Teleport: true, ExtractionOutputPreview: true } } })
    await wrapper.get('[data-testid="task-row"]').trigger('click')
    expect(wrapper.find('[role="dialog"]').exists()).toBe(false)
    expect(wrapper.get('.table-body').find('[data-testid="decompression-task-details"]').exists()).toBe(true)
    expect(store.tasks[0].configurationMode).toBe('global')
    await wrapper.get('.task-subfolder-option').trigger('click')
    expect(store.tasks[0].extractToSubfolder).toBe(true)
    expect(store.tasks[0].configurationMode).toBe('individual')
    wrapper.unmount()
  })
  it('只提示本次选中的待执行任务共用目录，忽略大小写和尾部斜杠差异', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const store = useTaskStore()
    for (const id of ['one', 'two']) store.addTask({ id, name: `${id}.zip`, type: 'decompression', sourceFiles: [`C:/${id}.zip`], outputPath: 'C:/output' })
    const wrapper = mount(AeroTable, { props: { taskType: 'decompression' }, global: { plugins: [pinia], stubs: { Teleport: true,
      ExtractionOutputPreview: { name: 'ExtractionOutputPreview', props: ['sharedCount'], template: '<div :data-shared="sharedCount" />' },
    } } })
    expect(wrapper.findAllComponents({ name: 'ExtractionOutputPreview' })).toHaveLength(0)
    await wrapper.get('[data-testid="toggle-archive-task-one"]').trigger('click')
    await flushPromises()
    const preview = wrapper.findComponent({ name: 'ExtractionOutputPreview' })
    expect(preview.props('sharedCount')).toBe(2)
    expect(wrapper.get('[data-testid="task-row"]').find('[data-shared]').exists()).toBe(false)
    await wrapper.setProps({ selectedTaskIds: new Set(['one']) })
    await flushPromises()
    expect(wrapper.findComponent({ name: 'ExtractionOutputPreview' }).props('sharedCount')).toBe(1)
    wrapper.unmount()
  })
  it('limits failure inspection to explicit task IDs and treats an empty scope as empty', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const store = useTaskStore()
    for (const id of ['old-failure', 'this-run-failure']) {
      store.addTask({ id, name: id, type: 'decompression', sourceFiles: [`C:/${id}.zip`], outputPath: 'C:/out' })
      store.tasks.find(task => task.id === id)!.status = 'failed'
    }
    const wrapper = mount(AeroTable, {
      props: { taskType: 'decompression', taskIds: ['this-run-failure'] },
      global: { plugins: [pinia], stubs: { Teleport: true, Transition: false, TransitionGroup: false } },
    })
    expect(wrapper.findAll('[data-testid="task-row"]')).toHaveLength(1)
    expect(wrapper.text()).toContain('this-run-failure')
    expect(wrapper.text()).not.toContain('old-failure')
    await wrapper.setProps({ taskIds: [] })
    await vi.waitFor(() => expect(wrapper.findAll('[data-testid="task-row"]')).toHaveLength(0))
    await wrapper.setProps({ taskIds: undefined })
    expect(wrapper.findAll('[data-testid="task-row"]')).toHaveLength(2)
    wrapper.unmount()
  })

  it('sorts imported tasks by natural name and exposes full truncated names', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const taskStore = useTaskStore()
    for (const [id, name] of [['20', '福建兄弟视频-20.rar'], ['2', '福建兄弟视频-2.rar'], ['10', '福建兄弟视频-10.rar']] as const) {
      taskStore.addTask({ id, name, type: 'decompression', sourceFiles: [`C:\\fixtures\\${name}`], outputPath: 'C:\\fixtures\\output', format: 'rar' })
    }

    const wrapper = mount(AeroTable, {
      props: { taskType: 'decompression' },
      global: { plugins: [pinia], stubs: { Teleport: true, Transition: false, TransitionGroup: false } },
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
      global: { plugins: [pinia], stubs: { Teleport: true, Transition: false, TransitionGroup: false } },
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
      global: { plugins: [pinia], stubs: { Teleport: true, Transition: false, TransitionGroup: false } },
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
        stubs: { Teleport: true, Transition: false, TransitionGroup: false },
      },
    })

    await wrapper.get('[data-testid="task-row"]').trigger('click')
    const configPanel = wrapper.get('[data-testid="decompression-config-panel"]')
    expect(configPanel.find('.config-source-switch').exists()).toBe(false)
    const passwordInput = configPanel.get('input[type="password"]')
    expect(configPanel.find('[data-testid="inline-extraction-recovery"]').exists()).toBe(true)
    expect(passwordInput.attributes('data-testid')).toBe('retry-password')
    expect(configPanel.get('[data-testid="create-retry-drafts"]').text()).toContain('检查并加入待处理队列')
    expect(configPanel.find('.retry-fields').exists()).toBe(true)
  })

  it('shows the actual password candidate in expanded extraction details', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const taskStore = useTaskStore()
    taskStore.addTask({
      id: 'password-attempt',
      name: 'encrypted.rar',
      type: 'decompression',
      sourceFiles: ['C:\\fixtures\\encrypted.rar'],
      outputPath: 'C:\\fixtures\\output',
      format: 'rar',
    })
    taskStore.updateTaskStatus('password-attempt', 'extracting')
    taskStore.tasks[0].currentPassword = '保险箱「常用密码」· 密码「open-sesame」'
    taskStore.tasks[0].passwordAttemptCurrent = 2
    taskStore.tasks[0].passwordAttemptTotal = 5

    const wrapper = mount(AeroTable, {
      props: { taskType: 'decompression' },
      global: { plugins: [pinia], stubs: { Teleport: true, Transition: false, TransitionGroup: false } },
    })
    await wrapper.get('[data-testid="task-row"]').trigger('click')

    const executionPanel = wrapper.get('[data-testid="decompression-execution-panel"]')
    expect(executionPanel.text()).toContain('密码「open-sesame」')
    expect(executionPanel.text()).toContain('当前实际候选密码会直接显示')
    expect(executionPanel.text()).not.toContain('不在执行日志中记录密码明文')
  })
})
