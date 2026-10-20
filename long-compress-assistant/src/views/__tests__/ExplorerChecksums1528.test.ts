import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import View from '../FileIntegrityView.vue'
import Panel from '@/components/integrity/ChecksumVerificationPanel.vue'
import { useAppStore } from '@/stores/app'
const mocks=vi.hoisted(()=>({invoke:vi.fn(),listen:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:mocks.listen}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn(),save:vi.fn()}))
vi.mock('vue-router',()=>({useRouter:()=>({push:vi.fn()})}))
vi.mock('@/composables/useTauriCommands',()=>({useTauriCommands:()=>({invoke:mocks.invoke})}))
let pinia:ReturnType<typeof createPinia>
const report=(path:string)=>({checksumPath:path,manifestId:path,valid:true,message:'一致',entries:[],cancelled:false})
beforeEach(()=>{vi.clearAllMocks();localStorage.clear();pinia=createPinia();setActivePinia(pinia);mocks.listen.mockResolvedValue(()=>{});mocks.invoke.mockImplementation(async(cmd:string,args:any)=>cmd==='verify_checksum_file'?report(args.checksumPath):[])})
describe('explorer checksum 1528',()=>{
  it('reports an actual read failure without a green success notification',async()=>{
    const app=useAppStore();app.enqueueExplorerChecksums({kind:'calculate',paths:['C:/已移动.bin']})
    mocks.invoke.mockImplementation(async(cmd:string)=>cmd==='calculate_checksum_batch'?[{path:'C:/已移动.bin',fileName:'已移动.bin',algorithm:'SHA256',checksum:'',status:'error',error:'找不到文件'}]:undefined)
    const w=mount(View,{global:{plugins:[pinia]}});await flushPromises();await w.get('[data-testid="checksum-calculate-start"]').trigger('click');await flushPromises()
    expect(app.error).toContain('1 个文件未能计算');expect(app.successMessage).toBeNull();expect(w.get('[data-status="error"]').text()).toContain('找不到文件');w.unmount()
  })

  it('stages multiple manifests on first navigation and reads only the explicitly chosen one',async()=>{
    const app=useAppStore();app.enqueueExplorerChecksums({kind:'verify',paths:['C:/甲.sha256','D:/乙.md5']})
    const w=mount(View,{global:{plugins:[pinia]}});await flushPromises()
    expect(w.get('[data-testid="checksum-explorer-manifests"]').text()).toContain('2 份')
    expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='verify_checksum_file')).toBe(false)
    await w.get('[data-testid="checksum-staged-path"]').setValue('D:/乙.md5');await w.get('[data-testid="checksum-staged-start"]').trigger('click');await flushPromises()
    expect(mocks.invoke).toHaveBeenCalledWith('verify_checksum_file',expect.objectContaining({checksumPath:'D:/乙.md5'}));expect(w.findAll('[data-testid="checksum-staged-path"] option')).toHaveLength(2);w.unmount()
  })
  it('preserves completed calculation until the user accepts incoming manifests',async()=>{
    const app=useAppStore();app.enqueueExplorerChecksums({kind:'calculate',paths:['C:/资料.bin']})
    const w=mount(View,{global:{plugins:[pinia]}});await flushPromises()
    expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='calculate_checksum_batch')).toBe(false)
    await w.get('[data-testid="checksum-calculate-start"]').trigger('click');await flushPromises()
    app.enqueueExplorerChecksums({kind:'verify',paths:['C:/资料.sha256']});await flushPromises()
    expect(w.find('[data-testid="checksum-explorer-pending"]').exists()).toBe(true)
    expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='verify_checksum_file')).toBe(false)
    await w.get('[data-testid="checksum-explorer-use"]').trigger('click');await flushPromises()
    expect(w.find('[data-testid="checksum-explorer-manifests"]').exists()).toBe(true);await w.get('.integrity-mode-switch button').trigger('click');await flushPromises();expect(w.get('[data-testid="checksum-source-summary"]').text()).toContain('资源管理器');expect(mocks.invoke.mock.calls.filter(([cmd])=>cmd==='calculate_checksum_batch')).toHaveLength(1);w.unmount()
  })
  it('keeps busy work and merges later selections without duplicate paths',async()=>{
    const app=useAppStore();app.enqueueExplorerChecksums({kind:'calculate',paths:['C:/资料.bin']})
    const w=mount(View,{global:{plugins:[pinia]}});await flushPromises()
    let finish!:(v:any)=>void;mocks.invoke.mockImplementation(async(cmd:string)=>cmd==='calculate_checksum_batch'?new Promise(resolve=>finish=resolve):undefined)
    await w.get('[data-testid="checksum-calculate-start"]').trigger('click');await flushPromises()
    app.enqueueExplorerChecksums({kind:'calculate',paths:['c:/资料.bin','C:/新增.txt']});await flushPromises()
    expect(w.get('[data-testid="checksum-explorer-append"]').attributes('disabled')).toBeDefined()
    finish([]);await flushPromises();await w.get('[data-testid="checksum-explorer-append"]').trigger('click');await flushPromises()
    await w.get('[data-testid="checksum-calculate-start"]').trigger('click');await flushPromises()
    expect(mocks.invoke).toHaveBeenLastCalledWith('calculate_checksum_batch',expect.objectContaining({paths:['C:/资料.bin','C:/新增.txt']}));finish([]);await flushPromises();w.unmount()
  })
  it('queues later manifest bursts and retains the last report while choosing another',async()=>{
    const w=mount(Panel,{global:{plugins:[pinia]}})
    w.vm.stage(['C:/甲.sha256']);await flushPromises();await w.get('[data-testid="checksum-staged-start"]').trigger('click');await flushPromises()
    w.vm.stage(['c:/甲.sha256','D:/乙.sfv']);await flushPromises()
    expect(w.findAll('option')).toHaveLength(2);expect((w.get('[data-testid="checksum-staged-path"]').element as HTMLSelectElement).value).toBe('C:/甲.sha256');expect(w.get('[data-testid="checksum-report"]').text()).toContain('C:/甲.sha256')
    await w.get('[data-testid="checksum-staged-path"]').setValue('D:/乙.sfv');await flushPromises()
    expect(mocks.invoke.mock.calls.filter(([cmd])=>cmd==='verify_checksum_file')).toHaveLength(1);w.unmount()
  })
})
