import {it,expect,vi,beforeEach} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia} from 'pinia'
import LzopRecoveryDialog from '../LzopRecoveryDialog.vue'
import {copyExecutionConfig} from '@/types/executionConfig'
import type {LzopRecoveryReport} from '@/types/lzopRecovery'
const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
const report=():LzopRecoveryReport=>({format:'TAR.LZO',revision:'a'.repeat(64),entries:[{id:0,name:'资料/空目录',bytes:0,sha256:'1'.repeat(64),isDirectory:true},{id:1,name:'资料/说明.txt',bytes:10,sha256:'2'.repeat(64),isDirectory:false}],complete:false,stopReason:'外层下一块未完整到达',stoppedMember:'late.bin',stoppedOffset:1024,availableVolumes:2,ignoredVolumes:1})
const create=()=>mount(LzopRecoveryDialog,{props:{path:'C:/source.tar.lzo.004',initial:report(),originTaskId:'failed-original'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}})
beforeEach(()=>{mocks.invoke.mockReset()})
it('tarlzo175 reuses the diagnosis, distinguishes folders, and keeps secondary details folded',async()=>{
 const w=create();await flushPromises();expect(mocks.invoke).not.toHaveBeenCalled();expect(w.text()).toContain('TAR.LZO 部分取回');expect(w.get('[data-testid="lzop-recovery-count"]').text()).toContain('1 个完整文件');expect(w.get('[data-testid="lzop-recovery-count"]').text()).toContain('1 个目录');expect(w.get('[data-recovery-member="0"]').text()).toContain('目录');expect(w.get('.stop-detail').attributes('open')).toBeUndefined();w.unmount()
})
it('tarlzo175 an empty folder can be taken alone without counting it as a file or selecting children',async()=>{
 const w=create();await flushPromises();await w.get('[data-recovery-member="1"] input').setValue(false)
 mocks.invoke.mockImplementation(async(cmd:string)=>cmd==='recover_lzop_members'?{outputDirectory:'D:/result/部分取回',recoveredFiles:0,recoveredDirectories:1,recoveredBytes:0,entries:report().entries.slice(0,1),stopReason:report().stopReason,partial:true}:null)
 await w.get('[data-testid="lzop-recovery-output"]').setValue('D:/result');await w.get('[data-testid="lzop-recovery-confirm"]').trigger('click');await flushPromises()
 expect(mocks.invoke).toHaveBeenCalledWith('recover_lzop_members',expect.objectContaining({selected:[0]}));expect(w.get('[data-testid="lzop-recovery-result"]').text()).toContain('0 个文件、1 个目录');expect(mocks.invoke).toHaveBeenCalledWith('save_task_history',{record:expect.objectContaining({format:'TAR.LZO',retryOfTaskId:'failed-original',executionConfig:expect.objectContaining({recoveredFiles:0,recoveredDirectories:1})})});w.unmount()
})
it('tarlzo175 recovery history copies directory counts and rejects invalid counts',()=>{
 const config={schemaVersion:1 as const,kind:'archive-recovery' as const,outputDirectory:'D:/result',recoveredFiles:0,recoveredDirectories:1,stopReason:'incomplete tail',revision:'b'.repeat(64)}
 expect(copyExecutionConfig(config)).toEqual(config);expect(copyExecutionConfig({...config,recoveredDirectories:-1})).toBeNull();expect(copyExecutionConfig({...config,recoveredDirectories:100001})).toBeNull()
})

it('tarlzo175 selecting a folder includes only confirmed descendants',async()=>{
 const initial=report();initial.entries[0].name='资料';initial.entries.push({id:2,name:'elsewhere.txt',bytes:4,sha256:'c'.repeat(64)});
 const w=mount(LzopRecoveryDialog,{props:{path:'C:/source.tar.lzo',initial},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}});await flushPromises();await w.get('[data-recovery-member="0"] input').setValue(false);expect((w.get('[data-recovery-member="1"] input').element as HTMLInputElement).checked).toBe(false);expect((w.get('[data-recovery-member="2"] input').element as HTMLInputElement).checked).toBe(true);await w.get('[data-recovery-member="0"] input').setValue(true);expect((w.get('[data-recovery-member="1"] input').element as HTMLInputElement).checked).toBe(true);w.unmount()
})
