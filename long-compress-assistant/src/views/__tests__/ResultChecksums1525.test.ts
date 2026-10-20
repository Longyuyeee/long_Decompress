import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import FileIntegrityView from '../FileIntegrityView.vue'
import ResultActions from '@/components/tasks/ResultActions.vue'
import { useAppStore } from '@/stores/app'
import { checksumBaseDirectory, checksumSources } from '@/utils/checksumSources'
import type { ChecksumCalculationEntry } from '@/types/checksum'

const mocks=vi.hoisted(()=>({invoke:vi.fn(),open:vi.fn(),save:vi.fn(),listen:vi.fn(),push:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:mocks.listen}))
vi.mock('@tauri-apps/api/dialog',()=>({open:mocks.open,save:mocks.save}))
vi.mock('vue-router',()=>({useRouter:()=>({push:mocks.push})}))
vi.mock('@/composables/useTauriCommands',()=>({useTauriCommands:()=>({invoke:mocks.invoke})}))

const paths=['C:/分享/甲/同名.zip.001','C:/分享/乙/同名.zip.001']
const entries=(statuses:string[]= ['success','success']):ChecksumCalculationEntry[]=>paths.map((path,i)=>({path,fileName:'同名.zip.001',algorithm:'SHA256',checksum:statuses[i]==='success'?'a'.repeat(64):'',status:statuses[i] as ChecksumCalculationEntry['status']}))
let pinia:ReturnType<typeof createPinia>
beforeEach(()=>{
  vi.clearAllMocks();localStorage.clear();pinia=createPinia();setActivePinia(pinia)
  mocks.listen.mockResolvedValue(()=>{});mocks.push.mockResolvedValue(undefined)
  mocks.invoke.mockImplementation(async(cmd:string)=>cmd==='calculate_checksum_batch'?entries():undefined)
  mocks.save.mockResolvedValue('C:/分享/checksums.sha256')
})
const view=async()=>{
  useAppStore().pendingChecksumSources={paths:[...paths],label:'同名.zip.001'}
  const w=mount(FileIntegrityView,{global:{plugins:[pinia]}});await flushPromises();return w
}

describe('result checksums v1.5.25',()=>{
  it('uses every physical published volume without scanning unrelated files or hashing',async()=>{
    const output={primaryPath:paths[0],split:true,totalBytes:6,files:paths.map(path=>({path,bytes:3}))}
    const w=mount(ResultActions,{props:{path:paths[0],output},global:{plugins:[pinia]}})
    await w.get('[data-testid="result-generate-checksums"]').trigger('click');await flushPromises()
    expect(useAppStore().pendingChecksumSources?.paths).toEqual(paths)
    expect(mocks.push).toHaveBeenCalledWith('/integrity')
    expect(mocks.invoke.mock.calls.some(([cmd])=>['detect_split_archive','calculate_checksum_batch'].includes(cmd))).toBe(false);w.unmount()
  })
  it('does not offer implicit recursive hashing of extraction directories',()=>{
    const w=mount(ResultActions,{props:{path:'C:/解压结果',directory:true},global:{plugins:[pinia]}})
    expect(w.find('[data-testid="result-generate-checksums"]').exists()).toBe(false);w.unmount()
  })
  it('resolves legacy split results and explains missing members instead of passing only the first',async()=>{
    mocks.invoke.mockResolvedValue({is_split:true,is_complete:true,parts:paths,missing_parts:[]})
    expect(await checksumSources(paths[1])).toEqual(paths)
    mocks.invoke.mockResolvedValue({is_split:true,is_complete:false,parts:[paths[0]],missing_parts:['同名.zip.002']})
    await expect(checksumSources(paths[0])).rejects.toThrow('补齐')
  })
  it('suggests the common parent and handles roots and different disks',()=>{
    expect(checksumBaseDirectory(paths)).toBe('C:/分享')
    expect(checksumBaseDirectory(['C:/甲/a','C:/乙/a'])).toBe('C:/')
    expect(checksumBaseDirectory(['C:/a','D:/a'])).toBe('')
    expect(checksumBaseDirectory(['//server/share/甲/a','//server/share/乙/a'])).toBe('//server/share')
  })
  it('waits for the user to start, distinguishes same names and keeps inline details',async()=>{
    const w=await view()
    expect(w.get('[data-testid="checksum-source-summary"]').text()).toContain('整组 2 个文件')
    expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='calculate_checksum_batch')).toBe(false)
    await w.get('[data-testid="checksum-calculate-start"]').trigger('click');await flushPromises()
    expect(mocks.invoke).toHaveBeenCalledWith('calculate_checksum_batch',expect.objectContaining({paths,algorithm:'sha256'}))
    expect(w.findAll('[data-testid="checksum-calculation-entry"]')).toHaveLength(2)
    await w.findAll('[data-testid="checksum-calculation-toggle"]')[1].trigger('click')
    expect(w.get('[data-testid="checksum-calculation-details"]').text()).toContain(paths[1]);w.unmount()
  })
  it('exports full paths and completed observations only, then verifies only on explicit click',async()=>{
    mocks.invoke.mockImplementation(async(cmd:string)=>cmd==='calculate_checksum_batch'?entries(['success','interrupted']):cmd==='verify_checksum_file'?{valid:true,message:'匹配',checksumPath:'C:/分享/checksums.sha256',manifestId:'id',entries:[],cancelled:false}:undefined)
    const w=await view();await w.get('[data-testid="checksum-calculate-start"]').trigger('click');await flushPromises()
    await w.get('[data-testid="checksum-export"]').trigger('click');await flushPromises()
    expect(mocks.save).toHaveBeenCalledWith(expect.objectContaining({defaultPath:'C:/分享/checksums.sha256'}))
    expect(mocks.invoke).toHaveBeenCalledWith('export_checksum_file',{path:'C:/分享/checksums.sha256',results:[{path:paths[0],checksum:'a'.repeat(64),status:'success'}],algorithm:'sha256'})
    expect(w.get('[data-testid="checksum-export-receipt"]').text()).toContain('其余 1 项未写入')
    expect(mocks.invoke.mock.calls.filter(([cmd])=>cmd==='calculate_checksum_batch')).toHaveLength(1)
    expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='verify_checksum_file')).toBe(false)
    await w.get('[data-testid="checksum-verify-exported"]').trigger('click');await flushPromises()
    expect(mocks.invoke).toHaveBeenCalledWith('verify_checksum_file',expect.objectContaining({checksumPath:'C:/分享/checksums.sha256'}));w.unmount()
  })
  it('keeps export failure guidance visible and preserves calculated results',async()=>{
    const w=await view();await w.get('[data-testid="checksum-calculate-start"]').trigger('click');await flushPromises()
    mocks.invoke.mockImplementation(async(cmd:string)=>{if(cmd==='export_checksum_file')throw Error('请保存到共同上级目录')})
    await w.get('[data-testid="checksum-export"]').trigger('click');await flushPromises()
    expect(w.get('[data-testid="checksum-export-error"]').text()).toContain('共同上级目录')
    expect(w.findAll('[data-testid="checksum-calculation-entry"]')).toHaveLength(2)
    expect(w.find('[data-testid="checksum-export-receipt"]').exists()).toBe(false);w.unmount()
  })
  it('does not replace an active or completed workspace when another result arrives',async()=>{
    let finish!:(rows:ChecksumCalculationEntry[])=>void
    mocks.invoke.mockImplementation(async(cmd:string)=>cmd==='calculate_checksum_batch'?new Promise<ChecksumCalculationEntry[]>(resolve=>{finish=resolve}):undefined)
    const w=await view();await w.get('[data-testid="checksum-calculate-start"]').trigger('click');await flushPromises()
    useAppStore().pendingChecksumSources={paths:['D:/新的结果.zip'],label:'新的结果'};await flushPromises()
    expect(w.get('[data-testid="checksum-pending-sources"] button').attributes('disabled')).toBeDefined()
    finish(entries());await flushPromises()
    expect(w.findAll('[data-testid="checksum-calculation-entry"]')).toHaveLength(2)
    expect(useAppStore().pendingChecksumSources?.paths).toEqual(['D:/新的结果.zip'])
    await w.get('[data-testid="checksum-pending-sources"] button').trigger('click');await flushPromises()
    expect(w.get('[data-testid="checksum-source-summary"]').text()).toContain('新的结果')
    expect(w.find('[data-testid="checksum-calculation-entry"]').exists()).toBe(false);w.unmount()
  })
})
