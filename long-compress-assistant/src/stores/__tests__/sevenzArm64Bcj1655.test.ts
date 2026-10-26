import {beforeEach,afterEach,it,expect,vi} from 'vitest'
import {createPinia,setActivePinia} from 'pinia'
import {useCompressionStore} from '../compression'
import {useArchiveConversionStore} from '../archiveConversion'
import {usePendingDraftStore,PENDING_DRAFTS_KEY} from '../pendingDrafts'
import {useAppStore} from '../app'
import {useTaskStore} from '../task'
import {copyExecutionConfig} from '@/types/executionConfig'
import {workflowCompressionOptions} from '@/utils/workflow'
import {compressionAnalysisFormat} from '@/utils/sevenZipMethod'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({ask:vi.fn(async()=>true)}))
let recovery:ReturnType<typeof usePendingDraftStore>|undefined
beforeEach(()=>{localStorage.clear();setActivePinia(createPinia());mocks.invoke.mockReset();mocks.invoke.mockImplementation(async cmd=>cmd==='detect_split_archive'?{is_split:false,archive_format:'zip'}:cmd==='get_file_info'?{name:'file',size:20,is_dir:false}:cmd==='convert_archive'?{primaryPath:'D:/new.7z',files:[],totalBytes:20,split:false}:undefined)})
afterEach(()=>{recovery?.dispose();recovery=undefined})
const file=(name:string)=>({path:`C:/${name}`,name,size:20,type:'file',isDirectory:false})
it('mixed batch changes only 7Z preprocessing and preserves other settings and inheritance',()=>{
 const s=useCompressionStore();s.globalSettings.format='zip';s.addFile(file('seven'));s.addFile(file('zip'));s.updateFileSettings('C:/seven',{...s.globalSettings,format:'7z',password:'keep',level:8,splitArchive:true,splitSize:'2'})
 const global=JSON.stringify(s.globalSettings),other=JSON.stringify(s.selectedFiles[1]);expect(s.applyBatchConfiguration(s.selectedFiles.map(f=>f.path),{sevenZipFilter:'bcj_arm64',sevenZipDeltaDistance:256})).toBe(1)
 expect(s.selectedFiles[0].settings).toMatchObject({sevenZipFilter:'bcj_arm64',sevenZipDeltaDistance:256,password:'keep',level:8,splitSize:'2'});expect(JSON.stringify(s.globalSettings)).toBe(global);expect(JSON.stringify(s.selectedFiles[1])).toBe(other)
 expect(()=>s.applyBatchConfiguration(['C:/zip'],{sevenZipFilter:'bcj_arm64'})).toThrow('没有 7Z');expect(()=>s.applyBatchConfiguration(['C:/seven'],{sevenZipDeltaDistance:257})).toThrow('1～256')
})
it('history workflow and preset keep target filter independently of method and old defaults',()=>{
 const s=useCompressionStore(),settings={...s.globalSettings,format:'7z' as const,sevenZipMethod:'lzma' as const,sevenZipFilter:'bcj_arm64' as const,sevenZipDeltaDistance:256}
 const config=copyExecutionConfig({schemaVersion:1,kind:'archive-compression',outputDirectory:'D:/out',passwordRequired:false,settings});expect(config?.kind==='archive-compression'&&config.settings).toMatchObject({sevenZipFilter:'bcj_arm64',sevenZipDeltaDistance:256})
 expect(workflowCompressionOptions(settings,'')).toMatchObject({seven_zip_method:'lzma',seven_zip_filter:'bcj_arm64',seven_zip_delta_distance:256});expect(compressionAnalysisFormat(settings)).toBe('7z-lzma-bcj-arm64')
 expect(compressionAnalysisFormat({...settings,sevenZipMethod:'bzip2'})).toBe('7z-bzip2');expect(compressionAnalysisFormat({format:'7z'})).toBe('7z')
 const app=useAppStore();app.saveCompressionPreset('ARM64 BCJ','7z',8,undefined,undefined,undefined,'lzma','bcj_arm64',256);expect(JSON.parse(localStorage.getItem('compression-presets')!)[0]).toMatchObject({sevenZipFilter:'bcj_arm64',sevenZipDeltaDistance:256})
})
it('pending draft restores ARM64 BCJ and distance without passwords or automatic execution',async()=>{
 recovery=usePendingDraftStore();recovery.initialize();const s=useCompressionStore();s.globalSettings.format='7z';s.addFile(file('seven'));s.updateFileSettings('C:/seven',{...s.globalSettings,sevenZipFilter:'bcj_arm64',sevenZipDeltaDistance:4,password:'session-only'});recovery.flush();expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('session-only');recovery.dispose();setActivePinia(createPinia());recovery=usePendingDraftStore();recovery.initialize();await recovery.restore();expect(useCompressionStore().selectedFiles[0].settings).toMatchObject({sevenZipFilter:'bcj_arm64',sevenZipDeltaDistance:4});expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='compress_files')).toBe(false)
})
it('conversion single defaults mixed batch and actual IPC keep the chosen filter and distance',async()=>{
 const s=useArchiveConversionStore();s.updateDefaults({format:'7z',sevenZipFilter:'bcj_arm64',sevenZipDeltaDistance:256});s.addSources(['C:/first.zip','C:/second.zip']);await s.refreshPendingSources();const [one,two]=s.pending;s.updateItem(one.id,{sevenZipDeltaDistance:4,outputPath:'D:/new.7z'});s.updateItem(two.id,{format:'zip'});one.selected=two.selected=true;await s.applySelected({sevenZipFilter:'bcj_arm64',sevenZipDeltaDistance:16});expect(s.effectiveSettings(one)).toMatchObject({sevenZipFilter:'bcj_arm64',sevenZipDeltaDistance:16});expect(s.effectiveSettings(two).sevenZipDeltaDistance).toBe(256);two.selected=false;await s.start();expect(mocks.invoke).toHaveBeenCalledWith('convert_archive',expect.objectContaining({options:expect.objectContaining({seven_zip_filter:'bcj_arm64',seven_zip_delta_distance:16})}));expect(useTaskStore().tasks[0].executionConfig).toMatchObject({sevenZipFilter:'bcj_arm64',sevenZipDeltaDistance:16})
})
