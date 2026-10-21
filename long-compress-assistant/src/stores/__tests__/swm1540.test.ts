import { beforeEach, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { shallowMount } from '@vue/test-utils'
import WimMethodField from '@/components/compression/WimMethodField.vue'
import { useCompressionStore } from '../compression'
import { useArchiveConversionStore, conversionFormats } from '../archiveConversion'
import { useTaskStore } from '../task'
import { createTaskHistoryRecord } from '@/types/taskHistory'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({ask:vi.fn(async()=>true)}))
beforeEach(()=>{setActivePinia(createPinia());mocks.invoke.mockReset()})
const file=(name:string)=>({name,path:'C:/'+name,size:1,type:'file',isDirectory:false})
it('swm1540 inherited native volumes stay inherited and default to a supported method',()=>{
 const s=useCompressionStore();s.globalSettings.format='swm';s.globalSettings.wimMethod='solid';s.addFile(file('one'))
 expect(s.selectedFiles[0].settings).toBeUndefined();expect(s.getEffectiveSettings()).toMatchObject({format:'swm',wimMethod:'store',splitArchive:true,splitSize:'1024'})
 expect(s.globalSettings.wimMethod).toBe('solid');expect(s.selectedFiles[0].settings).toBeUndefined()
})
it('swm1540 bulk format preserves sizes, other inheritance and rejects password removal',()=>{
 const s=useCompressionStore();['a','b','other'].forEach(n=>s.addFile(file(n)));const other=JSON.stringify(s.selectedFiles[2]);const global=JSON.stringify(s.globalSettings)
 for(const[i,f]of s.selectedFiles.slice(0,2).entries())s.updateFileSettings(f.path,{...s.globalSettings,splitArchive:true,splitSize:String(i+1)})
 expect(s.applyBatchConfiguration(['C:/a','C:/b'],{format:'swm',wimMethod:'lzx'})).toBe(2)
 s.selectedFiles.slice(0,2).forEach((f,i)=>expect(s.getEffectiveSettings(f.settings)).toMatchObject({format:'swm',wimMethod:'lzx',splitArchive:true,splitSize:String(i+1)}))
 expect(JSON.stringify(s.selectedFiles[2])).toBe(other);expect(JSON.stringify(s.globalSettings)).toBe(global)
 s.updateFileSettings('C:/other',{...s.globalSettings,password:'keep'});expect(()=>s.applyBatchConfiguration(['C:/a','C:/other'],{format:'swm'})).toThrow(/密码/)
 expect(s.selectedFiles[2].settings?.password).toBe('keep')
})
it('swm1540 method-only mixed batch applies LZX to SWM but never applies solid',()=>{
 const s=useCompressionStore();['native','zip'].forEach(n=>s.addFile(file(n)));s.updateFileSettings('C:/native',{...s.globalSettings,format:'swm',wimMethod:'xpress',splitSize:'2'})
 const other=JSON.stringify(s.selectedFiles[1]);expect(s.applyBatchConfiguration(['C:/native','C:/zip'],{wimMethod:'lzx'})).toBe(1)
 expect(s.getEffectiveSettings(s.selectedFiles[0].settings).wimMethod).toBe('lzx');expect(s.applyBatchConfiguration(['C:/native'],{wimMethod:'solid'})).toBe(0);expect(JSON.stringify(s.selectedFiles[1])).toBe(other)
})
it('swm1540 conversion keeps source password separate and persists method through history',async()=>{
 mocks.invoke.mockImplementation(async cmd=>cmd==='detect_split_archive'?{is_split:false,archive_format:'zip'}:cmd==='convert_archive'?{primaryPath:'D:/new.swm',files:[{path:'D:/new.swm',bytes:1},{path:'D:/new2.swm',bytes:2}],totalBytes:3,split:true}:undefined)
 const s=useArchiveConversionStore();s.addSources(['C:/secret.zip']);s.updateItem(s.pending[0].id,{format:'swm',wimMethod:'lzx',sourcePassword:'old-key',splitMiB:1,outputPath:'D:/new.swm'})
 expect(conversionFormats.some(option=>option.value==='swm')).toBe(true);await s.start()
 expect(mocks.invoke).toHaveBeenCalledWith('convert_archive',expect.objectContaining({sourcePassword:'old-key',outputPath:'D:/new.swm',options:expect.objectContaining({format:'swm',wim_method:'lzx',password:null,split_size:1048576})}))
 const record=createTaskHistoryRecord(useTaskStore().tasks[0]);expect(record.executionConfig).toMatchObject({format:'swm',wimMethod:'lzx',splitSize:1048576});s.restore(record)
 expect(s.effectiveSettings(s.pending.at(-1)!)).toMatchObject({format:'swm',wimMethod:'lzx',sourcePassword:'',splitMiB:1})
})
it('swm1540 choosing native format gives a valid size but editing zero remains an actionable error',()=>{
 const s=useArchiveConversionStore();s.addSources(['C:/old.zip']);s.updateItem(s.pending[0].id,{format:'swm'});expect(s.effectiveSettings(s.pending[0]).splitMiB).toBe(1024)
 s.updateItem(s.pending[0].id,{splitMiB:0});expect(s.effectiveSettings(s.pending[0]).splitMiB).toBe(0)
})
it('swm1540 method selector exposes only actual non-solid methods',()=>{
 const wrapper=shallowMount(WimMethodField,{props:{format:'swm',modelValue:'xpress'},global:{stubs:{ThemedSelect:true,HelpHint:true}}})
 expect(wrapper.findComponent({name:'ThemedSelect'}).props('options').map((o:{value:string})=>o.value)).toEqual(['store','xpress','lzx'])
 expect(wrapper.text()).toContain('SWM 压缩方法')
})

it('swm1540 lets a user clear and replace volume size without echoing the default during typing',()=>{
 const s=useCompressionStore();s.globalSettings.format='swm';s.globalSettings.splitSize='';expect(s.getEffectiveSettings().splitSize).toBe('');s.globalSettings.splitSize='1';expect(s.getEffectiveSettings().splitSize).toBe('1')
})
