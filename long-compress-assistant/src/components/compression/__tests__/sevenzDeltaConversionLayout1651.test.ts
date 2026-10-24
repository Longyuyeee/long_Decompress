import {it,expect} from 'vitest'
import {mount} from '@vue/test-utils'
import {createPinia} from 'pinia'
import Fields from '../ConversionSettingsFields.vue'
import Filter from '../SevenZipPreprocessField.vue'
import {defaultConversionSettings} from '@/utils/archiveConversionDraft'
it('conversion preprocessing in its own row keeps filter and distance update events',async()=>{
 const w=mount(Fields,{props:{settings:{...defaultConversionSettings(),format:'7z',sevenZipFilter:'delta',sevenZipDeltaDistance:256}},global:{plugins:[createPinia()],stubs:{ThemedSelect:true,HelpHint:true}}});const filter=w.getComponent(Filter);expect(filter.props('distance')).toBe(256);await filter.get('input').setValue('16');expect(w.emitted('update')?.at(-1)).toEqual([{sevenZipDeltaDistance:16}]);filter.vm.$emit('update:modelValue','none');expect(w.emitted('update')?.at(-1)).toEqual([{sevenZipFilter:'none'}]);w.unmount()
})
