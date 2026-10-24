import {it,expect,vi} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia} from 'pinia'
import Filter from '../SevenZipPreprocessField.vue'
import Card from '../CompressionAnalysisCard.vue'
import {useCompressionStore} from '@/stores/compression'
const mocks=vi.hoisted(()=>({analyze:vi.fn()}))
vi.mock('@/composables/useTauriCommands',()=>({useTauriCommands:()=>({analyzeCompressionSources:mocks.analyze})}))
it('advanced preprocessing starts collapsed and shows distance only for Delta; inactive method retains it',async()=>{
 const w=mount(Filter,{props:{format:'7z'},global:{stubs:{ThemedSelect:true,HelpHint:true}}});expect(w.get('details').attributes('open')).toBeUndefined();expect(w.find('input').exists()).toBe(false);await w.setProps({modelValue:'delta',distance:256});expect((w.get('input').element as HTMLInputElement).value).toBe('256');await w.setProps({method:'bzip2'});expect(w.text()).toContain('暂未应用');expect(w.get('input').attributes('disabled')).toBeDefined();expect(w.emitted()).toEqual({});w.unmount()
})
it('analysis sends the selected Delta chain and distance changes invalidate the result',async()=>{
 mocks.analyze.mockResolvedValue({totalSize:65539,fileCount:1,sampledFiles:1,sampledBytes:65539,estimatedSize:20000,estimatedRatio:.3,estimatedSecondsLow:1,estimatedSecondsHigh:3,confidence:'low',recommendedFormat:'zip',recommendedLevel:6,recommendedSolid:false,lowValueBytes:0,lowValueFileCount:0,reasons:[]});const pinia=createPinia(),settings={...useCompressionStore(pinia).globalSettings,format:'7z' as const,sevenZipMethod:'lzma2' as const,sevenZipFilter:'delta' as const,sevenZipDeltaDistance:256};const w=mount(Card,{props:{jobId:'delta',paths:['C:/sample.pcm'],modelValue:settings},global:{plugins:[pinia]}});await w.get('.analysis-button').trigger('click');await flushPromises();expect(mocks.analyze).toHaveBeenCalledWith(expect.any(String),['C:/sample.pcm'],'7z-lzma2-delta-256',6);await w.setProps({modelValue:{...settings,sevenZipDeltaDistance:4}});expect(w.find('.analysis-stale').exists()).toBe(true);expect(w.get('.analysis-apply').attributes('disabled')).toBeDefined();w.unmount()
})
it('distance can be cleared and retyped without forcing the old default into the input',async()=>{
 const w=mount(Filter,{props:{format:'7z',modelValue:'delta',distance:4},global:{stubs:{ThemedSelect:true,HelpHint:true}}});await w.get('input').setValue('');await w.setProps({distance:undefined});expect((w.get('input').element as HTMLInputElement).value).toBe('');await w.get('input').setValue('256');expect(w.emitted('update:distance')?.at(-1)).toEqual([256]);w.unmount()
})
