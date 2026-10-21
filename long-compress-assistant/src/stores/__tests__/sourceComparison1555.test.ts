import {beforeEach,it,expect,vi} from 'vitest'
import {createPinia,setActivePinia} from 'pinia'
import {flushPromises,mount} from '@vue/test-utils'
import {useSourceComparisonStore} from '../sourceComparison'
import {useTaskStore} from '../task'
import {copyComparisonSelection,type ComparisonSource,type ContentVerdict} from '@/types/sourceComparison'
import {createTaskHistoryRecord} from '@/types/taskHistory'
import SourceComparisonView from '@/views/SourceComparisonView.vue'
const mocks=vi.hoisted(()=>({invoke:vi.fn(),listeners:new Map<string,(event:any)=>void>(),push:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async(event,callback)=>{mocks.listeners.set(event,callback);return vi.fn()})}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
vi.mock('vue-router',()=>({useRouter:()=>({push:mocks.push})}))
const source=(path:string,zip=false):ComparisonSource=>({name:path.split('/').pop()!,source:{path,kind:zip?'zip':'directory',revision:'a'.repeat(64)},entries:['one','two'].map(path=>({path,size:4,modified:'same-time',directory:false,selectable:true}))})
let store:ReturnType<typeof useSourceComparisonStore>
beforeEach(()=>{setActivePinia(createPinia());vi.clearAllMocks();mocks.listeners.clear();mocks.invoke.mockResolvedValue(undefined);store=useSourceComparisonStore();store.sides.left.source=source('C:/left');store.sides.left.path='C:/left';store.sides.right.source=source('C:/right.zip',true);store.sides.right.path='C:/right.zip';store.options.outputDirectory='D:/out'})
it('comparison1555 applies actual verdicts and filters pending pairs without claiming directories equal',async()=>{
 mocks.invoke.mockResolvedValueOnce({entries:[{path:'one',kind:'same',reason:'full contents'}],stopped:false});await store.verify(['one']);expect(store.rows.find(row=>row.path==='one')!.kind).toBe('same');store.filter='unverified';expect(store.filtered.map(row=>row.path)).toEqual(['two']);store.filter='same';expect(store.filtered.map(row=>row.path)).toEqual(['one']);expect(useTaskStore().tasks).toHaveLength(0)
})
it('comparison1555 stopping keeps complete results and next verification reads only remaining pairs',async()=>{
 await store.attach();let complete!:(value:any)=>void;mocks.invoke.mockImplementation(command=>command==='comparison_verify'?new Promise(resolve=>complete=resolve):Promise.resolve());const first=store.verifyFiltered();await flushPromises();const jobId=store.verifyJob;mocks.listeners.get('comparison-content-progress')!({payload:{jobId,progress:{processedBytes:8,totalBytes:16,completed:1,total:2,path:'one',side:'right'}}});const entry:ContentVerdict={path:'one',kind:'same',reason:'full read'};mocks.listeners.get('comparison-content-result')!({payload:{jobId,entry}});expect(store.verifyProgress.processedBytes).toBe(8);await store.stopVerification();expect(mocks.invoke).toHaveBeenCalledWith('library_cancel',{jobId});complete({entries:[entry],stopped:true});await first;expect(store.verdicts.one.kind).toBe('same');expect(store.verifyNote).toContain('已停止');mocks.invoke.mockResolvedValueOnce({entries:[{path:'two',kind:'different',reason:'bytes'}],stopped:false});await store.verifyFiltered();expect(mocks.invoke.mock.calls.filter(([command])=>command==='comparison_verify').at(-1)![1].request.paths).toEqual(['two'])
})
it('comparison1555 a changed source revokes old verdicts and late obsolete events cannot restore them',async()=>{
 await store.attach();store.verdicts.one={path:'one',kind:'same',reason:'old'};let oldJob='';mocks.invoke.mockImplementation(async(command,args)=>{if(command==='comparison_verify'){oldJob=args.jobId;throw Error('[comparison:changed] 来源已变化')} });await store.verify(['two']);expect(store.verdicts).toEqual({});mocks.listeners.get('comparison-content-result')!({payload:{jobId:oldJob,entry:{path:'two',kind:'same'}}});expect(store.verdicts).toEqual({});expect(store.error).toContain('来源已变化')
})
it('comparison1555 large verification scope is explained and single confirmed rows can be rechecked',async()=>{
 store.sides.left.source!.entries=Array.from({length:2001},(_,i)=>({path:String(i),size:1,modified:null,directory:false,selectable:true}));store.sides.right.source!.entries=store.sides.left.source!.entries;await store.verifyFiltered();expect(mocks.invoke).not.toHaveBeenCalled();expect(store.error).toContain('2000');store.verdicts['0']={path:'0',kind:'same',reason:'old'};mocks.invoke.mockResolvedValueOnce({entries:[{path:'0',kind:'different',reason:'new'}],stopped:false});await store.verify(['0'],true);expect(store.verdicts['0'].kind).toBe('different')
})
it('comparison1555 password and encoding enter runtime but only exact scope and encoding enter history',async()=>{
 store.sides.right.source!.entries[0].encrypted=true;store.sides.right.source!.source.encrypted=true;store.sides.right.password='private-1555';store.sides.right.encoding='cp936';store.toggle('right','one');await store.queue();const task=useTaskStore().tasks[0];expect(task).toMatchObject({password:'private-1555',comparisonSelection:{source:{encrypted:true},paths:['one'],zipPasswordEncoding:'cp936'}});expect(mocks.invoke).toHaveBeenCalledWith('comparison_validate',expect.objectContaining({password:'private-1555'}));task.status='completed';const saved=createTaskHistoryRecord(task);expect(JSON.stringify(saved)).not.toContain('private-1555');expect(saved.executionConfig).toMatchObject({comparisonSelection:{zipPasswordEncoding:'cp936',source:{encrypted:true}}});expect(copyComparisonSelection({...task.comparisonSelection!,zipPasswordEncoding:'bad' as any})).toBeNull()
})
it('comparison1555 re-reading the same source keeps valid paths and credentials but revokes conclusions',async()=>{
 store.toggle('left','one');store.sides.left.password='session-only';store.verdicts.one={path:'one',kind:'same',reason:'old'};mocks.invoke.mockImplementation(async command=>command==='comparison_read'?source('C:/left'):undefined);await store.read('left','C:/left');expect(store.selected.left.has('one')).toBe(true);expect(store.sides.left.password).toBe('session-only');expect(store.verdicts).toEqual({});mocks.invoke.mockResolvedValueOnce(source('C:/other'));await store.read('left','C:/other');expect(store.selected.left.size).toBe(0);expect(store.sides.left.password).toBe('')
})
it('comparison1555 failed password side stays selected and only corrected remaining paths are later queued',async()=>{
 store.sides.right.source!.entries[0].encrypted=true;store.toggle('left','one');store.toggle('right','one');mocks.invoke.mockImplementation(async(command,args)=>{if(command==='comparison_validate'&&args.request.source.kind==='zip'&&!args.password)throw Error('需要密码')});await store.queue();expect(useTaskStore().tasks).toHaveLength(1);expect(store.selected.right.has('one')).toBe(true);store.sides.right.password='fixed';await store.queue();expect(useTaskStore().tasks).toHaveLength(2);expect(store.selectedCount).toBe(0)
})
it('comparison1555 new controls keep inline details and use a source-password dialog',async()=>{
 const view=mount(SourceComparisonView,{global:{stubs:{ComparisonRetrievalDialog:true}}});await view.get('[data-comparison-expand="one"]').trigger('click');expect(view.find('[data-comparison-details="one"]').exists()).toBe(true);expect(view.find('[data-comparison-verify="one"]').exists()).toBe(true);await view.get('[data-comparison-password="right"]').trigger('click');await flushPromises();expect(document.querySelector('[data-testid="comparison-password-right"]')).not.toBeNull();view.unmount()
})

it('comparison1555 stopping during initial source reading preserves earlier confirmed files',async()=>{
 store.verdicts.one={path:'one',kind:'same',reason:'prior full read'};mocks.invoke.mockRejectedValueOnce(Error('资料对比或取回已停止'));await store.verify(['two']);expect(store.verdicts.one.kind).toBe('same');expect(store.verifyNote).toContain('保留已完成项')
})

it('comparison1555 top change and pending counts match the corresponding content filters',()=>{
 store.sides.left.source!.entries.push({path:'only-left',size:1,modified:null,directory:false,selectable:true});store.verdicts.one={path:'one',kind:'different',reason:'actual bytes'};store.verdicts.two={path:'two',kind:'unreadable',reason:'needs password'};expect(store.changedCount).toBe(1);store.filter='changes';expect(store.filtered.map(row=>row.path)).toEqual(['one']);expect(store.unverifiedCount).toBe(1);store.filter='unverified';expect(store.filtered.map(row=>row.path)).toEqual(['two'])
})
