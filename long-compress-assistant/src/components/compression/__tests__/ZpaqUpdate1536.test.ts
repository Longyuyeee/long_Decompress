import { beforeEach, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ZpaqUpdateFields from '../ZpaqUpdateFields.vue'
import ConversionBatchConfig from '../ConversionBatchConfig.vue'
import ArchiveOutputSummary from '../ArchiveOutputSummary.vue'
import { defaultConversionSettings } from '@/utils/archiveConversionDraft'
import { useArchiveConversionStore } from '@/stores/archiveConversion'
import { copyExecutionConfig } from '@/types/executionConfig'
const mocks=vi.hoisted(()=>({open:vi.fn(),invoke:vi.fn()}))
vi.mock('@tauri-apps/api/dialog',()=>({open:mocks.open}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
beforeEach(()=>{setActivePinia(createPinia());mocks.open.mockReset();mocks.invoke.mockReset();mocks.invoke.mockImplementation(async cmd=>cmd==='load_app_settings'?'{}':{is_split:false,archive_format:'zpaq'})})
it('zpaq1536 shows actual copy cost and exact path mapping, merges choices, and reports selection errors inline',async()=>{
  const wrapper=mount(ZpaqUpdateFields,{props:{settings:{...defaultConversionSettings(),updateSources:['C:/说明.txt'],archivePrefix:'资料'},baseBytes:1048576}})
  expect(wrapper.text()).toContain('1 MB');expect(wrapper.text()).toContain('资料/说明.txt');expect(wrapper.text()).toContain('旧内容保留');expect(wrapper.text()).toContain('改密')
  mocks.open.mockResolvedValue(['C:/说明.txt','C:/new.txt']);await wrapper.get('[data-testid="zpaq-update-add-files"]').trigger('click');await flushPromises()
  expect(wrapper.emitted('update')).toEqual([[{updateSources:['C:/说明.txt','C:/new.txt']}]])
  mocks.open.mockRejectedValue(Error('dialog unavailable'));await wrapper.get('[data-testid="zpaq-update-add-folder"]').trigger('click');await flushPromises();expect(wrapper.get('[role="alert"]').text()).toContain('dialog unavailable');wrapper.unmount()
})
it('zpaq1536 bulk configuration stays in a modal and hides ineffective format and new-password controls',async()=>{
  const pinia=createPinia();setActivePinia(pinia);const store=useArchiveConversionStore();store.showUpdate(['C:/one.zpaq','C:/two.zpaq'])
  const wrapper=mount(ConversionBatchConfig,{global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}});await flushPromises()
  expect(wrapper.find('[data-testid="conversion-batch-fields"]').exists()).toBe(false)
  await wrapper.get('[data-testid="conversion-batch-config"]').trigger('click');await flushPromises()
  expect(wrapper.find('[data-testid="conversion-batch-format"]').exists()).toBe(false);expect(wrapper.text()).toContain('原密码')
  await wrapper.get('[data-testid="conversion-batch-level"]').trigger('click');const option=wrapper.findAll('[role="option"]').find(o=>o.text().trim()==='3')!;expect(option).toBeDefined();await option.trigger('click');await wrapper.get('[data-testid="conversion-batch-apply"]').trigger('click');await flushPromises()
  expect(store.pending.map(i=>store.effectiveSettings(i).level)).toEqual([3,3]);expect(wrapper.find('[data-testid="conversion-batch-fields"]').exists()).toBe(false);wrapper.unmount()
})
it('zpaq1536 result distinguishes unchanged copies and persisted execution config rejects malformed updates',()=>{
  const config={schemaVersion:1 as const,kind:'archive-conversion' as const,operation:'zpaq-update' as const,updateSources:['C:/new.txt'],archivePrefix:'资料',format:'zpaq',outputPath:'D:/new.zpaq',level:3,splitSize:0,encryptFilenames:false,passwordRequired:true}
  const copy=copyExecutionConfig(config)!;config.updateSources.push('C:/more.txt');expect(copy).toMatchObject({updateSources:['C:/new.txt']});expect(copyExecutionConfig({...config,format:'zip'})).toBeNull();expect(copyExecutionConfig({...config,updateSources:[]})).toBeNull()
  const output={primaryPath:'D:/new.zpaq',files:[],split:false,totalBytes:100,updateSummary:{updatedEntries:0,newFragments:0,baseSnapshots:3,snapshots:3,unchanged:true,copiedBytes:100,inputBytes:20}}
  const wrapper=mount(ArchiveOutputSummary,{props:{output},global:{plugins:[createPinia()]}});expect(wrapper.text()).toContain('未新增');expect(wrapper.text()).toContain('快照');wrapper.unmount()
})
