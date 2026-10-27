import {it,expect,vi,beforeEach} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia} from 'pinia'
import LzopRecoveryDialog from '../LzopRecoveryDialog.vue'
import {copyExecutionConfig} from '@/types/executionConfig'
import {recoveryTimeDescription,type LzopRecoveryReport,type LzopRecoveryOutcome} from '@/types/lzopRecovery'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
beforeEach(()=>mocks.invoke.mockReset())
it('directory1712 a directory-only result shows folded directory times and warnings and retains counts in history',async()=>{
 const initial:LzopRecoveryReport={format:'TAR.LZO',revision:'a'.repeat(64),entries:[{id:0,name:'资料',bytes:0,sha256:'b'.repeat(64),isDirectory:true,modifiedTime:{seconds:1700001712,nanoseconds:125000000}},{id:1,name:'资料/empty',bytes:0,sha256:'b'.repeat(64),isDirectory:true,timeWarning:'目录原修改时间缺失，内容仍保留'}],complete:false,stopReason:'后续缺卷',stoppedMember:null,stoppedOffset:1024,availableVolumes:1,ignoredVolumes:1}
 const timePreservation={preservedFiles:0,unpreservedFiles:0,preservedDirectories:1,unpreservedDirectories:1}
 const result:LzopRecoveryOutcome={outputDirectory:'D:/result',recoveredFiles:0,recoveredDirectories:2,recoveredBytes:0,entries:initial.entries,stopReason:initial.stopReason,partial:true,timePreservation}
 mocks.invoke.mockImplementation(async(cmd:string)=>cmd==='recover_lzop_members'?result:null)
 const w=mount(LzopRecoveryDialog,{props:{path:'C:/dirs.tar.lzo',initial,originTaskId:'failed-directory-original'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}});await flushPromises();await w.get('[data-testid="lzop-recovery-confirm"]').trigger('click');await flushPromises()
 expect(w.get('[data-testid="recovery-time-details"]').attributes('open')).toBeUndefined();expect(w.get('[data-testid="recovery-time-summary"]').text()).toBe('原修改时间：1 个目录已保留，1 个未完整保留。');expect(w.text()).toContain('2023-11-14 22:41:52.125 UTC');expect(w.text()).toContain('目录原修改时间缺失，内容仍保留')
 expect(mocks.invoke).toHaveBeenCalledWith('save_task_history',{record:expect.objectContaining({status:'completed',retryOfTaskId:'failed-directory-original',executionConfig:expect.objectContaining({recoveredFiles:0,recoveredDirectories:2,timePreservation})})});w.unmount()
})
it('directory1712 history keeps optional paired directory totals and does not invent them for older records',()=>{
 const old={schemaVersion:1 as const,kind:'archive-recovery' as const,outputDirectory:'D:/result',recoveredFiles:1,recoveredDirectories:2,stopReason:'tail',revision:'a'.repeat(64),timePreservation:{preservedFiles:1,unpreservedFiles:0}}
 expect(copyExecutionConfig(old)).toEqual(old);expect(recoveryTimeDescription(old.timePreservation)).toBe('原修改时间：1 个文件已保留。')
 const current={...old,timePreservation:{...old.timePreservation,preservedDirectories:1,unpreservedDirectories:1}};expect(copyExecutionConfig(current)).toEqual(current);expect(recoveryTimeDescription(current.timePreservation)).toBe('原修改时间：1 个文件已保留；1 个目录已保留，1 个未完整保留。')
 for(const counts of [{preservedDirectories:3,unpreservedDirectories:0},{preservedDirectories:1,unpreservedDirectories:0},{preservedDirectories:-1,unpreservedDirectories:3},{preservedDirectories:1.5,unpreservedDirectories:.5},{preservedDirectories:1}])expect(copyExecutionConfig({...old,timePreservation:{...old.timePreservation,...counts}})).toBeNull()
})
