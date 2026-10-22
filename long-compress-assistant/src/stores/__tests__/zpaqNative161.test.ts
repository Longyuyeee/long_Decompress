import {beforeEach,expect,it,vi} from 'vitest'
import {flushPromises,mount} from '@vue/test-utils'
import {createPinia,setActivePinia} from 'pinia'
import {useArchiveConversionStore} from '../archiveConversion'
import ZpaqUpdateFields from '@/components/compression/ZpaqUpdateFields.vue'
import ArchiveOutputSummary from '@/components/compression/ArchiveOutputSummary.vue'
import {defaultConversionSettings} from '@/utils/archiveConversionDraft'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({ask:vi.fn(),open:vi.fn()}))
const source='C:/profile/zpaq-sources/id/journal01.zpaq',parts=['D:/received/journal01.zpaq','D:/received/journal02.zpaq','D:/received/journal03.zpaq']
beforeEach(()=>{setActivePinia(createPinia());mocks.invoke.mockReset();mocks.invoke.mockImplementation(async cmd=>cmd==='detect_split_archive'?{is_split:true,format:'ZpaqNativeSplit',archive_format:'ZPAQ',first_part:source,parts,total_parts:3,total_size:400000,is_complete:true,missing_parts:[]}:undefined)})
it('zpaq161 confirms a native source, inherits transaction output and applies shared settings without numeric slicing',async()=>{
 const store=useArchiveConversionStore();store.defaults.splitMiB=16;store.showUpdate([source]);await flushPromises();const item=store.pending[0]
 expect(item.sourceIssue).toBeUndefined();expect(item.sourceNativeParts).toBe(3);expect(store.effectiveSettings(item).splitMiB).toBe(0);expect(store.outputFor(item).replaceAll('\\','/')).toMatch(/^D:\/received\//)
 expect(await store.applySelected({level:3,splitMiB:8})).toBe('');expect(store.effectiveSettings(item)).toMatchObject({level:3,splitMiB:0});expect(store.batchNotice).toContain('按事务')
 expect(await store.applySelected({splitMiB:8})).toContain('不应用数字分卷')
})
it('zpaq161 shows compact native group rules and keeps an expandable list of actual parts',()=>{
 const fields=mount(ZpaqUpdateFields,{props:{settings:defaultConversionSettings(),nativeParts:3,baseBytes:400000}})
 expect(fields.get('[data-testid="zpaq-native-update-mode"]').text()).toContain('3 个原部分');expect(fields.find('[data-testid="zpaq-update-split"]').exists()).toBe(false);expect(fields.find('[data-testid="zpaq-update-add-files"]').exists()).toBe(true);fields.unmount()
 const output={primaryPath:'D:/new.part1.zpaq',files:[{path:'D:/new.part1.zpaq',bytes:100},{path:'D:/new.part2.zpaq',bytes:20}],totalBytes:120,split:true,updateSummary:{nativeParts:2,baseSnapshots:1,snapshots:2,updatedEntries:1,newFragments:1,unchanged:false,copiedBytes:100,inputBytes:10}}
 const result=mount(ArchiveOutputSummary,{props:{output},global:{plugins:[createPinia()]}});expect(result.text()).toContain('原生多部分组 · 2 个部分');expect(result.find('ol').exists()).toBe(false);result.unmount()
})
