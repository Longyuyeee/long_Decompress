import {it,expect,vi,beforeEach} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia} from 'pinia'
import LzopRecoveryDialog from '../LzopRecoveryDialog.vue'
import {copyExecutionConfig} from '@/types/executionConfig'
import type {LzopRecoveryReport,LzopRecoveryOutcome} from '@/types/lzopRecovery'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
const report=():LzopRecoveryReport=>({format:'TAR.LZO',revision:'a'.repeat(64),entries:[{id:0,name:'资料/负时间.txt',bytes:10,sha256:'1'.repeat(64),modifiedTime:{seconds:-2,nanoseconds:876543300}},{id:1,name:'精度.txt',bytes:10,sha256:'2'.repeat(64),modifiedTime:{seconds:1700001711,nanoseconds:123456789}}],complete:false,stopReason:'后续数据块不完整',stoppedMember:null,stoppedOffset:1024,availableVolumes:1,ignoredVolumes:0})
beforeEach(()=>mocks.invoke.mockReset())
it('time1711 keeps time details folded, shows content success with a time warning, and saves separate counts',async()=>{
 const initial=report();const outcome:LzopRecoveryOutcome={outputDirectory:'D:/result',recoveredFiles:2,recoveredBytes:20,entries:initial.entries.map(e=>e.id===1?{...e,timeWarning:'保存位置精度有限，文件内容已取回'}:e),stopReason:initial.stopReason,partial:true,timePreservation:{preservedFiles:1,unpreservedFiles:1}}
 mocks.invoke.mockImplementation(async(cmd:string)=>cmd==='recover_lzop_members'?outcome:null)
 const w=mount(LzopRecoveryDialog,{props:{path:'C:/broken.tar.lzo',initial,originTaskId:'original-failure'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}});await flushPromises()
 await w.get('[data-testid="lzop-recovery-confirm"]').trigger('click');await flushPromises()
 expect(w.get('[data-testid="lzop-recovery-result"]').text()).toContain('已部分取回 2 个文件')
 expect(w.get('[data-testid="recovery-time-details"]').attributes('open')).toBeUndefined()
 expect(w.get('[data-testid="recovery-time-summary"]').text()).toContain('1 个文件已保留，1 个未完整保留')
 expect(w.text()).toContain('1969-12-31 23:59:58.8765433 UTC')
 expect(w.text()).toContain('保存位置精度有限，文件内容已取回')
 expect(mocks.invoke).toHaveBeenCalledWith('save_task_history',{record:expect.objectContaining({status:'completed',retryOfTaskId:'original-failure',executionConfig:expect.objectContaining({timePreservation:outcome.timePreservation})})});w.unmount()
})
it('time1711 optional history counts stay backward compatible and reject impossible or fractional totals',()=>{
 const old={schemaVersion:1 as const,kind:'archive-recovery' as const,outputDirectory:'D:/result',recoveredFiles:2,stopReason:'tail',revision:'b'.repeat(64)}
 expect(copyExecutionConfig(old)).toEqual(old)
 const current={...old,timePreservation:{preservedFiles:1,unpreservedFiles:1}};expect(copyExecutionConfig(current)).toEqual(current)
 for(const timePreservation of[{preservedFiles:3,unpreservedFiles:0},{preservedFiles:1,unpreservedFiles:0},{preservedFiles:-1,unpreservedFiles:3},{preservedFiles:1.5,unpreservedFiles:.5}])expect(copyExecutionConfig({...old,timePreservation})).toBeNull()
})
