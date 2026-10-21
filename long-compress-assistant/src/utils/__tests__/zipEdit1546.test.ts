import { afterEach,beforeEach,describe,expect,it,vi } from 'vitest'
import { createPinia,setActivePinia } from 'pinia'
import { mount,flushPromises } from '@vue/test-utils'
import { useZipEditStore,type ZipEditSession } from '@/stores/zipEdit'
import { useTaskStore } from '@/stores/task'
import { usePasswordStore } from '@/stores/password'
import { savedZipEdit,parseZipEdits } from '../zipEditDrafts'
import ZipEditBar from '@/components/archive/ZipEditBar.vue'
import ZipEditCredentialsPanel from '@/components/archive/ZipEditCredentialsPanel.vue'
import ZipEditRecoveryNotice from '@/components/archive/ZipEditRecoveryNotice.vue'
import type { ZipEditCatalog } from '@/types/zipEdit'
const mocks=vi.hoisted(()=>({invoke:vi.fn(),open:vi.fn(),browser:vi.fn(),success:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({open:mocks.open,save:vi.fn()}))
vi.mock('@/stores/app',()=>({useAppStore:()=>({t:(key:string)=>key,openArchiveInBrowser:mocks.browser,setSuccess:mocks.success})}))
const catalog:ZipEditCatalog={revision:'r1',methods:['Deflated'],entries:[{path:'docs/',isDir:true,size:0},{path:'docs/a.txt',isDir:false,size:3}],encrypted:true,defaultEncryption:'aes256',encryptions:{'docs/a.txt':'aes256'}}
let stored:string|null=null,issues:Array<{index:number;issue:string}>=[],receipt:any=null,sourceCatalog=catalog
const stores:ReturnType<typeof useZipEditStore>[]=[]
const create=()=>{const store=useZipEditStore();stores.push(store);return store}
const snapshot=(source='C:/source.zip')=>({sourcePath:source,catalog:structuredClone(catalog),drafts:[{id:'input1',batch:'batch1',order:0,choice:'choose',sourcePath:'C:/external.txt',targetPath:'new.txt',isDir:false,size:4,sourceRevision:'external1'}],operations:[{id:'rename1',order:1,kind:'rename',targets:[{id:'original:docs/',path:'manual/'}]}],method:'deflate',level:6,outputPath:'C:/edited.zip',saves:0,sourceEncoding:'cp936',outputMode:'aes256',outputEncoding:'cp950',savedAt:'2026-10-21'} as any)
const output={primaryPath:'C:/edited.zip',files:[{path:'C:/edited.zip',bytes:30}],totalBytes:30,split:false}
function native(command:string,args:any){
 if(command==='load_zip_edit_drafts')return stored
 if(command==='save_zip_edit_drafts'){stored=args.data;return}
 if(command==='inspect_zip_edit')return structuredClone(sourceCatalog)
 if(command==='check_zip_edit_inputs')return issues
 if(command==='zip_edit_task_receipt')return receipt
 if(command==='prepare_zip_edit_inputs')return[{sourcePath:args.sources[0],targetPath:'relocated.txt',size:8,isDir:false,sourceRevision:'external2'}]
 if(command==='save_zip_edit')return output
 if(command==='load_app_settings')return'{}'
}
beforeEach(()=>{setActivePinia(createPinia());stored=null;issues=[];receipt=null;sourceCatalog=catalog;mocks.invoke.mockReset();mocks.invoke.mockImplementation(async(command,args)=>native(command,args));mocks.open.mockReset();mocks.browser.mockReset();mocks.success.mockReset()})
afterEach(async()=>{for(const store of stores.splice(0))store.dispose();await flushPromises()})
describe('zipEdit1546 credentials and recovery',()=>{
 it('serializes only editable references and settings, never passwords or transient execution state',async()=>{
  const store=create();await store.begin('C:/source.zip');const session=store.sessionFor('C:/source.zip')!;Object.assign(session,snapshot(),{sourcePassword:'source-secret',outputPassword:'output-secret',busy:true,error:'runtime error',inputIssues:{input1:'missing'}})
  const saved=savedZipEdit(session),raw=JSON.stringify({version:1,sessions:[saved]});expect(raw).not.toContain('source-secret');expect(raw).not.toContain('output-secret');expect(raw).not.toContain('runtime error');expect(parseZipEdits(raw)[0]).toMatchObject({outputMode:'aes256',outputEncoding:'cp950',sourceEncoding:'cp936'});expect(()=>parseZipEdits('{"version":1,"sessions":[{}]}')).toThrow()
 })
 it('preserves unopened drafts while saving new work in this session',async()=>{
  stored=JSON.stringify({version:1,sessions:[snapshot('C:/older.zip')]});const store=create();await store.initialize();await store.begin('C:/new.zip');const session=store.sessionFor('C:/new.zip')!;session.operations=snapshot().operations;session.sourcePassword='not-saved';await store.flush();const rows=parseZipEdits(stored);expect(rows.map(row=>row.sourcePath)).toEqual(['C:/older.zip','C:/new.zip']);expect(stored).not.toContain('not-saved')
 })
 it('restores edits without executing, clears credentials and exposes changed external references',async()=>{
  stored=JSON.stringify({version:1,sessions:[snapshot()]});issues=[{index:0,issue:'来源已变化'}];const store=create();await store.initialize();expect(await store.restoreDraft('C:/source.zip')).toEqual({path:'C:/source.zip'});const session=store.sessionFor('C:/source.zip')!;expect(session.sourcePassword).toBe('');expect(session.outputPassword).toBe('');expect(session.outputMode).toBe('aes256');expect(session.inputIssues.input1).toContain('变化');expect(store.plan(session).nodes.some(node=>node.path==='manual/a.txt')).toBe(true);expect(mocks.invoke.mock.calls.some(call=>call[0]==='save_zip_edit')).toBe(false)
 })
 it('keeps a changed original blocked until the user explicitly reads and accepts it',async()=>{
  stored=JSON.stringify({version:1,sessions:[snapshot()]});sourceCatalog={...catalog,revision:'r2'};const store=create();await store.initialize();await store.restoreDraft('C:/source.zip');const session=store.sessionFor('C:/source.zip')!;expect(session.sourceIssue).toContain('原包已变化');expect(session.catalog.revision).toBe('r1');await store.refresh(session.sourcePath);expect(session.sourceIssue).toBe('');expect(session.catalog.revision).toBe('r2');expect(session.operations).toHaveLength(1)
 })
 it('relinks an external file without changing its target, identity or structural dependencies',async()=>{
  stored=JSON.stringify({version:1,sessions:[snapshot()]});const store=create();await store.initialize();await store.restoreDraft('C:/source.zip');await store.relinkInput('C:/source.zip','input1','D:/found.txt');const draft=store.sessionFor('C:/source.zip')!.drafts[0];expect(draft).toMatchObject({id:'input1',targetPath:'new.txt',sourcePath:'D:/found.txt',sourceRevision:'external2',size:8});expect(store.sessionFor('C:/source.zip')!.operations).toHaveLength(1)
 })
 it('relinks a directory using only planned references and does not add newly appeared files',async()=>{
  const saved=snapshot();saved.drafts=[{...saved.drafts[0],id:'folder',sourcePath:'C:/old',targetPath:'incoming/',isDir:true,size:0,sourceRevision:'directory'},{...saved.drafts[0],id:'child',sourcePath:'C:/old/file.txt',targetPath:'incoming/file.txt'}];stored=JSON.stringify({version:1,sessions:[saved]});const store=create();await store.initialize();await store.restoreDraft(saved.sourcePath)
  mocks.invoke.mockImplementation(async(command,args)=>command==='prepare_zip_edit_inputs'?[{sourcePath:'D:/new',targetPath:'new/',isDir:true,size:0,sourceRevision:'directory'},{sourcePath:'D:/new/file.txt',targetPath:'new/file.txt',isDir:false,size:8,sourceRevision:'updated'},{sourcePath:'D:/new/unplanned.txt',targetPath:'new/unplanned.txt',isDir:false,size:2,sourceRevision:'extra'}]:native(command,args))
  await store.relinkInput(saved.sourcePath,'folder','D:/new');expect(store.sessionFor(saved.sourcePath)!.drafts).toHaveLength(2);expect(store.sessionFor(saved.sourcePath)!.drafts[1]).toMatchObject({id:'child',sourcePath:'D:/new/file.txt',targetPath:'incoming/file.txt',sourceRevision:'updated'})
 })
 it('relocates the original and keeps all edits under its new source identity',async()=>{
  stored=JSON.stringify({version:1,sessions:[snapshot()]});const store=create();await store.initialize();await store.restoreDraft('C:/source.zip');await store.relocateSource('C:/source.zip','D:/found.zip');expect(store.sessionFor('C:/source.zip')).toBeUndefined();expect(store.sessionFor('D:/found.zip')?.operations).toHaveLength(1);expect(parseZipEdits(stored)[0].sourcePath).toBe('D:/found.zip')
 })
 it('requires source and target credentials before submitting a restored encrypted save',async()=>{
  stored=JSON.stringify({version:1,sessions:[snapshot()]});const store=create();await store.initialize();await store.restoreDraft('C:/source.zip');const session=store.sessionFor('C:/source.zip')!;await store.save(session.sourcePath);expect(session.error).toContain('原包密码');session.sourcePassword='source';await store.save(session.sourcePath);expect(session.error).toContain('新包密码');expect(mocks.invoke.mock.calls.some(call=>call[0]==='save_zip_edit')).toBe(false)
 })
 it('supports password-only save and passes the selected credentials to the new-result reader',async()=>{
  const store=create();await store.begin('C:/source.zip',{password:'source',encoding:'cp936'});const session=store.sessionFor('C:/source.zip')!;session.outputMode='aes256';session.outputPassword='target';session.outputEncoding='cp950';await store.save(session.sourcePath);expect(store.taskFor(session)?.status).toBe('completed');expect(store.resultReadOptions(session)).toEqual({password:'target',zipPasswordEncoding:'cp950'});const args=mocks.invoke.mock.calls.find(call=>call[0]==='save_zip_edit')![1];expect(args.originalEdits).toEqual([]);expect(args.changes).toEqual([]);expect(args.credentials).toMatchObject({sourcePassword:'source',sourceEncoding:'cp936',outputMode:'aes256',outputPassword:'target',outputEncoding:'cp950'})
 })
 it('recognizes an already completed task instead of replaying a stale draft',async()=>{
  const saved={...snapshot(),taskId:'already-done'};stored=JSON.stringify({version:1,sessions:[saved]});receipt={status:'completed',output};const store=create();await store.initialize();expect(await store.restoreDraft(saved.sourcePath)).toEqual({published:output.primaryPath});expect(store.sessionFor(saved.sourcePath)).toBeUndefined();expect(store.draftCount).toBe(0);expect(mocks.invoke.mock.calls.some(call=>call[0]==='save_zip_edit')).toBe(false);expect(stored).toBeNull()
 })
 it('links a resumed failed draft to its persisted original without rewriting that record',async()=>{
  const saved={...snapshot(),taskId:'old-failure'};stored=JSON.stringify({version:1,sessions:[saved]});receipt={status:'failed'};const store=create();await store.initialize();await store.restoreDraft(saved.sourcePath);const session=store.sessionFor(saved.sourcePath)!;session.sourcePassword='source';session.outputPassword='target';await store.save(saved.sourcePath);expect(store.taskFor(session)?.retryOfTaskId).toBe('old-failure');expect(mocks.invoke.mock.calls.filter(call=>call[0]==='save_task_history').every(call=>call[1].record.id!=='old-failure')).toBe(true)
 })
 it('keeps pending edits and reports draft write errors',async()=>{
  const store=create();await store.initialize();await store.begin('C:/source.zip');store.sessionFor('C:/source.zip')!.operations=snapshot().operations;mocks.invoke.mockImplementation(async(command,args)=>{if(command==='save_zip_edit_drafts')throw new Error('disk unavailable');return native(command,args)});expect(await store.flush()).toBe(false);expect(store.saveError).toContain('无法保存');expect(store.sessionFor('C:/source.zip')!.operations).toHaveLength(1)
 })
 it('chooses source credentials from the existing vault without copying them into drafts',async()=>{
  const pinia=createPinia();setActivePinia(pinia);const store=create();await store.begin('C:/source.zip');const vault=usePasswordStore(),entry:any={id:'vault1',name:'原包线索',password:'vault-secret',tags:[],favorite:true};vault.entries=[entry];vault.isInitialized=true;vault.isUnlocked=true;mocks.invoke.mockImplementation(async(command,args)=>command==='increment_encrypted_password_use_count'?entry:command==='list_encrypted_passwords'?[entry]:native(command,args))
  const wrapper=mount(ZipEditCredentialsPanel,{props:{sourcePath:'C:/source.zip'},global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}});await wrapper.get('[data-testid="zip-edit-source-vault"]').trigger('click');await flushPromises();await wrapper.get('[data-vault-entry="原包线索"]').trigger('click');await flushPromises();expect(store.sessionFor('C:/source.zip')!.sourcePassword).toBe('vault-secret');expect(JSON.stringify(savedZipEdit(store.sessionFor('C:/source.zip')!))).not.toContain('vault-secret');wrapper.unmount()
 })
 it('shows source correction inside the review and opens recovered edits without automatically saving',async()=>{
  const pinia=createPinia();setActivePinia(pinia);stored=JSON.stringify({version:1,sessions:[snapshot()]});const store=create();await store.initialize();store.recoveryVisible=true
  const notice=mount(ZipEditRecoveryNotice,{global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}});await notice.get('[data-testid="zip-edit-recover-one"]').trigger('click');await flushPromises();expect(mocks.browser).toHaveBeenCalled();const session=store.sessionFor('C:/source.zip')!;session.sourceIssue='原包已变化'
  const wrapper=mount(ZipEditBar,{props:{sourcePath:session.sourcePath,directory:''},global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}});await wrapper.get('[data-testid="zip-edit-review"]').trigger('click');expect(wrapper.get('[data-testid="zip-edit-source-status"]').text()).toContain('原包已变化');expect(wrapper.get('[data-testid="zip-edit-credentials"]').exists()).toBe(true);expect(mocks.invoke.mock.calls.some(call=>call[0]==='save_zip_edit')).toBe(false);wrapper.unmount();notice.unmount()
 })
})
