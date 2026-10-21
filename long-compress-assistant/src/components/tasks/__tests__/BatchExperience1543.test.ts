import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { taskScope, taskScopeCounts } from '@/utils/taskScope'
import { batchConfigSummary } from '@/utils/batchConfigSummary'
import TaskScopeBar from '../TaskScopeBar.vue'
import BatchConfigBar from '../BatchConfigBar.vue'
import BatchResultActions from '../BatchResultActions.vue'
import ConversionBatchConfig from '@/components/compression/ConversionBatchConfig.vue'
import ArchiveConversionModal from '@/components/compression/ArchiveConversionModal.vue'
import { useArchiveConversionStore } from '@/stores/archiveConversion'
import { useTaskStore } from '@/stores/task'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/dialog', () => ({ open: vi.fn(), save: vi.fn() }))
const options = () => ({ global: { plugins: [createPinia()], stubs: { Teleport: true, Transition: false, RouterLink: true } } })
beforeEach(() => { setActivePinia(createPinia()); mocks.invoke.mockReset(); mocks.invoke.mockImplementation(async command => command === 'detect_split_archive' ? {is_split:false,archive_format:'zip'} : command === 'load_app_settings' ? '{}' : undefined) })

describe('batch1543', () => {
  it.each(['preparing','running','extracting','compressing','paused','finalizing','cancelling'])('keeps %s visible as in progress', status => { expect(taskScope(status)).toBe('active') })
  it('counts each task once and treats drafts separately from failures', () => {
    expect(taskScopeCounts([undefined,'pending','completed','failed','cancelled','paused'])).toEqual({all:6,pending:2,active:1,completed:1,attention:2})
    const wrapper=mount(TaskScopeBar,{props:{modelValue:'all',statuses:['completed','failed']}})
    wrapper.get('[data-testid="task-scope-attention"]').trigger('click')
    expect(wrapper.emitted('update:modelValue')).toEqual([['attention']]); wrapper.unmount()
  })
  it('summarizes explicit changes without exposing source or target passwords', () => {
    const rows=batchConfigSummary({format:'zip',password:'secret-target',sourcePassword:'secret-source',sourceZipPasswordEncoding:undefined,outputDirectory:'',splitMiB:0})
    expect(rows).toEqual(['格式：ZIP','输出目录：各自源文件目录','新包密码：统一设置','原包密码：统一填写','分卷：关闭','原包读取编码：格式默认值'])
    expect(rows.join()).not.toContain('secret'); expect(batchConfigSummary({})).toEqual([])
  })
  it('shows the selected subjects only inside the original modal and previews the exact submitted patch',async()=>{
    const wrapper=mount(BatchConfigBar,{...options(),props:{mode:'decompression',count:2,subjects:[{name:'同名.zip',path:'C:/甲/同名.zip'},{name:'同名.zip',path:'D:/乙/同名.zip'}]}})
    expect(wrapper.find('[data-testid="batch-config-impact"]').exists()).toBe(false)
    await wrapper.get('button[aria-haspopup="dialog"]').trigger('click')
    expect(wrapper.get('[data-testid="batch-selected-subjects"]').text()).toContain('D:/乙/同名.zip')
    await wrapper.get('[data-testid="batch-subfolder"]').trigger('click')
    await wrapper.findAll('[role="option"]').find(e=>e.text().includes('创建同名'))!.trigger('click')
    expect(wrapper.get('[data-testid="batch-change-summary"]').text()).toBe('文件放置：创建同名子文件夹')
    await wrapper.get('[data-testid="batch-config-apply"]').trigger('click')
    expect(wrapper.emitted('apply')![0][0]).toEqual({extractToSubfolder:true}); wrapper.unmount()
  })
  it('conversion summary matches the patch and preserves unspecified credentials',async()=>{
    const pinia=createPinia();setActivePinia(pinia);const store=useArchiveConversionStore();store.addSources(['C:/a.zip','D:/b.zip']);await flushPromises()
    store.updateItem(store.items[0].id,{sourcePassword:'original',password:'target'})
    const wrapper=mount(ConversionBatchConfig,{global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}})
    await wrapper.get('[data-testid="conversion-batch-config"]').trigger('click');await wrapper.get('[data-testid="conversion-batch-level"]').trigger('click')
    await wrapper.findAll('[role="option"]').find(e=>e.find('strong').text()==='4')!.trigger('click')
    expect(wrapper.get('[data-testid="batch-change-summary"]').text()).toBe('压缩强度：4')
    await wrapper.get('[data-testid="conversion-batch-apply"]').trigger('click');await flushPromises()
    expect(store.effectiveSettings(store.items[0])).toMatchObject({level:4,sourcePassword:'original',password:'target'});expect(store.effectiveSettings(store.items[1]).level).toBe(4);wrapper.unmount()
  })
  it('conversion filtering clears hidden selections, keeps tasks unchanged, and reveals a new draft',async()=>{
    const pinia=createPinia();setActivePinia(pinia);const store=useArchiveConversionStore();store.addSources(['C:/a.zip','D:/b.zip']);await flushPromises()
    const tasks=useTaskStore();tasks.addTask({id:'failed1543',type:'compression',name:'a.zip',sourceFiles:['C:/a.zip'],outputPath:'D:/out/a.zip'});tasks.tasks[0].status='failed';store.items[0].taskId='failed1543';store.visible=true
    const wrapper=mount(ArchiveConversionModal,{global:{plugins:[pinia],stubs:{Teleport:true,Transition:false,ConversionSettingsFields:true}}});await flushPromises()
    const original=JSON.stringify(tasks.tasks)
    await wrapper.get('[data-testid="task-scope-attention"]').trigger('click');expect(store.selected).toHaveLength(0);expect(wrapper.findAll('[data-testid="conversion-item"]')).toHaveLength(1)
    await wrapper.get('[data-testid="task-scope-completed"]').trigger('click');expect(wrapper.find('[data-testid="task-scope-empty"]').exists()).toBe(true)
    store.addSources(['E:/new.zip']);await flushPromises();expect(wrapper.get('[data-testid="task-scope-pending"]').attributes('aria-pressed')).toBe('true');expect(wrapper.findAll('[data-testid="conversion-item"]')).toHaveLength(2);expect(JSON.stringify(tasks.tasks)).toBe(original)
    const start=vi.spyOn(store,'start').mockResolvedValue(undefined);await wrapper.get('[data-testid="conversion-start"]').trigger('click');expect(start).toHaveBeenCalledOnce();expect(wrapper.get('[data-testid="task-scope-all"]').attributes('aria-pressed')).toBe('true');wrapper.unmount()
  })
  it('lists inaccessible directories and retries only those, without reopening successful locations',async()=>{
    let missing=true
    mocks.invoke.mockImplementation(async(command,{path})=>command==='inspect_extraction_destination' ? path==='D:/missing'&&missing?'missing':'occupied' : undefined)
    const wrapper=mount(BatchResultActions,{props:{locations:[{path:'C:/ready/a.zip',directory:false},{path:'C:/ready/b.zip',directory:false},{path:'D:/missing/c.zip',directory:false}]}})
    await wrapper.findAll('button')[0].trigger('click');await flushPromises();expect(wrapper.get('[data-testid="batch-result-unavailable"]').text()).toContain('D:/missing')
    expect(mocks.invoke.mock.calls.filter(c=>c[0]==='open_in_explorer').map(c=>c[1].path)).toEqual(['C:/ready'])
    missing=false;await wrapper.get('[data-testid="batch-result-unavailable"] button').trigger('click');await flushPromises()
    expect(mocks.invoke.mock.calls.filter(c=>c[0]==='open_in_explorer').map(c=>c[1].path)).toEqual(['C:/ready','D:/missing']);expect(wrapper.find('[data-testid="batch-result-unavailable"]').exists()).toBe(false);wrapper.unmount()
  })
})
