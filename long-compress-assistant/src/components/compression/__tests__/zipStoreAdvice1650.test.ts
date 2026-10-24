import {beforeEach,it,expect,vi} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia} from 'pinia'
import Card from '../CompressionAnalysisCard.vue'
import type {CompressionOptions} from '@/stores/compression'
const mocks=vi.hoisted(()=>({analyze:vi.fn()}))
vi.mock('@/composables/useTauriCommands',()=>({useTauriCommands:()=>({analyzeCompressionSources:mocks.analyze})}))
const settings:CompressionOptions={format:'zip',zipMethod:'deflate',zipEncryption:'zipcrypto',zipSplitMode:'standard',level:8,password:'keep-password',filename:'资料',splitArchive:true,splitSize:'2',keepStructure:true,deleteAfter:false,verifyAfter:true,createSolidArchive:true,encryptFilenames:true}
const advice={totalSize:65539,fileCount:1,sampledFiles:1,sampledBytes:65539,estimatedSize:66000,estimatedRatio:1.01,estimatedSecondsLow:1,estimatedSecondsHigh:3,confidence:'low',recommendedFormat:'zip',recommendedLevel:8,recommendedSolid:false,recommendedZipMethod:'store',lowValueBytes:0,lowValueFileCount:0,reasons:['内容抽样收益不足 2%']}
beforeEach(()=>{mocks.analyze.mockReset();mocks.analyze.mockResolvedValue(advice)})
const card=()=>mount(Card,{props:{jobId:'advice',paths:['C:/unknown.bin'],modelValue:{...settings}},global:{plugins:[createPinia()]}})
it('Store advice changes only the chosen method on explicit adoption and keeps password solid volumes and strength',async()=>{
 const w=card();await w.get('.analysis-button').trigger('click');await flushPromises()
 expect(w.find('[data-testid="zip-store-advice"]').exists()).toBe(true);expect(w.emitted('update:modelValue')).toBeUndefined()
 await w.get('.analysis-apply').trigger('click');const changed=w.emitted('update:modelValue')?.[0][0]
 expect(changed).toEqual({...settings,zipMethod:'store'});await w.setProps({modelValue:changed as CompressionOptions})
 expect(w.find('.analysis-stale').exists()).toBe(true);expect(w.find('.analysis-apply').exists()).toBe(false)
 mocks.analyze.mockResolvedValue({...advice,recommendedZipMethod:undefined});await w.get('.analysis-button').trigger('click');await flushPromises()
 expect(mocks.analyze).toHaveBeenLastCalledWith(expect.any(String),['C:/unknown.bin'],'zip-store',8);expect(w.find('.analysis-stale').exists()).toBe(false);expect(w.text()).toContain('仅存储不压缩数据');w.unmount()
})
it('a changed method makes prior Store advice stale and prevents applying it',async()=>{
 const w=card();await w.get('.analysis-button').trigger('click');await flushPromises();await w.setProps({modelValue:{...settings,zipMethod:'lzma'}})
 expect(w.find('.analysis-stale').exists()).toBe(true);expect(w.get('.analysis-apply').attributes('disabled')).toBeDefined();await w.get('.analysis-apply').trigger('click');expect(w.emitted('update:modelValue')).toBeUndefined();w.unmount()
})
