import {it,expect,vi,beforeEach} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia} from 'pinia'
import LzopRecoveryDialog from '../LzopRecoveryDialog.vue'
import original from '../../../../tests/fixtures/archives/tar-pax10-1714/manifest.json'
import type {LzopRecoveryReport} from '@/types/lzopRecovery'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
beforeEach(()=>mocks.invoke.mockReset())
const report=():LzopRecoveryReport=>({format:'TAR.LZO',revision:'a'.repeat(64),entries:original.entries.slice(0,9).map((e,id)=>({...e,id,...(original.records[id].mapBytes?{storedBytes:original.records[id].storedBytes}:{})})),complete:false,stopReason:'late.bin 数据不完整',stoppedMember:'late.bin',stoppedOffset:1234,availableVolumes:8,ignoredVolumes:1})
const create=()=>mount(LzopRecoveryDialog,{props:{path:'C:/pax10.tar.lzo.017',initial:report()},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}})
it('pax1714 displays real GNU names, logical size and zero stored bytes in the original compact list',async()=>{
 const w=create();await flushPromises();expect(w.get('[data-recovery-member="7"]').text()).toContain('跨块映射.bin');expect(w.get('[data-recovery-member="7"]').text()).toContain('16,777,216 B');expect(w.get('[data-recovery-member="5"]').text()).toContain('包内数据 0 B');expect(w.text()).not.toContain('GNUSparseFile');expect(w.get('.stop-detail').attributes('open')).toBeUndefined();w.unmount()
})
it('pax1714 selected sparse files and a folder preserve folded results and logical byte history',async()=>{
 const entries=report().entries.filter(e=>[2,4,6,7].includes(e.id));mocks.invoke.mockImplementation(async cmd=>cmd==='recover_lzop_members'?{outputDirectory:'D:/result',recoveredFiles:3,recoveredDirectories:1,recoveredBytes:92274688,entries,partial:true,stopReason:'late.bin 数据不完整'}:null)
 const w=create();await flushPromises();await w.findAll('button').find(b=>b.text()==='取消所列')!.trigger('click');for(const id of[2,4,6,7])await w.get(`[data-recovery-member="${id}"] input`).setValue(true)
 await w.get('[data-testid="lzop-recovery-confirm"]').trigger('click');await flushPromises();expect(mocks.invoke).toHaveBeenCalledWith('recover_lzop_members',expect.objectContaining({selected:[2,4,6,7]}));expect(w.get('[data-testid="recovery-time-details"]').attributes('open')).toBeUndefined();expect(mocks.invoke).toHaveBeenCalledWith('save_task_history',{record:expect.objectContaining({processedBytes:92274688,executionConfig:expect.objectContaining({recoveredFiles:3,recoveredDirectories:1})})});w.unmount()
})
