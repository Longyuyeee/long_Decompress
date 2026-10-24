import {beforeEach,it,expect,vi} from 'vitest'
import {createPinia,setActivePinia} from 'pinia'
import {useArchiveConversionStore} from '../archiveConversion'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
beforeEach(()=>{setActivePinia(createPinia());mocks.invoke.mockImplementation(async cmd=>cmd==='detect_split_archive'?{is_split:false,archive_format:'zip'}:undefined)})
it('single defaults and mixed conversion accept the selected target method without restoring the old value',async()=>{
 const s=useArchiveConversionStore();s.updateDefaults({format:'zip',zipMethod:'store',level:8});expect(s.defaults.zipMethod).toBe('store');s.addSources(['C:/first.zip','C:/second.zip']);await s.refreshPendingSources()
 const [one,two]=s.pending;expect(s.effectiveSettings(one).zipMethod).toBe('store');s.updateItem(one.id,{zipMethod:'lzma',password:'keep',splitMiB:2,zipSplitMode:'standard'});expect(s.effectiveSettings(one)).toMatchObject({zipMethod:'lzma',level:8,password:'keep',splitMiB:2,zipSplitMode:'standard'})
 s.updateItem(two.id,{format:'7z',sevenZipMethod:'copy'});expect(s.effectiveSettings(two).sevenZipMethod).toBe('copy');one.selected=two.selected=true
 await s.applySelected({zipMethod:'store'});expect(s.effectiveSettings(one)).toMatchObject({zipMethod:'store',password:'keep',level:8,splitMiB:2,zipSplitMode:'standard'});expect(s.effectiveSettings(two)).toMatchObject({format:'7z',sevenZipMethod:'copy'});expect(mocks.invoke.mock.calls.some(([c])=>c==='convert_archive')).toBe(false)
})
