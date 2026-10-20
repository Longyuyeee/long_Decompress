import {beforeEach,describe,expect,it,vi} from 'vitest'
import {flushPromises,mount} from '@vue/test-utils'
import {createPinia,setActivePinia} from 'pinia'
import ConversionSettingsFields from '../ConversionSettingsFields.vue'
import ConversionBatchConfig from '../ConversionBatchConfig.vue'
import ZipPasswordEncodingField from '@/components/ui/ZipPasswordEncodingField.vue'
import {defaultConversionSettings} from '@/utils/archiveConversionDraft'
import {useArchiveConversionStore} from '@/stores/archiveConversion'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
beforeEach(()=>{setActivePinia(createPinia());mocks.invoke.mockReset();mocks.invoke.mockImplementation(async cmd=>cmd==='load_app_settings'?'{}':{is_split:false,archive_format:'zip'})})
describe('conversion encoding controls',()=>{
  it('labels input and output independently, preserves input auto, and trusts detected format over suffix',async()=>{
    const wrapper=mount(ConversionSettingsFields,{props:{settings:{...defaultConversionSettings(),format:'zip',password:'new',sourceZipPasswordEncoding:'cp932',zipPasswordEncoding:'cp950'},sourcePath:'C:/old.7z',sourceFormat:'ZIP'},global:{plugins:[createPinia()],stubs:{Teleport:true}}})
    const fields=wrapper.findAllComponents(ZipPasswordEncodingField)
    expect(fields.map(f=>f.props('label'))).toEqual(['原包 ZIP 密码编码','新包 ZIP 密码编码'])
    fields[0].vm.$emit('update:modelValue',undefined)
    fields[1].vm.$emit('update:modelValue','cp936')
    expect(wrapper.emitted('update')).toEqual([[{sourceZipPasswordEncoding:undefined}],[{zipPasswordEncoding:'cp936'}]])
    await wrapper.setProps({sourcePath:'C:/old.zip',sourceFormat:'7Z'})
    expect(wrapper.findAllComponents(ZipPasswordEncodingField)).toHaveLength(1)
    expect(wrapper.findAllComponents(ZipPasswordEncodingField)[0].props('mode')).toBe('output')
    wrapper.unmount()
  })
  it('applies separate bulk input and output selections through the real dialog',async()=>{
    const pinia=createPinia();setActivePinia(pinia)
    const store=useArchiveConversionStore();store.addSources(['C:/old.zip'])
    store.updateItem(store.items[0].id,{format:'zip'})
    const wrapper=mount(ConversionBatchConfig,{global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}})
    await flushPromises();await wrapper.get('[data-testid="conversion-batch-config"]').trigger('click')
    for(const [selector,label] of [['conversion-batch-source-encoding','CP932'],['conversion-batch-target-encoding','CP950']]){
      await wrapper.get(`[data-testid="${selector}"]`).trigger('click')
      await wrapper.findAll('[role="option"]').find(o=>o.text().includes(label))!.trigger('click')
    }
    await wrapper.get('[data-testid="conversion-batch-apply"]').trigger('click');await flushPromises()
    expect(store.items[0].settings).toMatchObject({sourceZipPasswordEncoding:'cp932',zipPasswordEncoding:'cp950'})
    expect(wrapper.find('[data-testid="conversion-batch-fields"]').exists()).toBe(false)
    wrapper.unmount()
  })
})

it('wim1527 store only disables ineffective strength and native password without changing source credentials',()=>{
  const wrapper=mount(ConversionSettingsFields,{props:{settings:{...defaultConversionSettings(),format:'wim',sourcePassword:'原包密码',level:6,splitMiB:2},sourcePath:'C:/source.zip',sourceFormat:'ZIP'},global:{plugins:[createPinia()],stubs:{Teleport:true}}})
  expect(wrapper.get('input[type="range"]').attributes('disabled')).toBeDefined();expect(wrapper.get('[data-testid="conversion-password"]').attributes('disabled')).toBeDefined();expect(wrapper.get('[data-testid="conversion-source-password"]').element.value).toBe('原包密码');expect(wrapper.text()).toContain('仅存储与去重');expect(wrapper.emitted('update')).toBeUndefined();wrapper.unmount()
})
