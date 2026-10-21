import { beforeEach, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import CompressionPurposePicker from '../CompressionPurposePicker.vue'
import CompressionSettingsPanel from '../CompressionSettingsPanel.vue'
import ConversionSettingsFields from '../ConversionSettingsFields.vue'
import ConversionBatchConfig from '../ConversionBatchConfig.vue'
import BatchConfigBar from '@/components/tasks/BatchConfigBar.vue'
import { compressionPurposePatch, conversionPurposePatch } from '@/utils/compressionPurpose'
import { defaultConversionSettings } from '@/utils/archiveConversionDraft'
import { useCompressionStore, type CompressionOptions } from '@/stores/compression'
import { useArchiveConversionStore } from '@/stores/archiveConversion'

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn(async (cmd: string) => cmd === 'get_archive_engine_capabilities' ? {formats:[{name:'zip',canCreate:true,extensions:['zip']},{name:'7z',canCreate:true,extensions:['7z']}]} : undefined) }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
vi.mock('@tauri-apps/api/dialog', () => ({ open: vi.fn(), ask: vi.fn() }))
const options = (): CompressionOptions => ({format:'zip',level:6,password:'',filename:'保持名称',splitArchive:false,splitSize:'1024',keepStructure:true,deleteAfter:false,verifyAfter:true,createSolidArchive:false})
const global = () => ({ plugins:[createPinia()], stubs:{Teleport:true,Transition:false,ProfileSelector:true,ProfileManager:true,PasswordGeneratorDialog:true} })
const choose = async (wrapper: ReturnType<typeof mount>, id: string) => { await wrapper.get(`[data-testid="purpose-${id}"]`).trigger('click'); await wrapper.get('[data-testid="purpose-apply"]').trigger('click'); await flushPromises() }
beforeEach(() => { setActivePinia(createPinia()); localStorage.clear() })

it('1541 requires confirmation and a real password; reuse is explicit and reopening clears typed secrets', async () => {
  const wrapper = mount(CompressionPurposePicker)
  expect(wrapper.emitted('apply')).toBeUndefined()
  await wrapper.get('[data-testid="purpose-encrypted"]').trigger('click')
  expect(wrapper.get('[data-testid="purpose-apply"]').attributes('disabled')).toBeDefined()
  await wrapper.get('[data-testid="purpose-password"]').setValue('中文 密码!')
  await wrapper.get('[data-testid="purpose-apply"]').trigger('click')
  expect(wrapper.emitted('apply')?.[0]).toEqual([{format:'7z',level:6,password:'中文 密码!',encryptFilenames:true}])
  expect(wrapper.find('[data-testid="purpose-password"]').exists()).toBe(false)
  await wrapper.setProps({hasPassword:true}); await choose(wrapper,'encrypted')
  expect(wrapper.emitted('apply')?.at(-1)).toEqual([{format:'7z',level:6,encryptFilenames:true}])
  wrapper.unmount()
})

it('1541 keeps inherited compression values on open and applies only the chosen parameters', async () => {
  const original={...options(),password:'原密码',splitArchive:true,splitSize:'5',zipPasswordEncoding:'cp936' as const}
  const wrapper=mount(CompressionSettingsPanel,{props:{modelValue:original,outputPath:'D:/保持目录'},global:global()}); await flushPromises()
  expect(wrapper.emitted('update:modelValue')).toBeUndefined()
  await wrapper.get('[data-testid="purpose-smaller"]').trigger('click')
  expect(wrapper.emitted('update:modelValue')).toBeUndefined()
  await wrapper.get('[data-testid="purpose-apply"]').trigger('click'); await flushPromises()
  expect(wrapper.emitted('update:modelValue')?.at(-1)?.[0]).toEqual({...original,format:'7z',level:9,fastPack:false})
  expect(wrapper.emitted('update:outputPath')).toBeUndefined(); wrapper.unmount()
})

it('1541 validates split size and passes standard ZIP sizes into the existing conversion fields without changing input credentials or output', async () => {
  const settings={...defaultConversionSettings(),sourcePassword:'原包密码',sourceZipPasswordEncoding:'cp932' as const,outputPath:'D:/result.7z',password:'新包密码'}
  const wrapper=mount(ConversionSettingsFields,{props:{settings},global:global()})
  await wrapper.get('[data-testid="purpose-split"]').trigger('click')
  await wrapper.get('[data-testid="purpose-split-size"]').setValue('0')
  expect(wrapper.get('[data-testid="purpose-apply"]').attributes('disabled')).toBeDefined()
  await wrapper.get('[data-testid="purpose-split-size"]').setValue('25')
  await wrapper.get('[data-testid="purpose-apply"]').trigger('click')
  expect(wrapper.emitted('update')?.[0]).toEqual([{format:'zip',level:6,zipMethod:'deflate',encryptFilenames:false,splitMiB:25,zipSplitMode:'standard'}])
  expect(settings.sourcePassword).toBe('原包密码'); wrapper.unmount()
})

it('1541 changes only chosen compression drafts, preserves globals and filenames, and explicitly removes incompatible hidden names', () => {
  const store=useCompressionStore()
  for (const name of ['a','b','c']) store.addFile({path:'C:/'+name,name,size:1,type:'file',isDirectory:false})
  store.updateFileSettings('C:/a',{...options(),format:'7z',password:'old',encryptFilenames:true,splitArchive:true,splitSize:'4'})
  const globals=JSON.stringify(store.globalSettings), other=JSON.stringify(store.selectedFiles[2])
  expect(store.applyBatchConfiguration(['C:/a','C:/b'],compressionPurposePatch('share'))).toBe(2)
  expect(store.selectedFiles[0].settings).toMatchObject({format:'zip',password:'old',encryptFilenames:false,splitSize:'4',filename:'保持名称'})
  store.applyBatchConfiguration(['C:/b'],compressionPurposePatch('encrypted',{password:'new'}))
  expect(store.selectedFiles[1].settings).toMatchObject({format:'7z',password:'new',encryptFilenames:true})
  expect(JSON.stringify(store.globalSettings)).toBe(globals); expect(JSON.stringify(store.selectedFiles[2])).toBe(other)
  expect(()=>store.applyBatchConfiguration(['C:/a','C:/b'],{format:'zip',encryptFilenames:true})).toThrow('所选配置未修改')
})

it('1541 keeps multi-select configuration inside the original modal and cancellation discards its purpose patch', async () => {
  const wrapper=mount(BatchConfigBar,{props:{count:2,mode:'compression'},global:global()})
  expect(wrapper.find('[data-testid="purpose-picker"]').exists()).toBe(false)
  await wrapper.get('button[aria-haspopup="dialog"]').trigger('click'); await choose(wrapper,'smaller')
  expect(wrapper.emitted('apply')).toBeUndefined()
  await wrapper.get('[data-testid="batch-config-cancel"]').trigger('click')
  await wrapper.get('button[aria-haspopup="dialog"]').trigger('click')
  expect(wrapper.get('[data-testid="batch-config-apply"]').attributes('disabled')).toBeDefined()
  await wrapper.get('[data-testid="purpose-encrypted"]').trigger('click')
  await wrapper.get('[data-testid="purpose-password"]').setValue('同一密码')
  await wrapper.get('[data-testid="purpose-apply"]').trigger('click')
  await wrapper.get('[data-testid="batch-config-apply"]').trigger('click')
  expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({format:'7z',level:6,password:'同一密码',encryptFilenames:true})
  wrapper.unmount()
})

it('1541 converts using the same scheme, keeps source secrets and unselected inheritance, and applies defaults only to inherited drafts', async () => {
  const store=useArchiveConversionStore();store.addSources(['C:/a.zip','C:/b.zip']);store.pending[1].selected=false
  const item=store.pending[0];store.updateItem(item.id,{sourcePassword:'source',sourceZipPasswordEncoding:'cp936',password:'target',outputPath:'D:/a.7z'})
  await store.applySelected(conversionPurposePatch(compressionPurposePatch('split',{splitMiB:3})))
  expect(item.settings).toMatchObject({format:'zip',sourcePassword:'source',sourceZipPasswordEncoding:'cp936',password:'target',splitMiB:3,zipSplitMode:'standard',outputPath:'D:/a.zip'})
  store.updateDefaults(conversionPurposePatch(compressionPurposePatch('smaller')))
  expect(store.pending[1].settings).toBeUndefined(); expect(store.effectiveSettings(store.pending[1])).toMatchObject({format:'7z',level:9})
  expect(item.settings?.format).toBe('zip')
})

it('1541 fills conversion multi-select without touching tasks until the existing final confirmation', async () => {
  const pinia=createPinia();setActivePinia(pinia);const store=useArchiveConversionStore();store.addSources(['C:/a.zip','C:/b.zip'])
  const wrapper=mount(ConversionBatchConfig,{global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}})
  await wrapper.get('[data-testid="conversion-batch-config"]').trigger('click')
  await choose(wrapper,'split'); expect(store.pending.every(item=>!item.settings)).toBe(true)
  await wrapper.get('[data-testid="conversion-batch-apply"]').trigger('click');await flushPromises()
  expect(store.pending.every(item=>item.settings?.format==='zip'&&item.settings.splitMiB===1024&&item.settings.zipSplitMode==='standard')).toBe(true)
  wrapper.unmount()
})

it('1541 retains already filled password and split settings when another purpose adjusts only format and level', async () => {
  const wrapper=mount(BatchConfigBar,{props:{count:2,mode:'compression'},global:global()})
  await wrapper.get('button[aria-haspopup="dialog"]').trigger('click')
  await choose(wrapper,'split')
  await wrapper.get('[data-testid="purpose-encrypted"]').trigger('click')
  await wrapper.get('[data-testid="purpose-password"]').setValue('keep this password')
  await wrapper.get('[data-testid="purpose-apply"]').trigger('click')
  await choose(wrapper,'smaller')
  await wrapper.get('[data-testid="batch-config-apply"]').trigger('click')
  expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({format:'7z',level:9,password:'keep this password',encryptFilenames:true,splitArchive:true,splitSize:'1024',zipSplitMode:'standard'})
  wrapper.unmount()
})
