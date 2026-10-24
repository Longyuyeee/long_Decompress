import {beforeEach,afterEach,it,expect,vi} from 'vitest'
import {createPinia,setActivePinia} from 'pinia'
import {useCompressionStore} from '../compression'
import {usePendingDraftStore,PENDING_DRAFTS_KEY} from '../pendingDrafts'
import {copyExecutionConfig} from '@/types/executionConfig'
import {workflowCompressionOptions,workflowSettings} from '@/utils/workflow'
import {compressionAnalysisFormat} from '@/utils/sevenZipMethod'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
let recovery:ReturnType<typeof usePendingDraftStore>|undefined
const file=(name:string)=>({path:`C:/${name}`,name,size:20,type:'file',isDirectory:false})
beforeEach(()=>{localStorage.clear();setActivePinia(createPinia());mocks.invoke.mockResolvedValue({name:'file',size:20,is_dir:false})})
afterEach(()=>recovery?.dispose())
it('ZIP-only and mixed method batch preserves unrelated inheritance and passwords',()=>{
 const s=useCompressionStore();s.globalSettings.format='7z';['zip','seven','inherited'].forEach(n=>s.addFile(file(n)))
 s.updateFileSettings('C:/zip',{...s.globalSettings,format:'zip',password:'keep',splitArchive:true,splitSize:'2'})
 const other=JSON.stringify(s.selectedFiles.slice(1)),global=JSON.stringify(s.globalSettings)
 expect(s.applyBatchConfiguration(s.selectedFiles.map(f=>f.path),{zipMethod:'store'})).toBe(1)
 expect(s.selectedFiles[0].settings).toMatchObject({zipMethod:'store',password:'keep',splitSize:'2'})
 expect(JSON.stringify(s.selectedFiles.slice(1))).toBe(other);expect(JSON.stringify(s.globalSettings)).toBe(global)
 expect(()=>s.applyBatchConfiguration(['C:/seven'],{zipMethod:'store'})).toThrow('没有 ZIP')
 expect(s.applyBatchConfiguration(['C:/zip','C:/seven'],{zipMethod:'store',sevenZipMethod:'copy'})).toBe(2)
 expect(s.selectedFiles[0].settings?.zipMethod).toBe('store');expect(s.selectedFiles[1].settings?.sevenZipMethod).toBe('copy')
})
it('history workflow and analysis keep ZIP Store separate from source and legacy zero semantics',()=>{
 const settings={...workflowSettings(),format:'zip' as const,zipMethod:'store' as const,level:9}
 const config=copyExecutionConfig({schemaVersion:1,kind:'archive-compression',outputDirectory:'D:/out',passwordRequired:false,settings})
 expect(config?.kind==='archive-compression'&&config.settings.zipMethod).toBe('store')
 expect(workflowCompressionOptions(settings,'').zip_method).toBe('store');expect(compressionAnalysisFormat(settings)).toBe('zip-store')
 expect(compressionAnalysisFormat({format:'zip',level:0})).toBe('zip-store')
 for(const [method,route] of [['zstd','zip-zstd'],['bzip2','zip-bzip2'],['ppmd','zip-ppmd'],['lzma','zip-lzma']])expect(compressionAnalysisFormat({format:'zip',zipMethod:method,level:0})).toBe(route)
 expect(compressionAnalysisFormat({format:'zipx',zipMethod:'store'})).toBe('xz')
 expect(compressionAnalysisFormat({format:'zip',level:6})).toBe('zip')
})
it('pending draft restores selected method and global default without auto executing',async()=>{
 recovery=usePendingDraftStore();recovery.initialize();const s=useCompressionStore();s.globalSettings.format='zip';s.globalSettings.zipMethod='deflate';s.addFile(file('seven'));s.updateFileSettings('C:/seven',{...s.globalSettings,zipMethod:'store',password:'session-only'});recovery.flush()
 expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('session-only');recovery.dispose();setActivePinia(createPinia());recovery=usePendingDraftStore();recovery.initialize();await recovery.restore()
 expect(useCompressionStore().globalSettings.zipMethod).toBe('deflate');expect(useCompressionStore().selectedFiles[0].settings?.zipMethod).toBe('store');expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='compress_files')).toBe(false)
})
