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
describe('ZIPX output names and editable recovery',()=>{
 it.each([['C:/source.zip',['Deflated'],'C:/source_已编辑.zip'],['C:/source.ZIPX',['Xz'],'C:/source_已编辑.zipx'],['C:/method95.zip',['Xz'],'C:/method95_已编辑.zipx'],['C:/mixed.zipx',['Stored'],'C:/mixed_已编辑.zipx']])('suggests a clear output for %s',async(source,methods,expected)=>{sourceCatalog={...catalog,methods};const store=create();await store.begin(source);expect(store.sessionFor(source)?.outputPath).toBe(expected)})
 it('restores a ZIPX edit with its exact output and refills credentials only for this session',async()=>{const saved=snapshot('C:/source.zipx');saved.catalog.methods=['Xz'];saved.outputPath='D:/chosen.zipx';stored=JSON.stringify({version:1,sessions:[saved]});const store=create();await store.initialize();await store.restoreDraft(saved.sourcePath);expect(store.sessionFor(saved.sourcePath)).toMatchObject({outputPath:'D:/chosen.zipx',sourcePassword:'',outputPassword:''});expect(mocks.invoke.mock.calls.some(c=>c[0]==='save_zip_edit')).toBe(false)})
 it('submits ZIPX identity and the selected output without changing the modification plan',async()=>{sourceCatalog={...catalog,methods:['Xz']};const store=create();await store.begin('C:/source.zipx',{password:'source'});const session=store.sessionFor('C:/source.zipx')!;session.outputMode='aes256';session.outputPassword='target';await store.save(session.sourcePath);expect(store.taskFor(session)?.format).toBe('zipx');const args=mocks.invoke.mock.calls.find(c=>c[0]==='save_zip_edit')![1];expect(args.outputPath).toBe('C:/source_已编辑.zipx');expect(args.credentials).toMatchObject({outputMode:'aes256',sourcePassword:'source',outputPassword:'target'})})
})
