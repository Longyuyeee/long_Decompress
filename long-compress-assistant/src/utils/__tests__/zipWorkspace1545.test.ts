import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { resolveZipWorkspace, previewZipStructure, validZipPath, zipBatchTargets } from '../zipWorkspace'
import { useZipEditStore } from '@/stores/zipEdit'
import { useTaskStore } from '@/stores/task'
import ZipOrganizeDialog from '@/components/archive/ZipOrganizeDialog.vue'
import ZipEditBar from '@/components/archive/ZipEditBar.vue'
import type { ZipEditCatalog, ZipEditDraft, ZipStructureOperation } from '@/types/zipEdit'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn(),save:vi.fn()}))
const catalog:ZipEditCatalog={revision:'r1',methods:['Deflated'],entries:[{path:'docs/',isDir:true,size:0},{path:'docs/a.txt',isDir:false,size:3},{path:'docs/b.txt',isDir:false,size:4},{path:'empty/',isDir:true,size:0},{path:'keep.txt',isDir:false,size:2}]}
const input=(path:string,order=0,choice:ZipEditDraft['choice']='choose'):ZipEditDraft=>({id:path+order,batch:'batch'+order,order,sourcePath:'C:/inputs/'+path,targetPath:path,isDir:path.endsWith('/'),size:path.endsWith('/')?0:9,sourceRevision:'input1',choice})
const op=(kind:'rename'|'delete',source:string,path:string|null,order=1):ZipStructureOperation=>({id:'operation'+order,kind,order,targets:[{id:'original:'+source,path}]})
beforeEach(()=>{setActivePinia(createPinia());mocks.invoke.mockReset();mocks.invoke.mockImplementation(async command=>command==='inspect_zip_edit'?structuredClone(catalog):command==='load_app_settings'?'{}':undefined)})
describe('zipWorkspace1545 composition',()=>{
 it('combines replacement, directory rename, later additions and deletion without changing original identities',()=>{
   const replacement=input('docs/a.txt',0,'replace'),later=input('manual/new.txt',3)
   const result=resolveZipWorkspace(catalog,[replacement,later],[op('rename','docs/','manual/',1),op('delete','docs/b.txt',null,2)])
   expect(result.unresolved).toBe(0);expect(result.nodes.map(n=>n.path)).toEqual(expect.arrayContaining(['manual/','manual/a.txt','manual/new.txt','keep.txt','empty/']))
   expect(result.nodes.some(n=>n.path==='manual/b.txt'||n.path==='docs/')).toBe(false)
   expect(result.changes.map(c=>c.targetPath)).toEqual(['manual/a.txt','manual/new.txt'])
   expect(result.originalEdits).toEqual(expect.arrayContaining([{sourcePath:'docs/',targetPath:'manual/'},{sourcePath:'docs/a.txt',targetPath:null},{sourcePath:'docs/b.txt',targetPath:null}]))
 })
 it('requires explicit collision choices for additions after a directory rename',()=>{
   const addition=input('manual/a.txt',2)
   let result=resolveZipWorkspace(catalog,[addition],[op('rename','docs/','manual/',1)]);expect(result.unresolved).toBe(1)
   addition.choice='keep-both';result=resolveZipWorkspace(catalog,[addition],[op('rename','docs/','manual/',1)]);expect(result.unresolved).toBe(0);expect(result.changes[0].targetPath).toBe('manual/a (2).txt')
 })
 it('deletes a selected directory once even when a child is also selected and undo restores the whole tree',()=>{
   const operation=op('delete','docs/',null);operation.targets.push({id:'original:docs/a.txt',path:null})
   const result=resolveZipWorkspace(catalog,[],[operation]);expect(result.structureRows[0].affected).toHaveLength(3);expect(result.nodes.some(n=>n.path.startsWith('docs/'))).toBe(false)
   expect(resolveZipWorkspace(catalog,[],[]).hasChanges).toBe(false)
 })
 it('supports case-only names and simultaneous name swaps',()=>{
   const operation=op('rename','docs/a.txt','docs/b.txt');operation.targets.push({id:'original:docs/b.txt',path:'docs/a.txt'})
   let result=resolveZipWorkspace(catalog,[],[operation]);expect(result.unresolved).toBe(0);expect(result.nodes.find(n=>n.originalPath==='docs/a.txt')?.path).toBe('docs/b.txt')
   result=resolveZipWorkspace(catalog,[],[op('rename','docs/','DOCS/')]);expect(result.unresolved).toBe(0);expect(result.nodes.find(n=>n.originalPath==='docs/a.txt')?.path).toBe('DOCS/a.txt')
 })
 it('rejects collisions, file-directory conflicts, own descendants and invalid paths before applying anything',()=>{
   const nodes=resolveZipWorkspace(catalog,[]).nodes
   for(const operation of [op('rename','docs/a.txt','keep.txt'),op('rename','docs/a.txt','empty'),op('rename','docs/','docs/sub/'),op('rename','docs/','../escape/'),op('rename','keep.txt','CON.txt'),op('rename','keep.txt','bad./file.txt')]){
     const result=previewZipStructure(nodes,operation);expect(result.issue).toBeTruthy();expect(result.nodes).toBe(nodes)
   }
   expect(validZipPath('folder\\file.txt',false)).toBe('folder/file.txt')
 })
 it('marks a dependent edit unresolved when its new input was undone',()=>{
   const addition=input('new.txt'),operation:ZipStructureOperation={id:'rename-input',order:1,kind:'rename',targets:[{id:'input:'+addition.id,path:'renamed.txt'}]}
   expect(resolveZipWorkspace(catalog,[addition],[operation]).unresolved).toBe(0)
   const result=resolveZipWorkspace(catalog,[],[operation]);expect(result.unresolved).toBe(1);expect(result.structureRows[0].issue).toContain('已撤销')
 })
 it('recomputes later operations by stable identity when an earlier rename is undone',()=>{
   const operations=[op('rename','docs/','manual/',1),op('rename','docs/a.txt','delivery/a.txt',2)]
   const result=resolveZipWorkspace(catalog,[],operations.slice(1));expect(result.unresolved).toBe(0);expect(result.nodes.find(n=>n.originalPath==='docs/a.txt')?.path).toBe('delivery/a.txt');expect(result.nodes.some(n=>n.path==='manual/')).toBe(false)
 })
 it('keeps extensions while applying literal text rules and a shared destination to independent roots',()=>{
   const nodes=resolveZipWorkspace(catalog,[]).nodes.filter(n=>['docs/a.txt','docs/b.txt'].includes(n.path))
   expect(zipBatchTargets(nodes,{prefix:'交付-',suffix:'-新',search:'a',replacement:'A',destination:'资料/整理'}).map(t=>t.path)).toEqual(['资料/整理/交付-A-新.txt','资料/整理/交付-b-新.txt'])
 })
 it('cancels out an addition deleted before save and leaves untouched originals unchanged',()=>{
   const addition=input('new.txt'),operation:ZipStructureOperation={id:'remove-input',order:1,kind:'delete',targets:[{id:'input:'+addition.id,path:null}]}
   const result=resolveZipWorkspace(catalog,[addition],[operation]);expect(result.hasChanges).toBe(false);expect(result.changes).toEqual([]);expect(result.originalEdits).toEqual([])
 })
})
describe('zipWorkspace1545 store and interface',()=>{
 it('refuses invalid organizing and can undo a structural action without dropping added files',async()=>{
   const store=useZipEditStore();await store.begin('C:/source.zip');const session=store.sessionFor('C:/source.zip')!;session.drafts=[input('new.txt')]
   expect(store.organize(session.sourcePath,'rename',op('rename','docs/a.txt','keep.txt').targets)).toBe(false);expect(session.operations).toHaveLength(0)
   expect(store.organize(session.sourcePath,'delete',op('delete','empty/',null).targets)).toBe(true);store.undoOperation(session.sourcePath,session.operations[0].id)
   expect(session.operations).toHaveLength(0);expect(session.drafts).toHaveLength(1)
 })
 it('saves delete-only edits, retains them on failure and links a corrected save to the old task',async()=>{
   const store=useZipEditStore();await store.begin('C:/source.zip');const session=store.sessionFor('C:/source.zip')!;store.organize(session.sourcePath,'delete',op('delete','docs/',null).targets)
   let calls=0;mocks.invoke.mockImplementation(async command=>{if(command==='save_zip_edit'){if(++calls===1)throw new Error('occupied');return {primaryPath:'C:/out.zip',files:[{path:'C:/out.zip',bytes:22}],totalBytes:22,split:false}}})
   await store.save(session.sourcePath);const failed=JSON.parse(JSON.stringify(store.taskFor(session)));expect(session.operations).toHaveLength(1)
   const args=mocks.invoke.mock.calls.find(call=>call[0]==='save_zip_edit')![1];expect(args.changes).toEqual([]);expect(args.originalEdits).toHaveLength(3)
   await store.save(session.sourcePath);expect(session.operations).toEqual([]);expect(store.taskFor(session)?.retryOfTaskId).toBe(failed.id);expect(JSON.parse(JSON.stringify(useTaskStore().tasks.find(t=>t.id===failed.id)))).toEqual(failed)
 })
 it('stages a multi-selection rule inside the organizing dialog and updates the projected tree',async()=>{
   const pinia=createPinia();setActivePinia(pinia);const store=useZipEditStore();await store.begin('C:/source.zip')
   const wrapper=mount(ZipOrganizeDialog,{props:{visible:true,sourcePath:'C:/source.zip',initialPaths:['docs/a.txt','docs/b.txt'],initialMode:'rename'},global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}});await flushPromises()
   expect(wrapper.get('[data-testid="zip-organize-apply"]').attributes('disabled')).toBeDefined();await wrapper.get('[data-testid="zip-organize-prefix"]').setValue('交付-');expect(wrapper.get('[data-testid="zip-organize-preview"]').text()).toContain('docs/交付-a.txt')
   await wrapper.get('[data-testid="zip-organize-apply"]').trigger('click');expect(store.sessionFor('C:/source.zip')!.operations).toHaveLength(1);expect(wrapper.get('[data-testid="zip-organize-notice"]').text()).toContain('影响 2 项');expect(mocks.invoke.mock.calls.some(c=>c[0]==='save_zip_edit')).toBe(false);wrapper.unmount()
 })
 it('maps a right-click on an original entry to its renamed location',async()=>{
   const pinia=createPinia();setActivePinia(pinia);const store=useZipEditStore();await store.begin('C:/source.zip');store.organize('C:/source.zip','rename',op('rename','docs/','manual/').targets)
   const wrapper=mount(ZipOrganizeDialog,{props:{visible:true,sourcePath:'C:/source.zip',initialPaths:['docs/a.txt'],initialMode:'rename'},global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}});await flushPromises();expect((wrapper.get('[data-testid="zip-organize-single-path"]').element as HTMLInputElement).value).toBe('manual/a.txt');wrapper.unmount()
 })
 it('searches structural child paths in the unified review and undo removes only that action',async()=>{
   const pinia=createPinia();setActivePinia(pinia);const store=useZipEditStore();await store.begin('C:/source.zip');store.organize('C:/source.zip','rename',op('rename','docs/','manual/').targets)
   const wrapper=mount(ZipEditBar,{props:{sourcePath:'C:/source.zip',directory:''},global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}});await wrapper.get('[data-testid="zip-edit-review"]').trigger('click');await wrapper.get('[data-testid="zip-edit-review-search"]').setValue('manual/a.txt');expect(wrapper.findAll('[data-testid="zip-edit-change"]')).toHaveLength(1)
   await wrapper.get('[data-testid="zip-edit-undo-operation"]').trigger('click');expect(store.sessionFor('C:/source.zip')!.operations).toHaveLength(0);expect(wrapper.get('[data-testid="zip-edit-save"]').attributes('disabled')).toBeDefined();wrapper.unmount()
 })
 it('routes original-folder additions and repeated original-file replacements to their current edited paths',async()=>{
   const store=useZipEditStore();await store.begin('C:/source.zip');const session=store.sessionFor('C:/source.zip')!;store.organize(session.sourcePath,'rename',op('rename','docs/','manual/').targets)
   mocks.invoke.mockImplementation(async(command,args)=>command==='prepare_zip_edit_inputs'?[{sourcePath:'C:/new.txt',targetPath:args.directory+'new.txt',isDir:false,size:9,sourceRevision:'s1'}]:undefined)
   await store.add(session.sourcePath,['C:/new.txt'],'docs');expect(mocks.invoke.mock.calls.find(call=>call[0]==='prepare_zip_edit_inputs')![1].directory).toBe('manual/')
   await store.add(session.sourcePath,['C:/new.txt'],'','docs/a.txt');let current=store.plan(session).nodes.find(node=>node.originalAlias==='docs/a.txt')!;expect(current.path).toBe('manual/a.txt')
   store.organize(session.sourcePath,'rename',[{id:current.id,path:'delivery/a.txt'}]);await store.add(session.sourcePath,['C:/new.txt'],'','docs/a.txt');current=store.plan(session).nodes.find(node=>node.originalAlias==='docs/a.txt')!;expect(current.path).toBe('delivery/a.txt');expect(store.plan(session).unresolved).toBe(0)
 })
 it('refuses editing an original location already deleted from the new package',async()=>{
   const store=useZipEditStore();await store.begin('C:/source.zip');store.organize('C:/source.zip','delete',op('delete','docs/',null).targets)
   await store.add('C:/source.zip',['C:/new.txt'],'docs');expect(store.sessionFor('C:/source.zip')!.error).toContain('已从新包移除');expect(mocks.invoke.mock.calls.some(call=>call[0]==='prepare_zip_edit_inputs')).toBe(false)
 })

})
