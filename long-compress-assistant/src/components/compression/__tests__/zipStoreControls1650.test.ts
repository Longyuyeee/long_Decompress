import {it,expect} from 'vitest'
import {mount} from '@vue/test-utils'
import {createPinia} from 'pinia'
import ConversionFields from '../ConversionSettingsFields.vue'
import {defaultConversionSettings} from '@/utils/archiveConversionDraft'
it('conversion Store hides strength without changing saved level and restores it after switching back',async()=>{
 const settings={...defaultConversionSettings(),format:'zip',zipMethod:'store' as const,level:8}
 const w=mount(ConversionFields,{props:{settings},global:{plugins:[createPinia()],stubs:{ThemedSelect:true,HelpHint:true,ZipMethodField:true,SevenZipMethodField:true,WimMethodField:true}}})
 expect(w.find('[data-testid="conversion-level-control"]').exists()).toBe(false);expect(w.emitted('update')).toBeUndefined()
 await w.setProps({settings:{...settings,zipMethod:'deflate'}});expect((w.get('[data-testid="conversion-level-control"] input').element as HTMLInputElement).value).toBe('8');w.unmount()
})
