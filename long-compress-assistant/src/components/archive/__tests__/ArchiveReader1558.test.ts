import {beforeEach,afterEach,it,expect,vi} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia,setActivePinia} from 'pinia'
import ArchiveImageReader from '../ArchiveImageReader.vue'
import ReaderRetrievalDialog from '../ReaderRetrievalDialog.vue'
import {useArchiveReadingStore} from '@/stores/archiveReading'
import {useTaskStore} from '@/stores/task'
import {copyExecutionConfig} from '@/types/executionConfig'
import {createTaskHistoryRecord} from '@/types/taskHistory'
import {copyReadingRecord,copyReaderSelection,READING_KEY,readingKey} from '@/types/archiveReading'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>()=>{})}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
const revision='a'.repeat(64)
const picture=(path:string)=>({path,name:path.split('/').pop()!,size:10,isDir:false,compressedSize:null,modified:null,crc:null,encrypted:false})
const props={filePath:'C:/pages.zip',format:'ZIP',entries:Array.from({length:125},(_,i)=>picture(`pages/${i+1}.png`)),initialPath:'pages/1.png',directory:'pages',revision,password:'session-1558',supported:true,imageExtensions:['png']}
const saved=()=>({filePath:props.filePath,format:'ZIP',revision,path:'pages/101.png',directory:'pages',scope:'directory' as const,fit:false,zoom:1.5,selected:['pages/2.png','pages/101.png'],updatedAt:new Date().toISOString()})
const mountReader=(extra={})=>mount(ArchiveImageReader,{props:{...props,...extra},global:{stubs:{Teleport:true}}})
beforeEach(()=>{localStorage.clear();setActivePinia(createPinia());mocks.invoke.mockReset();mocks.invoke.mockImplementation(async(command,args)=>command==='read_archive_image'?{entryPath:args.entryPath,dataUrl:'data:image/png;base64,AA==',width:100,height:120,byteSize:10,mimeType:'image/png'}:undefined);vi.stubGlobal('URL',{createObjectURL:vi.fn(()=> 'blob:1558'),revokeObjectURL:vi.fn()})})
afterEach(()=>{vi.unstubAllGlobals();document.body.innerHTML=''})
it('reader1558 resume restores the path, scope, view and cross-page selection after a new store, without credentials or image payloads',async()=>{
 const row={...saved(),password:'must-not-save',dataUrl:'must-not-save'};useArchiveReadingStore().save(row);setActivePinia(createPinia());const restored=useArchiveReadingStore();expect(restored.records).toHaveLength(1);expect(localStorage.getItem(READING_KEY)).not.toContain('must-not-save')
 const w=mountReader({resume:restored.records[0]});await flushPromises();expect(w.get('[data-testid="reader-current-path"]').text()).toBe('pages/101.png');expect(w.get('[data-testid="reader-zoom"]').text()).toBe('150%');expect(w.get('[data-testid="reader-collect"]').text()).toContain('2');await w.get('[data-testid="reader-next"]').trigger('click');expect(w.get('[data-testid="reader-zoom"]').text()).toBe('150%');w.unmount();expect(restored.find(props.filePath)?.path).toBe('pages/102.png')
})
it('reader1558 changed source keeps a surviving path but clears old selections and tells the reader',async()=>{
 const w=mountReader({resume:saved(),revision:'b'.repeat(64)});await flushPromises();expect(w.get('[data-testid="reader-current-path"]').text()).toBe('pages/101.png');expect(w.get('[data-testid="reader-resume-notice"]').text()).toContain('所选图片已清空');expect(w.find('[data-testid="reader-collect"]').exists()).toBe(false);w.unmount()
})
it('reader1558 missing original page asks for a new choice rather than jumping to the old page number',async()=>{
 const w=mountReader({resume:{...saved(),path:'pages/deleted.png'}});await flushPromises();expect(w.get('[data-testid="reader-resume-notice"]').text()).toContain('不会按旧页码');expect(w.find('[data-testid="reader-image"]').exists()).toBe(false);expect(w.find('[data-testid="reader-finder"]').exists()).toBe(true);expect(mocks.invoke).not.toHaveBeenCalled();await w.get('[data-reader-entry="pages/1.png"] .finder-item').trigger('click');await flushPromises();expect(w.get('[data-testid="reader-current-path"]').text()).toBe('pages/1.png');w.unmount()
})
it('reader1558 long list renders only forty rows, searches paths, and collects explicit cross-page choices without decoding the whole list',async()=>{
 const w=mountReader();await flushPromises();await w.get('[data-testid="reader-find-toggle"]').trigger('click');expect(w.findAll('[data-reader-entry]')).toHaveLength(40);await w.get('[data-testid="reader-select-current"]').setValue(true);await w.get('[data-testid="reader-finder-next"]').trigger('click');expect(w.get('[data-testid="reader-finder-page"]').text()).toContain('2 / 4');await w.get('[data-reader-entry="pages/41.png"] input').setValue(true);await w.get('[data-testid="reader-search"]').setValue('125.png');expect(w.findAll('[data-reader-entry]')).toHaveLength(1);await w.get('[data-testid="reader-collect"]').trigger('click');expect(w.emitted('collect')?.[0][0]).toEqual(['pages/1.png','pages/41.png']);expect(mocks.invoke.mock.calls.filter(([c])=>c==='read_archive_image')).toHaveLength(2);w.unmount()
})
it('reader1558 records cap recent sources and keep snapshots separate and canonical Windows paths equivalent',()=>{
 const store=useArchiveReadingStore();for(let n=0;n<35;n++)store.save({...saved(),filePath:`C:/pages${n}.zip`});expect(store.records).toHaveLength(30);expect(store.records[0].filePath).toBe('C:/pages34.zip');store.save({...saved(),snapshot:{version:1,date:20261021010000}});store.save({...saved(),snapshot:{version:2,date:20261021020000}});expect(store.records.filter(row=>row.filePath===props.filePath)).toHaveLength(2);expect(readingKey('\\\\?\\C:\\Pages.zip')).toBe(readingKey(props.filePath));expect(copyReadingRecord({...saved(),zoom:9})).toBeNull();store.remove(store.records[0]);expect(store.records).toHaveLength(29)
})
it('reader1558 corrupt or unavailable local storage leaves reading usable and gives a recoverable notice',async()=>{
 localStorage.setItem(READING_KEY,'not json');setActivePinia(createPinia());expect(useArchiveReadingStore().notice).toContain('直接打开');const w=mountReader();await flushPromises();expect(w.find('[data-testid="reader-image"]').exists()).toBe(true);w.unmount()
})
it('reader1558 exact selection survives execution config and terminal history while injected secrets and empty ranges are discarded',()=>{
 const readerSelection={revision,paths:['pages/2.png','pages/101.png'],keepStructure:false,conflictPolicy:'skip' as const,password:'do-not-copy'};const config=copyExecutionConfig({schemaVersion:1,kind:'archive-extraction',outputDirectory:'D:/out',extractToSubfolder:false,readerSelection});expect(config).toMatchObject({readerSelection:{paths:readerSelection.paths}});expect(JSON.stringify(config)).not.toContain('do-not-copy');expect(copyReaderSelection({...readerSelection,paths:[]})).toBeNull();expect(copyReaderSelection({...readerSelection,paths:['same','same']})).toBeNull()
 const tasks=useTaskStore();tasks.addTask({id:'reader',name:'reader',type:'decompression',sourceFiles:[props.filePath],outputPath:'D:/out',readerSelection,executionConfig:config!,password:'not-history'});const task=tasks.tasks[0];task.status='completed';const history=createTaskHistoryRecord(task);expect(history.executionConfig).toMatchObject({readerSelection:{paths:readerSelection.paths,keepStructure:false,conflictPolicy:'skip'}});expect(JSON.stringify(history)).not.toContain('not-history')
})
const dialogProps={filePath:props.filePath,format:'ZIP',revision,paths:['pages/2.png','pages/101.png'],outputPath:'D:/out',password:'session-only',passwordRequired:true}
it('reader1558 retrieval modal confirms only chosen paths and queues one original pending task; cancellation does no work',async()=>{
 const cancelled=mount(ReaderRetrievalDialog,{props:dialogProps,global:{stubs:{Modal:{template:'<div><slot/><slot name="footer"/></div>'},RouterLink:true}}});await cancelled.findAll('button').find(b=>b.text()==='返回阅读')!.trigger('click');expect(useTaskStore().tasks).toHaveLength(0);expect(mocks.invoke).not.toHaveBeenCalled();cancelled.unmount()
 const w=mount(ReaderRetrievalDialog,{props:dialogProps,global:{stubs:{Modal:{template:'<div><slot/><slot name="footer"/></div>'},RouterLink:true}}});await w.get('[data-testid="reader-retrieval-structure"]').setValue(false);await w.get('[data-testid="reader-retrieval-conflict"]').setValue('skip');await w.get('[data-testid="reader-retrieval-confirm"]').trigger('click');await flushPromises();expect(mocks.invoke).toHaveBeenCalledWith('check_image_reader_source',{filePath:props.filePath,revision});expect(useTaskStore().tasks).toHaveLength(1);expect(useTaskStore().tasks[0]).toMatchObject({status:'pending',sourceFiles:[props.filePath],readerSelection:{paths:dialogProps.paths,keepStructure:false,conflictPolicy:'skip'},password:'session-only'});expect(w.find('[data-testid="reader-retrieval-go-queue"]').exists()).toBe(true);expect(mocks.invoke.mock.calls.some(([c])=>c==='extract_file')).toBe(false);w.unmount()
})
it('reader1558 retrieval source change preserves choices and does not enqueue a whole archive',async()=>{
 mocks.invoke.mockRejectedValue(new Error('IMAGE_READER_CHANGED|来源已变化'));const w=mount(ReaderRetrievalDialog,{props:dialogProps,global:{stubs:{Modal:{template:'<div><slot/><slot name="footer"/></div>'},RouterLink:true}}});await w.get('[data-testid="reader-retrieval-confirm"]').trigger('click');await flushPromises();expect(w.get('[data-testid="reader-retrieval-error"]').text()).toBe('来源已变化');expect(w.findAll('.retrieval-path')).toHaveLength(2);expect(useTaskStore().tasks).toHaveLength(0);w.unmount()
})
