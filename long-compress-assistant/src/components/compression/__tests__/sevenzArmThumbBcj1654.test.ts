import {it,expect,vi} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia} from 'pinia'
import Filter from '../SevenZipPreprocessField.vue'
import Card from '../CompressionAnalysisCard.vue'
import Fields from '../ConversionSettingsFields.vue'
import {defaultConversionSettings} from '@/utils/archiveConversionDraft'
import {useCompressionStore} from '@/stores/compression'
const mocks=vi.hoisted(()=>({analyze:vi.fn()}))
vi.mock('@/composables/useTauriCommands',()=>({useTauriCommands:()=>({analyzeCompressionSources:mocks.analyze})}))
it('BCJ stays in collapsed original settings, hides dormant Delta distance and retains it on switching',async()=>{
 const w=mount(Filter,{props:{format:'7z'},global:{stubs:{ThemedSelect:true,HelpHint:true}}});expect(w.get('details').attributes('open')).toBeUndefined();await w.setProps({modelValue:'bcj_arm_thumb',distance:256});expect(w.find('input').exists()).toBe(false);expect(w.text()).toContain('ARM Thumb BCJ');await w.setProps({method:'copy'});expect(w.text()).toContain('暂未应用');await w.setProps({method:'lzma',modelValue:'delta'});expect((w.get('input').element as HTMLInputElement).value).toBe('256');expect(w.emitted()).toEqual({});w.unmount()
})
it('analysis uses the real BCJ route and switching filters invalidates the old result',async()=>{
 mocks.analyze.mockResolvedValue({totalSize:65539,fileCount:1,sampledFiles:1,sampledBytes:65539,estimatedSize:20000,estimatedRatio:.3,estimatedSecondsLow:1,estimatedSecondsHigh:3,confidence:'low',recommendedFormat:'zip',recommendedLevel:6,recommendedSolid:false,lowValueBytes:0,lowValueFileCount:0,reasons:[]});const pinia=createPinia(),settings={...useCompressionStore(pinia).globalSettings,format:'7z' as const,sevenZipMethod:'lzma2' as const,sevenZipFilter:'bcj_arm_thumb' as const,sevenZipDeltaDistance:256};const w=mount(Card,{props:{jobId:'bcj',paths:['C:/app.exe'],modelValue:settings},global:{plugins:[pinia]}});await w.get('.analysis-button').trigger('click');await flushPromises();expect(mocks.analyze).toHaveBeenCalledWith(expect.any(String),['C:/app.exe'],'7z-lzma2-bcj-arm-thumb',6);await w.setProps({modelValue:{...settings,sevenZipFilter:'delta'}});expect(w.find('.analysis-stale').exists()).toBe(true);expect(w.get('.analysis-apply').attributes('disabled')).toBeDefined();w.unmount()
})
it('conversion preprocessing remains on its own row and emits BCJ without changing distance',async()=>{
 const w=mount(Fields,{props:{settings:{...defaultConversionSettings(),format:'7z',sevenZipFilter:'bcj_arm_thumb',sevenZipDeltaDistance:256}},global:{plugins:[createPinia()],stubs:{ThemedSelect:true,HelpHint:true}}});const filter=w.getComponent(Filter);expect(filter.props('modelValue')).toBe('bcj_arm_thumb');expect(filter.find('input').exists()).toBe(false);filter.vm.$emit('update:modelValue','delta');expect(w.emitted('update')?.at(-1)).toEqual([{sevenZipFilter:'delta'}]);expect(filter.props('distance')).toBe(256);w.unmount()
})
