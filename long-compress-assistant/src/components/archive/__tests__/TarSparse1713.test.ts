import {it,expect,vi,beforeEach} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia} from 'pinia'
import LzopRecoveryDialog from '../LzopRecoveryDialog.vue'
import type {LzopRecoveryReport,LzopRecoveryOutcome} from '@/types/lzopRecovery'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
beforeEach(()=>mocks.invoke.mockReset())
const report=():LzopRecoveryReport=>({format:'TAR.LZO',revision:'a'.repeat(64),entries:[{id:0,name:'资料/全零.bin',bytes:1048576,storedBytes:0,sha256:'b'.repeat(64),modifiedTime:{seconds:1700001713,nanoseconds:0}},{id:1,name:'after.txt',bytes:30,sha256:'c'.repeat(64)}],complete:false,stopReason:'后续数据不完整',stoppedMember:'late.bin',stoppedOffset:4096,availableVolumes:2,ignoredVolumes:1})
it('sparse1713 list separates logical bytes from stored data, including a zero-data sparse file',async()=>{
 const w=mount(LzopRecoveryDialog,{props:{path:'C:/sparse.tar.lzo',initial:report()},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}});await flushPromises();expect(w.get('[data-recovery-member="0"]').text()).toContain('稀疏文件 · 包内数据 0 B');expect(w.get('[data-recovery-member="0"]').text()).toContain('1,048,576 B');expect(w.get('[data-recovery-member="1"]').text()).not.toContain('稀疏');expect(w.get('.stop-detail').attributes('open')).toBeUndefined();w.unmount()
})
it('sparse1713 recovery keeps original folded result and history counts without claiming a full archive',async()=>{
 const initial=report();const result:LzopRecoveryOutcome={outputDirectory:'D:/result',recoveredFiles:2,recoveredDirectories:0,recoveredBytes:1048606,entries:initial.entries,partial:true,stopReason:initial.stopReason,timePreservation:{preservedFiles:2,unpreservedFiles:0,preservedDirectories:0,unpreservedDirectories:0}};mocks.invoke.mockImplementation(async(cmd:string)=>cmd==='recover_lzop_members'?result:null)
 const w=mount(LzopRecoveryDialog,{props:{path:'C:/sparse.tar.lzo',initial,originTaskId:'sparse-original-failure'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}});await flushPromises();await w.get('[data-testid="lzop-recovery-confirm"]').trigger('click');await flushPromises();expect(w.get('[data-testid="lzop-recovery-result"]').text()).toContain('已部分取回 2 个文件');expect(w.get('[data-testid="recovery-time-details"]').attributes('open')).toBeUndefined();expect(w.text()).toContain('完整大小 1,048,576 B · 包内数据 0 B');expect(mocks.invoke).toHaveBeenCalledWith('save_task_history',{record:expect.objectContaining({retryOfTaskId:'sparse-original-failure',processedBytes:1048606,executionConfig:expect.objectContaining({kind:'archive-recovery',recoveredFiles:2})})});w.unmount()
})
