import {beforeEach,expect,it,vi} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia,setActivePinia} from 'pinia'
import {createMemoryHistory,createRouter} from 'vue-router'
import ZpaqSnapshotDiff from '../ZpaqSnapshotDiff.vue'
import {defaultDiffSelection,selectedDiffGroups,mergeVerifiedComparison,type ZpaqComparison} from '@/types/zpaqDiff'
import {copyExecutionConfig} from '@/types/executionConfig'
import {useTaskStore} from '@/stores/task'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
const one={version:1,date:20261020130000},two={version:2,date:20261020140000},side={size:4,date:20261020130000,directory:false,selectable:true}
function comparison():ZpaqComparison{return{before:one,after:two,beforeCatalogId:'a'.repeat(64),afterCatalogId:'b'.repeat(64),revision:'c'.repeat(64),counts:{added:1,removed:1,candidate:1},entries:[{path:'new.txt',kind:'added',reason:'仅新版',before:null,after:side,verified:false},{path:'old.txt',kind:'removed',reason:'旧版',before:side,after:null,verified:false},{path:'same.txt',kind:'candidate',reason:'待核对',before:side,after:side,verified:false}]}}
beforeEach(()=>{setActivePinia(createPinia());mocks.invoke.mockReset();mocks.invoke.mockImplementation(async cmd=>cmd==='compare_zpaq_snapshots'?comparison():cmd==='detect_split_archive'?{is_split:true,is_complete:true,first_part:'C:/archive.zpaq.001',missing_parts:[]}:cmd==='get_file_info'?{is_dir:true}:cmd==='load_app_settings'?'{}':undefined)})
function create(){const pinia=createPinia();setActivePinia(pinia);const router=createRouter({history:createMemoryHistory(),routes:[{path:'/',component:{template:'<div />'}},{path:'/decompress',component:{template:'<div />'}}]});return mount(ZpaqSnapshotDiff,{props:{visible:true,path:'C:/archive.zpaq.002',password:'native-secret-1537',history:{selected:two,snapshots:[{...one,updates:2,deletes:0},{...two,updates:2,deletes:1}]},outputDirectory:'D:/out'},global:{plugins:[pinia,router],stubs:{Teleport:true,Transition:false}}})}
it('zpaq1537 separates new/removed versions and retains exact guarded ranges through config copies',()=>{
  const result=comparison(),groups=selectedDiffGroups(result,defaultDiffSelection(result));expect(groups.map(g=>[g.snapshot.version,g.paths])).toEqual([[1,['old.txt']],[2,['new.txt','same.txt']]])
  const config={schemaVersion:1 as const,kind:'archive-extraction' as const,zpaqSnapshot:one,zpaqCatalogId:result.beforeCatalogId,selectedEntries:['old.txt'],outputDirectory:'D:/out',extractToSubfolder:false};const copy=copyExecutionConfig(config);config.selectedEntries.push('extra');expect(copy).toMatchObject({selectedEntries:['old.txt']});expect(copyExecutionConfig({...config,zpaqCatalogId:'bad'})).toBeNull();expect(copyExecutionConfig({...config,selectedEntries:[]})).toBeNull()
})
it('zpaq1537 remembers actual verified results only for the same archive revision',()=>{
  const old=comparison();old.entries[2]={...old.entries[2],kind:'changed',reason:'实际SHA256不同',verified:true};const merged=mergeVerifiedComparison(old,comparison());expect(merged.entries[2].verified).toBe(true);expect(merged.counts.changed).toBe(1)
  expect(mergeVerifiedComparison(old,{...comparison(),revision:'d'.repeat(64)}).entries[2].kind).toBe('candidate')
})
it('zpaq1537 creates two independent pending tasks, never starts extraction, and keeps original password out of history config',async()=>{
  const wrapper=create();await wrapper.get('[data-testid="zpaq-diff-compare"]').trigger('click');await flushPromises();expect(wrapper.findAll('[data-diff-path]')).toHaveLength(3)
  await wrapper.get('[data-testid="zpaq-diff-create"]').trigger('click');await flushPromises();const tasks=useTaskStore().tasks;expect(tasks).toHaveLength(2);expect(tasks.every(t=>t.status==='pending'&&t.sourceFiles[0]==='C:/archive.zpaq.001')).toBe(true);expect(tasks.map(t=>t.zpaqSnapshot)).toEqual([one,two]);expect(tasks[0].selectedEntries).toEqual(['old.txt']);expect(new Set(tasks.map(t=>t.outputPath)).size).toBe(2);expect(JSON.stringify(tasks.map(t=>t.executionConfig))).not.toContain('native-secret-1537');expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='extract_file')).toBe(false);wrapper.unmount()
})
it('zpaq1537 ignores a late cancelled comparison and clears stale results when precise verification fails',async()=>{
  let resolve!:(v:ZpaqComparison)=>void;mocks.invoke.mockImplementation(async cmd=>cmd==='compare_zpaq_snapshots'?new Promise<ZpaqComparison>(r=>{resolve=r}):undefined)
  const wrapper=create();await wrapper.get('[data-testid="zpaq-diff-compare"]').trigger('click');await flushPromises();await wrapper.get('[data-testid="zpaq-diff-stop"]').trigger('click');resolve(comparison());await flushPromises();expect(wrapper.find('[data-testid="zpaq-diff-entries"]').exists()).toBe(false)
  mocks.invoke.mockResolvedValueOnce(comparison());await wrapper.get('[data-testid="zpaq-diff-compare"]').trigger('click');await flushPromises();mocks.invoke.mockRejectedValueOnce(Error('来源已变化'));await wrapper.get('[data-testid="zpaq-diff-verify"]').trigger('click');await flushPromises();expect(wrapper.get('[role="alert"]').text()).toContain('来源已变化');expect(wrapper.find('[data-testid="zpaq-diff-entries"]').exists()).toBe(false);wrapper.unmount()
})
it('zpaq1537 displays at most 200 rows and can select just the next page',async()=>{
  const large=comparison();large.entries=Array.from({length:230},(_,i)=>({...large.entries[0],path:`${i}.txt`}));large.counts={added:230};mocks.invoke.mockResolvedValue(large)
  const wrapper=create();await wrapper.get('[data-testid="zpaq-diff-compare"]').trigger('click');await flushPromises();expect(wrapper.findAll('[data-diff-path]')).toHaveLength(200);await wrapper.findAll('button').find(b=>b.text()==='下一页')!.trigger('click');expect(wrapper.findAll('[data-diff-path]')).toHaveLength(30);await wrapper.findAll('button').find(b=>b.text()==='选择本页变化')!.trigger('click');expect(wrapper.text()).toContain('已选 30');wrapper.unmount()
})
it('zpaq1537 closing while source checks are pending cannot create a stale draft',async()=>{
  let release!:()=>void;const wrapper=create();await wrapper.get('[data-testid="zpaq-diff-compare"]').trigger('click');await flushPromises()
  mocks.invoke.mockImplementation(async cmd=>cmd==='detect_split_archive'?{is_split:false}:cmd==='get_file_info'?{is_dir:true}:cmd==='check_zpaq_snapshot_catalog'?new Promise<void>(r=>{release=r}):undefined)
  await wrapper.get('[data-testid="zpaq-diff-create"]').trigger('click');await flushPromises();await wrapper.setProps({visible:false});mocks.invoke.mockResolvedValue(undefined);release();await flushPromises();expect(useTaskStore().tasks).toHaveLength(0);wrapper.unmount()
})
