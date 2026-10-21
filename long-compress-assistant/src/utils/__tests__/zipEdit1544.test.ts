import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { resolveZipEdits } from '../zipEditPlan'
import { copyExecutionConfig } from '@/types/executionConfig'
import { useZipEditStore } from '@/stores/zipEdit'
import { useTaskStore } from '@/stores/task'
import ZipEditBar from '@/components/archive/ZipEditBar.vue'
import type { ZipEditCatalog, ZipEditDraft } from '@/types/zipEdit'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn(),save:vi.fn()}))
const catalog:ZipEditCatalog={revision:'source-revision',methods:['Deflated'],entries:[{path:'docs/',isDir:true,size:0},{path:'docs/a.txt',isDir:false,size:3},{path:'folder',isDir:false,size:2}]}
const draft=(targetPath:string,extra:Partial<ZipEditDraft>={}):ZipEditDraft=>({id:targetPath,batch:'batch',sourcePath:'C:/'+targetPath,targetPath,isDir:targetPath.endsWith('/'),size:targetPath.endsWith('/')?0:4,sourceRevision:'input-revision',choice:'choose',...extra})
beforeEach(()=>{setActivePinia(createPinia());mocks.invoke.mockReset();mocks.invoke.mockImplementation(async command=>command==='inspect_zip_edit'?structuredClone(catalog):command==='load_app_settings'?'{}':undefined)})
describe('zipEdit1544',()=>{
  it('requires explicit decisions for collisions and keeps unchosen entries out of save',()=>{
    const rows=[draft('docs/a.txt'),draft('docs/new.txt')];let result=resolveZipEdits(catalog,rows)
    expect(result.unresolved).toBe(1);expect(result.changes.map(c=>c.targetPath)).toEqual(['docs/new.txt'])
    rows[0].choice='replace';result=resolveZipEdits(catalog,rows);expect(result.unresolved).toBe(0);expect(result.changes[0].action).toBe('replace')
    rows[0].choice='skip';const skipped=resolveZipEdits(catalog,rows);expect(skipped.changes).toHaveLength(1);expect(skipped.rows[0].collision).toBe('file')
  })
  it('keeps both file copies with a free name and reverts to the original after undo',()=>{
    const row=draft('docs/a.txt',{choice:'keep-both'});expect(resolveZipEdits(catalog,[row]).changes[0]).toMatchObject({targetPath:'docs/a (2).txt',action:'add'})
    expect(resolveZipEdits(catalog,[]).changes).toEqual([])
  })
  it('renames or skips a colliding directory together with all descendants',()=>{
    const rows=[draft('folder/'),draft('folder/child.txt'),draft('folder/empty/')]
    expect(resolveZipEdits(catalog,rows).unresolved).toBe(3)
    rows[0].choice='keep-both';let result=resolveZipEdits(catalog,rows);expect(result.unresolved).toBe(0);expect(result.changes.map(c=>c.targetPath)).toEqual(['folder (2)/','folder (2)/child.txt','folder (2)/empty/'])
    rows[0].choice='skip';result=resolveZipEdits(catalog,rows);expect(result.changes).toEqual([]);expect(result.rows.every(r=>r.status==='skip')).toBe(true)
  })
  it('merges a directory with its original spelling and asks about contained files',()=>{
    const result=resolveZipEdits(catalog,[draft('Docs/'),draft('Docs/a.txt'),draft('Docs/new.txt')]);expect(result.rows[0].status).toBe('merge');expect(result.rows[1].target).toBe('docs/a.txt');expect(result.unresolved).toBe(1);expect(result.changes[0].targetPath).toBe('docs/new.txt')
  })
  it('replacement of an earlier addition remains one add and can be undone independently',()=>{
    const first=draft('new.txt'),second=draft('new.txt',{id:'second',batch:'second',sourcePath:'D:/new.txt',choice:'replace'})
    let result=resolveZipEdits(catalog,[first,second]);expect(result.changes).toHaveLength(1);expect(result.changes[0]).toMatchObject({sourcePath:'D:/new.txt',action:'add'});expect(result.rows[0].status).toBe('superseded')
    result=resolveZipEdits(catalog,[first]);expect(result.changes[0].sourcePath).toBe('C:/new.txt')
  })
  it('persists only the edit result identity and never an executable edit list',()=>{
    expect(copyExecutionConfig({schemaVersion:1,kind:'archive-edit',outputPath:'D:/edited.zip',changes:['a'],password:'secret'} as any)).toEqual({schemaVersion:1,kind:'archive-edit',outputPath:'D:/edited.zip'})
  })
  it('adds metadata without saving, ignores exact duplicates and removes a directory as a group',async()=>{
    const store=useZipEditStore();await store.begin('C:/source.zip');mocks.invoke.mockImplementation(async command=>command==='prepare_zip_edit_inputs'?[draft('new/'),draft('new/file.txt')]:undefined)
    await store.add('C:/source.zip',['C:/new'],'');await store.add('C:/source.zip',['C:/new'],'');const session=store.sessionFor('C:/source.zip')!
    expect(session.drafts).toHaveLength(2);expect(mocks.invoke.mock.calls.some(c=>c[0]==='save_zip_edit')).toBe(false)
    store.remove(session.sourcePath,session.drafts[0].id);expect(session.drafts).toHaveLength(0)
  })
  it('keeps edits and the old failure when retrying and clears edits only after publication succeeds',async()=>{
    const store=useZipEditStore();await store.begin('C:/source.zip');const session=store.sessionFor('C:/source.zip')!;session.drafts=[draft('new.txt')]
    let attempts=0;mocks.invoke.mockImplementation(async command=>{if(command==='save_zip_edit'){if(++attempts===1)throw new Error('输出位置已存在');return{primaryPath:'C:/new.zip',files:[{path:'C:/new.zip',bytes:20}],totalBytes:20,split:false}}})
    await store.save(session.sourcePath);expect(session.drafts).toHaveLength(1);expect(store.taskFor(session)!.status).toBe('failed');const original=structuredClone(JSON.parse(JSON.stringify(store.taskFor(session))))
    await store.save(session.sourcePath);expect(session.drafts).toHaveLength(0);expect(store.taskFor(session)!.status).toBe('completed');expect(store.taskFor(session)!.retryOfTaskId).toBe(original.id)
    expect(JSON.parse(JSON.stringify(useTaskStore().tasks.find(t=>t.id===original.id)))).toEqual(original)
  })
  it('waits for native edit cancellation to settle and retains pending modifications',async()=>{
    const store=useZipEditStore();await store.begin('C:/source.zip');const session=store.sessionFor('C:/source.zip')!;session.drafts=[draft('new.txt')]
    let rejectSave!:(e:Error)=>void;mocks.invoke.mockImplementation(async command=>command==='save_zip_edit'?new Promise((_,reject)=>{rejectSave=reject}):undefined)
    const saving=store.save(session.sourcePath);await flushPromises();await store.cancel(session.sourcePath);expect(store.taskFor(session)!.status).toBe('cancelling');expect(session.busy).toBe(true)
    rejectSave(new Error('ZIP_EDIT_CANCELLED'));await saving;expect(store.taskFor(session)!.status).toBe('cancelled');expect(session.drafts).toHaveLength(1);expect(session.busy).toBe(false)
  })
  it('places pending configuration in the original modal and blocks save until collisions are decided',async()=>{
    const pinia=createPinia();setActivePinia(pinia);const store=useZipEditStore();await store.begin('C:/source.zip');const session=store.sessionFor('C:/source.zip')!;session.drafts=[draft('docs/a.txt')]
    const wrapper=mount(ZipEditBar,{props:{sourcePath:'C:/source.zip',directory:'docs'},global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}})
    expect(wrapper.find('[data-testid="zip-edit-output"]').exists()).toBe(false);await wrapper.get('[data-testid="zip-edit-review"]').trigger('click');expect(wrapper.get('[data-testid="zip-edit-save"]').attributes('disabled')).toBeDefined()
    await wrapper.get('[data-testid="zip-edit-choice-skip"]').trigger('click');expect(wrapper.find('[data-testid="zip-edit-choice-keep-both"]').exists()).toBe(true);await wrapper.get('[data-testid="zip-edit-choice-keep-both"]').trigger('click');expect(wrapper.get('[data-testid="zip-edit-change"]').text()).toContain('docs/a (2).txt');expect(wrapper.get('[data-testid="zip-edit-save"]').attributes('disabled')).toBeUndefined();wrapper.unmount()
  })
})

it('zipEdit1544 completed results use one compact panel without editable pending settings',async()=>{
 const pinia=createPinia();setActivePinia(pinia);const store=useZipEditStore();await store.begin('C:/source.zip');const session=store.sessionFor('C:/source.zip')!;const tasks=useTaskStore();tasks.addTask({id:'done-edit',type:'compression',name:'edited.zip',sourceFiles:['C:/source.zip'],outputPath:'C:/new.zip'});tasks.tasks[0].archiveOutput={primaryPath:'C:/new.zip',files:[{path:'C:/new.zip',bytes:20}],totalBytes:20,split:false};tasks.tasks[0].status='completed';session.taskId='done-edit';session.editing=false;
 const wrapper=mount(ZipEditBar,{props:{sourcePath:'C:/source.zip',directory:''},global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}});await wrapper.findAll('button').find(button=>button.text()==='查看结果')!.trigger('click');expect(wrapper.find('[data-testid="zip-edit-change-list"]').exists()).toBe(false);expect(wrapper.find('[data-testid="zip-edit-output"]').exists()).toBe(false);expect(wrapper.find('[data-testid="zip-edit-browse-result"]').exists()).toBe(true);wrapper.unmount()
})
