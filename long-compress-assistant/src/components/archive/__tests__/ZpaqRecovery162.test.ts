import {beforeEach,expect,it,vi} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia,setActivePinia} from 'pinia'
import ZpaqRecoveryDialog from '../ZpaqRecoveryDialog.vue'
import {useTaskStore} from '@/stores/task'
import {usePendingDraftStore,PENDING_DRAFTS_KEY} from '@/stores/pendingDrafts'
import {createTaskHistoryRecord} from '@/types/taskHistory'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
const one={version:1,date:20261020130000,updates:3,deletes:0},two={version:2,date:20261020140000,updates:4,deletes:1}
beforeEach(()=>{setActivePinia(createPinia());localStorage.clear();mocks.invoke.mockReset();mocks.invoke.mockImplementation(async cmd=>cmd==='browse_archive'?{format:'ZPAQ',totalFiles:6,totalDirectories:3,entries:[]}:cmd==='get_file_info'?{isDir:false}:cmd==='load_app_settings'?'{}':cmd==='get_default_paths'?{home:'C:/',downloads:'C:/Downloads',desktop:'C:/Desktop'}:null)})
function create(){return mount(ZpaqRecoveryDialog,{props:{path:'C:/backup.zpaq',password:'session-only-162',history:{selected:two,snapshots:[one,two],interruptedTail:{offset:12345,date:20261022120000}},outputPath:'D:/restore'},global:{stubs:{RouterLink:{template:'<a><slot/></a>'},Modal:{props:['visible'],template:'<div v-if="visible"><slot/><slot name="footer"/></div>'}}}})}
it('zpaq162 explicitly queues the chosen complete snapshot and restores its scope without password or auto start',async()=>{
 const drafts=usePendingDraftStore();drafts.initialize();const w=create();await flushPromises()
 expect(useTaskStore().tasks).toHaveLength(0);await w.get('[data-testid="zpaq-recovery-version"]').setValue('1');await flushPromises();await w.get('[data-testid="zpaq-recovery-confirm"]').trigger('click')
 const task=useTaskStore().tasks[0];expect(task).toMatchObject({status:'pending',zpaqSnapshot:{version:1,date:one.date},extractToSubfolder:true,recycleSourceAfterExtract:false,configurationMode:'individual',outputPath:'D:/restore'});expect(task.name).toContain('不含中断更新')
 expect(createTaskHistoryRecord({...task,status:'completed'}).executionConfig).toMatchObject({kind:'archive-extraction',zpaqSnapshot:{version:1,date:one.date}})
 drafts.flush();expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('session-only-162');drafts.dispose();w.unmount()
 setActivePinia(createPinia());const restored=usePendingDraftStore();restored.initialize();await restored.restore();expect(useTaskStore().tasks[0]).toMatchObject({status:'pending',zpaqSnapshot:{version:1,date:one.date},draftPasswordRequired:true,outputPath:'D:/restore'});expect(useTaskStore().tasks[0].password).toBeUndefined();expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='extract_file')).toBe(false);restored.dispose()
})
it('zpaq162 failed reread cannot create a task from the previous snapshot',async()=>{
 const w=create();await flushPromises();mocks.invoke.mockImplementation(async cmd=>{if(cmd==='browse_archive')throw Error('所选快照已变化');return null});await w.get('[data-testid="zpaq-recovery-version"]').setValue('1');await flushPromises();expect(w.get('[data-testid="zpaq-recovery-error"]').text()).toContain('所选快照已变化');expect((w.get('[data-testid="zpaq-recovery-confirm"]').element as HTMLButtonElement).disabled).toBe(true);expect(useTaskStore().tasks).toHaveLength(0);w.unmount()
})
