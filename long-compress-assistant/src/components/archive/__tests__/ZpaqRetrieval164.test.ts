import {beforeEach,expect,it,vi} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia,setActivePinia} from 'pinia'
import ZpaqBlockDetails from '../ZpaqBlockDetails.vue'
import ZpaqRetrievalResult from '../ZpaqRetrievalResult.vue'
import {useTaskStore} from '@/stores/task'
import {copyZpaqRetrieval} from '@/types/zpaqRetrieval'
import {copyExecutionConfig} from '@/types/executionConfig'
import {createTaskHistoryRecord} from '@/types/taskHistory'
import {checkZpaqRetrieval} from '@/utils/zpaqRetrieval'
import type {ZpaqBlockReport} from '@/types/zpaqBlocks'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
const snapshot={version:3,date:20261020150000},selection={snapshot,revision:'a'.repeat(64),paths:['docs/good.txt']}
const report:ZpaqBlockReport={snapshot,revision:selection.revision,blocks:[],entries:[{path:'bad.txt',state:'affected',size:4,blocks:[]},{path:'unknown.txt',state:'unchecked',size:4,blocks:[]},{path:'docs/good.txt',state:'verified',size:4,blocks:[]},{path:'not-selected.txt',state:'verified',size:4,blocks:[]}]}
const global={stubs:{Modal:{props:['visible'],template:'<div v-if="visible"><slot/><slot name="footer"/></div>'},RouterLink:{template:'<a><slot/></a>'}}}
beforeEach(()=>{localStorage.clear();setActivePinia(createPinia());mocks.invoke.mockReset();mocks.invoke.mockImplementation(async cmd=>cmd==='load_app_settings'?'{}':null)})
it('zpaq164 only checked files can be selected and confirmation queues the exact subset without starting',async()=>{
 const w=mount(ZpaqBlockDetails,{props:{report:structuredClone(report),sourcePath:'C:/backup.zpaq',password:'session-164',encrypted:true},global})
 await w.get('[data-testid="zpaq-block-details-open"]').trigger('click');expect(w.get('[data-testid="zpaq-retrieval-select"]').attributes('disabled')).toBeDefined()
 await w.get('[data-testid="zpaq-block-filter"]').setValue('verified');await w.findAll('[data-testid="zpaq-retrieval-select"]')[0].setValue(true)
 await w.get('[data-testid="zpaq-retrieval-open"]').trigger('click');await flushPromises();await w.get('[data-testid="zpaq-retrieval-output"]').setValue('D:/retrieved');await w.get('[data-testid="zpaq-retrieval-confirm"]').trigger('click')
 const task=useTaskStore().tasks[0];expect(task).toMatchObject({status:'pending',zpaqRetrieval:selection,zpaqSnapshot:snapshot,selectedEntries:selection.paths,password:'session-164',recycleSourceAfterExtract:false,outputPath:'D:/retrieved'})
 const saved=createTaskHistoryRecord(task);expect(saved.executionConfig).toMatchObject({zpaqRetrieval:selection,zpaqSnapshot:snapshot});expect(JSON.stringify(saved)).not.toContain('session-164');expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='extract_file')).toBe(false);w.unmount()
})
it('zpaq164 execution copies retain immutable scope and reject missing or mismatched snapshots',()=>{
 const config={schemaVersion:1 as const,kind:'archive-extraction' as const,outputDirectory:'D:/out',extractToSubfolder:false,zpaqSnapshot:snapshot,zpaqRetrieval:structuredClone(selection)}
 const saved=copyExecutionConfig(config);config.zpaqRetrieval.paths.push('other.txt');expect(saved).toMatchObject({zpaqRetrieval:{paths:['docs/good.txt']}});expect(copyExecutionConfig({...config,zpaqSnapshot:undefined})).toBeNull();expect(copyExecutionConfig({...config,zpaqRetrieval:{...selection,paths:[]}})).toBeNull();expect(copyZpaqRetrieval({...selection,paths:['字'.repeat(21000)]})).toBeNull()
})
it('zpaq164 result details keep every exclusion reason and paginate the long receipt',async()=>{
 mocks.invoke.mockResolvedValue({schemaVersion:1,kind:'zpaq-partial-retrieval',source:'C:/backup.zpaq',snapshot,revision:selection.revision,entries:Array.from({length:44},(_,i)=>({path:`docs/file-${i}.txt`,size:i,state:i<2?'retrieved':i<4?'affected':i===4?'unchecked':'not_selected'}))})
 const w=mount(ZpaqRetrievalResult,{props:{path:'D:/actual-result',selection},global});await w.get('[data-testid="zpaq-retrieval-result-open"]').trigger('click');await flushPromises();expect(w.get('[data-testid="zpaq-retrieval-counts"]').text()).toContain('2 已取回');expect(w.get('[data-testid="zpaq-retrieval-counts"]').text()).toContain('2 受影响 · 1 未检查 · 39 未选择');expect(w.findAll('[role="listitem"]')).toHaveLength(20)
 await w.get('[data-testid="zpaq-retrieval-result-filter"]').setValue('unchecked');expect(w.findAll('[role="listitem"]')).toHaveLength(1);expect(w.get('[role="listitem"]').text()).toContain('未检查，未取回');w.unmount()
})
it('zpaq164 retry accepts the unchanged verified subset while rejecting a changed diagnosis',async()=>{
 mocks.invoke.mockResolvedValue({status:'zpaq_affected',zpaqBlocks:structuredClone(report)});await expect(checkZpaqRetrieval('C:/backup.zpaq','key',selection,'retry')).resolves.toBeUndefined()
 mocks.invoke.mockResolvedValue({status:'zpaq_affected',zpaqBlocks:{...report,revision:'b'.repeat(64)}});await expect(checkZpaqRetrieval('C:/backup.zpaq','key',selection,'retry')).rejects.toThrow('重新选择文件')
})
