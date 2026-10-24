import {beforeEach,afterEach,it,expect,vi} from 'vitest'
import {createPinia,setActivePinia} from 'pinia'
import {useCompressionStore} from '../compression'
import {usePendingDraftStore,PENDING_DRAFTS_KEY} from '../pendingDrafts'
import {copyExecutionConfig} from '@/types/executionConfig'
import {workflowCompressionOptions,workflowSettings} from '@/utils/workflow'
import {normalizeSevenZipMethod,compressionAnalysisFormat} from '@/utils/sevenZipMethod'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
let recovery:ReturnType<typeof usePendingDraftStore>|undefined
const file=(name:string)=>({path:`C:/${name}`,name,size:20,type:'file',isDirectory:false})
beforeEach(()=>{localStorage.clear();setActivePinia(createPinia());mocks.invoke.mockResolvedValue({name:'file',size:20,is_dir:false})})
afterEach(()=>recovery?.dispose())
it('7Z-only and mixed method batch preserves unrelated inheritance and passwords',()=>{
 const s=useCompressionStore();s.globalSettings.format='zip';['seven','zip','inherited'].forEach(n=>s.addFile(file(n)))
 s.updateFileSettings('C:/seven',{...s.globalSettings,format:'7z',password:'keep',splitArchive:true,splitSize:'2'})
 const other=JSON.stringify(s.selectedFiles.slice(1)),global=JSON.stringify(s.globalSettings)
 expect(s.applyBatchConfiguration(s.selectedFiles.map(f=>f.path),{sevenZipMethod:'bzip2'})).toBe(1)
 expect(s.selectedFiles[0].settings).toMatchObject({sevenZipMethod:'bzip2',password:'keep',splitSize:'2'})
 expect(JSON.stringify(s.selectedFiles.slice(1))).toBe(other);expect(JSON.stringify(s.globalSettings)).toBe(global)
 expect(()=>s.applyBatchConfiguration(['C:/zip'],{sevenZipMethod:'lzma'})).toThrow('没有 7Z')
 expect(s.applyBatchConfiguration(['C:/seven','C:/zip'],{sevenZipMethod:'lzma',zipMethod:'ppmd'})).toBe(2)
 expect(s.selectedFiles[0].settings?.sevenZipMethod).toBe('lzma');expect(s.selectedFiles[1].settings?.zipMethod).toBe('ppmd')
})
it('history and workflow keep target method independently of ZIP source and old default',()=>{
 for(const method of ['lzma2','lzma','bzip2'] as const){
  const settings={...workflowSettings(),format:'7z' as const,sevenZipMethod:method,zipMethod:'ppmd' as const}
  const config=copyExecutionConfig({schemaVersion:1,kind:'archive-compression',outputDirectory:'D:/out',passwordRequired:false,settings})
  expect(config?.kind==='archive-compression'&&config.settings.sevenZipMethod).toBe(method)
  expect(workflowCompressionOptions(settings,'').seven_zip_method).toBe(method)
  expect(compressionAnalysisFormat(settings)).toBe(method==='lzma2'?'7z':`7z-${method}`)
 }
 expect(workflowCompressionOptions({...workflowSettings(),format:'7z'},'').seven_zip_method).toBe('lzma2')
 expect(compressionAnalysisFormat({format:'tar',zipMethod:'ppmd'})).toBe('tar');expect(()=>normalizeSevenZipMethod('unknown')).toThrow()
})
it('pending draft restores selected method and global default without auto executing',async()=>{
 recovery=usePendingDraftStore();recovery.initialize();const s=useCompressionStore();s.globalSettings.format='7z';s.globalSettings.sevenZipMethod='lzma';s.addFile(file('seven'));s.updateFileSettings('C:/seven',{...s.globalSettings,sevenZipMethod:'bzip2',password:'session-only'});recovery.flush()
 expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('session-only');recovery.dispose();setActivePinia(createPinia());recovery=usePendingDraftStore();recovery.initialize();await recovery.restore()
 expect(useCompressionStore().globalSettings.sevenZipMethod).toBe('lzma');expect(useCompressionStore().selectedFiles[0].settings?.sevenZipMethod).toBe('bzip2');expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='compress_files')).toBe(false)
})
