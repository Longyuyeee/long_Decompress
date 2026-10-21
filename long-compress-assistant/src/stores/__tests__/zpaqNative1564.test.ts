import {expect,it,vi} from 'vitest'
import {flushPromises} from '@vue/test-utils'
import {createPinia,setActivePinia} from 'pinia'
import {useArchiveConversionStore} from '../archiveConversion'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({ask:vi.fn()}))
it('zpaq1564 conversion defaults to original member directory and rechecks the immutable source reference',async()=>{
 setActivePinia(createPinia());const source='C:/profile/zpaq-sources/id/archive01.zpaq',parts=['D:/received/archive01.zpaq','D:/received/archive02.zpaq'];mocks.invoke.mockImplementation(async cmd=>cmd==='detect_split_archive'?{is_split:true,format:'ZpaqNativeSplit',archive_format:'ZPAQ',first_part:source,parts,total_parts:2,total_size:100,is_complete:true,missing_parts:[]}:undefined)
 const store=useArchiveConversionStore();store.openSnapshot(source,{version:1,date:20261022010000},'session-key');await flushPromises();expect(store.pending[0].sourceReference).toBe(source);expect(store.outputFor(store.pending[0]).replaceAll('\\','/')).toMatch(/^D:\/received\//);expect(store.effectiveSettings(store.pending[0]).sourceZpaqSnapshot?.version).toBe(1)
})
