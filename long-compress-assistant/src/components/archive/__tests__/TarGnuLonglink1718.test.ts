import {it,expect,vi,beforeEach} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia} from 'pinia'
import LzopRecoveryDialog from '../LzopRecoveryDialog.vue'
import original from '../../../../tests/fixtures/archives/tar-gnu-longlink-1718/manifest.json'
import type {LzopRecoveryReport} from '@/types/lzopRecovery'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
beforeEach(()=>mocks.invoke.mockReset())
const report=():LzopRecoveryReport=>({format:'TAR.LZO',revision:'a'.repeat(64),entries:original.entries.slice(0,14).map((e,id)=>{const {targetId,...fields}=e;const record=original.records[typeof targetId==='number'?targetId:id];return{...fields,id,...(record.ranges.length?{storedBytes:record.storedBytes}:{})}}),complete:false,stopReason:'late.bin 内容不完整',stoppedMember:'late.bin',stoppedOffset:1234,availableVolumes:8,ignoredVolumes:1})
const create=()=>mount(LzopRecoveryDialog,{props:{path:'C:/links.tar.lzo.019',initial:report()},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false,ResultActions:true}}})
it('gnu1718 compact rows show target content size, full source in title, and zero-byte aliases',async()=>{
 const w=create();await flushPromises();expect(w.get('[data-recovery-member="4"]').text()).toContain('链接内容 · source.bin');expect(w.get('[data-recovery-member="4"]').text()).toContain('200,003 B');expect(w.get('[data-recovery-member="7"] small[title]').attributes('title')).toBe(original.entries[6].name);expect(w.get('[data-recovery-member="9"]').text()).toContain('0 B');expect(w.get('[data-recovery-member="3"]').text()).not.toContain('链接内容');expect(w.get('.stop-detail').attributes('open')).toBeUndefined();w.unmount()
})
it('gnu1718 aliases alone keep independent file counts, logical bytes and original folded history',async()=>{
 const ids=[2,4,5,7,9,11,12],entries=report().entries.filter(e=>ids.includes(e.id));mocks.invoke.mockImplementation(async cmd=>cmd==='recover_lzop_members'?{outputDirectory:'D:/result',recoveredFiles:6,recoveredDirectories:1,recoveredBytes:33954438,entries,partial:true,stopReason:'late.bin 内容不完整'}:null)
 const w=create();await flushPromises();await w.findAll('button').find(b=>b.text()==='取消所列')!.trigger('click');for(const id of ids)await w.get(`[data-recovery-member="${id}"] input`).setValue(true);await w.get('[data-testid="lzop-recovery-confirm"]').trigger('click');await flushPromises();expect(mocks.invoke).toHaveBeenCalledWith('recover_lzop_members',expect.objectContaining({selected:ids}));expect(w.get('[data-testid="recovery-time-details"]').attributes('open')).toBeUndefined();expect(w.text()).toContain('内容来源：source.bin');expect(mocks.invoke).toHaveBeenCalledWith('save_task_history',{record:expect.objectContaining({processedBytes:33954438,executionConfig:expect.objectContaining({recoveredFiles:6,recoveredDirectories:1})})});w.unmount()
})
