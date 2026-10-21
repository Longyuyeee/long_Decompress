import {beforeEach,afterEach,it,expect,vi} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia,setActivePinia} from 'pinia'
import {useTaskStore} from '../task'
import {usePendingDraftStore,PENDING_DRAFTS_KEY} from '../pendingDrafts'
import {useArchiveLibraryStore} from '../archiveLibrary'
import ArchiveLibraryView from '@/views/ArchiveLibraryView.vue'
import HistoryExtractionDraft from '@/components/tasks/HistoryExtractionDraft.vue'
import {extractionRetryPolicy,validateExtractionRetry} from '@/utils/extractionRetry'
import type {TaskHistoryRecord} from '@/types/taskHistory'
const mocks=vi.hoisted(()=>({invoke:vi.fn(),browse:vi.fn(),push:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>()=>{})}))
vi.mock('vue-router',()=>({useRouter:()=>({push:mocks.push})}))
vi.mock('@/stores/app',()=>({useAppStore:()=>({openArchiveInBrowser:mocks.browse,settings:{},t:(_key:string,fallback:string)=>fallback||_key})}))
const revision='a'.repeat(64),selection={revision,paths:['pages/2.png','pages/101.png'],keepStructure:false,conflictPolicy:'skip' as const}
const source={id:'pictures',path:'C:/pages.zip',name:'pages.zip',format:'ZIP',status:'ready' as const,note:'',revision:'library-revision',indexedAt:'2026-10-22',entryCount:125}
const hit={sourceId:source.id,sourcePath:source.path,sourceName:source.name,format:'ZIP',path:'pages/101.png',name:'101.png',extension:'png',size:10,modified:null}
let drafts:ReturnType<typeof usePendingDraftStore>|undefined
beforeEach(()=>{localStorage.clear();setActivePinia(createPinia());vi.clearAllMocks();mocks.invoke.mockImplementation(async(command,args)=>{
 if(command==='library_sources')return[source]
 if(command==='library_organization')return{sources:[],searches:[]}
 if(command==='library_search')return{items:[hit],total:400,page:args.request.page}
 if(command==='library_check')return source
 if(command==='detect_split_archive')return{is_split:false,archive_format:'ZIP'}
 if(command==='get_file_info')return{name:'pages.zip',is_dir:args.path==='D:/out',size:100}
 return undefined
})})
afterEach(()=>{drafts?.dispose();drafts=undefined;document.body.innerHTML=''})
it('reader1558 library matching image enters the shared reader and remount retains search, result page and scroll',async()=>{
 const library=useArchiveLibraryStore();library.query='101.png';library.category='image';library.page=7;library.workspace.scrollTop=143
 const w=mount(ArchiveLibraryView,{global:{stubs:{Modal:true,LibraryRetrievalDialog:true}}});await flushPromises();await w.get('[data-testid="library-hit-read"]').trigger('click');await flushPromises();expect(mocks.invoke).toHaveBeenCalledWith('library_check',{sourceId:'pictures',entryPath:'pages/101.png'});expect(mocks.browse).toHaveBeenCalledWith('C:/pages.zip',expect.objectContaining({focusEntryPath:'pages/101.png',entryAction:'read',librarySourceId:'pictures'}));expect(mocks.push).toHaveBeenCalledWith('/browser');w.unmount()
 const returned=mount(ArchiveLibraryView,{global:{stubs:{Modal:true,LibraryRetrievalDialog:true}}});await flushPromises();expect(library.query).toBe('101.png');expect(library.page).toBe(7);expect((returned.get('[data-testid="archive-library"]').element as HTMLElement).scrollTop).toBe(143);expect(returned.find('[data-testid="library-preview-hit"]').exists()).toBe(true);returned.unmount()
})
it('reader1558 pending reading tasks restore the exact paths and revision with fresh password required and no automatic extraction',async()=>{
 drafts=usePendingDraftStore();drafts.initialize();useTaskStore().addTask({id:'reader',name:'pages',type:'decompression',sourceFiles:['C:/pages.zip'],outputPath:'D:/out',readerSelection:selection,selectedEntries:selection.paths,password:'not-persisted-1558',configurationMode:'individual'});drafts.flush();expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('not-persisted-1558');drafts.dispose();setActivePinia(createPinia());drafts=usePendingDraftStore();drafts.initialize();await drafts.restore();expect(useTaskStore().tasks[0]).toMatchObject({readerSelection:selection,selectedEntries:selection.paths,status:'pending',draftPasswordRequired:true});expect(useTaskStore().tasks[0].password).toBeUndefined();expect(mocks.invoke.mock.calls.some(([c])=>c==='extract_file')).toBe(false)
})
it('reader1558 drafts do not merge the same paths from two different source revisions',async()=>{
 drafts=usePendingDraftStore();drafts.initialize();useTaskStore().addTask({id:'old',name:'pages',type:'decompression',sourceFiles:['C:/pages.zip'],outputPath:'D:/out',readerSelection:selection,selectedEntries:selection.paths});drafts.flush();drafts.dispose();setActivePinia(createPinia());useTaskStore().addTask({id:'new',name:'pages',type:'decompression',sourceFiles:['C:/pages.zip'],outputPath:'D:/out',readerSelection:{...selection,revision:'b'.repeat(64)},selectedEntries:selection.paths});drafts=usePendingDraftStore();drafts.initialize();await drafts.restore();expect(useTaskStore().tasks).toHaveLength(2);expect(new Set(useTaskStore().tasks.map(t=>t.readerSelection?.revision))).toEqual(new Set([revision,'b'.repeat(64)]))
})
it('reader1558 historical reading result recreates its exact scope after a source check, without copying a password',async()=>{
 const record:TaskHistoryRecord={id:'old',name:'pages.zip',taskType:'decompression',workloadKind:'archive',status:'completed',sourcePaths:['C:/pages.zip'],outputPath:'D:/out',completedAt:new Date().toISOString(),durationMs:0,processedBytes:0,totalBytes:0,logs:[],format:'ZIP',executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:'D:/out',extractToSubfolder:false,readerSelection:selection}}
 const w=mount(HistoryExtractionDraft,{props:{record},global:{stubs:{ExtractionOutputPreview:true}}});expect(w.get('[data-testid="history-reader-selection"]').text()).toContain('2 张图片');expect(w.get('[data-testid="history-draft-password"]').element.value).toBe('');await w.get('[data-testid="history-draft-create"]').trigger('click');await flushPromises();expect(mocks.invoke).toHaveBeenCalledWith('check_image_reader_source',{filePath:'C:/pages.zip',revision});expect(useTaskStore().tasks[0]).toMatchObject({readerSelection:selection,selectedEntries:selection.paths,status:'pending'});w.unmount()
})
it('reader1558 failure correction permits exact reading selection but refuses a changed version before creating a retry',async()=>{
 const tasks=useTaskStore();tasks.addTask({id:'failed',name:'pages',type:'decompression',sourceFiles:['C:/pages.zip'],outputPath:'D:/out',readerSelection:selection,selectedEntries:selection.paths});tasks.tasks[0].status='failed';tasks.tasks[0].error='output directory unavailable';expect(extractionRetryPolicy(tasks.tasks[0]).blocked).toBe('');mocks.invoke.mockImplementation(async(command,args)=>{if(command==='check_image_reader_source')throw Error('来源已变化');if(command==='detect_split_archive')return{is_split:false,archive_format:'ZIP'};return{name:'pages.zip',is_dir:args.path==='D:/out',size:100}})
 await expect(validateExtractionRetry({source:'C:/pages.zip',output:'D:/out',password:'fresh',extractToSubfolder:false,readerSelection:selection},false,'check',()=>false)).rejects.toThrow('来源已变化');expect(tasks.tasks).toHaveLength(1)
})
