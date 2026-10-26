import {it,expect,vi} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia,setActivePinia} from 'pinia'
import ConversionSettingsFields from '../ConversionSettingsFields.vue'
import ConversionBatchConfig from '../ConversionBatchConfig.vue'
import {useArchiveConversionStore} from '@/stores/archiveConversion'
import {defaultConversionSettings,normalizeConversionSettings,conversionOutputPath} from '@/utils/archiveConversionDraft'
vi.mock('@tauri-apps/api/tauri',()=>({invoke:vi.fn(async()=>[])}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({ask:vi.fn()}))
it('lzop1662 conversion retains source password while removing unsupported target settings and controls',async()=>{
 const original={...defaultConversionSettings(),format:'7z',sourcePassword:'source-only',password:'target-only',splitMiB:2,encryptFilenames:true,outputPath:'D:/out.7z'}
 const settings=normalizeConversionSettings(original,{format:'lzo'})
 expect(settings).toMatchObject({format:'lzo',sourcePassword:'source-only',password:'',splitMiB:2,encryptFilenames:false,outputPath:'D:/out.lzo'})
 expect(original.password).toBe('target-only');expect(conversionOutputPath('C:/input.zip',settings)).toBe('D:/out.lzo')
 const wrapper=mount(ConversionSettingsFields,{props:{settings},global:{plugins:[createPinia()],stubs:{Teleport:true}}});await flushPromises()
 expect(wrapper.get('[data-testid="conversion-source-password"]').element).toHaveProperty('value','source-only')
 for(const id of ['conversion-password','conversion-level-control','conversion-hidden-names'])expect(wrapper.find(`[data-testid="${id}"]`).exists()).toBe(false)
 expect(wrapper.find('[data-testid="conversion-single-file-hint"]').exists()).toBe(false)
 expect(wrapper.emitted('update')).toBeUndefined();wrapper.unmount()
})
it('lzop1662 conversion modal drops hidden RAR recovery choices after switching to LZOP',async()=>{
 const pinia=createPinia();setActivePinia(pinia);const store=useArchiveConversionStore();store.addSources(['C:/one.zip'])
 const apply=vi.spyOn(store,'applySelected').mockResolvedValue('')
 const wrapper=mount(ConversionBatchConfig,{global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}});await flushPromises()
 await wrapper.get('[data-testid="conversion-batch-config"]').trigger('click')
 const choose=async(id:string,label:string)=>{await wrapper.get(`[data-testid="${id}"]`).trigger('click');await wrapper.findAll('[role="option"]').find(o=>o.find('strong').text()===label)!.trigger('click');await flushPromises()}
 await choose('conversion-batch-rar-recovery','3%');await choose('conversion-batch-rar-recovery-volumes','1 个 .rev 恢复卷')
 await choose('conversion-batch-format','LZOP（多文件，无目录）')
 expect(wrapper.find('[data-testid="conversion-batch-level"]').exists()).toBe(false)
 await wrapper.get('[data-testid="conversion-batch-apply"]').trigger('click');await flushPromises()
 expect(apply).toHaveBeenCalledWith({format:'lzo'});wrapper.unmount()
})
