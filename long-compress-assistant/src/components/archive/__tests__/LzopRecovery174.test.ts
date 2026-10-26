import {it,expect,vi,beforeEach} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia} from 'pinia'
import LzopRecoveryDialog from '../LzopRecoveryDialog.vue'
import {copyExecutionConfig} from '@/types/executionConfig'
import {restoredExtractionConfig,type TaskHistoryRecord} from '@/types/taskHistory'
import type {LzopRecoveryReport} from '@/types/lzopRecovery'
const mocks=vi.hoisted(()=>({invoke:vi.fn(),listener:null as null|((e:any)=>void)}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async(_event:string,callback:any)=>{mocks.listener=callback;return vi.fn()})}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
const report=():LzopRecoveryReport=>({revision:'a'.repeat(64),entries:[{id:0,name:'资料/说明.txt',bytes:10,sha256:'1'.repeat(64)},{id:1,name:'资料/空文件.txt',bytes:0,sha256:'2'.repeat(64)},{id:2,name:'图标.png',bytes:20,sha256:'3'.repeat(64)}],complete:false,stopReason:'后续成员数据不完整',stoppedMember:'late.bin',stoppedOffset:99,availableVolumes:3,ignoredVolumes:1})
const create=(initial=report())=>mount(LzopRecoveryDialog,{props:{path:'C:/source.lzo.003',initial},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}})
beforeEach(()=>{mocks.invoke.mockReset();mocks.invoke.mockImplementation(async(cmd:string)=>cmd==='save_task_history'?undefined:cmd==='scan_lzop_recovery'?{...report(),revision:'b'.repeat(64)}:null)})
it('lzop174 reuses diagnosis and saves only selected complete files with distinct history',async()=>{
 const w=create();await flushPromises();expect(mocks.invoke).not.toHaveBeenCalled();expect(w.get('.stop-detail').attributes('open')).toBeUndefined()
 await w.get('[data-recovery-member="2"] input').setValue(false)
 mocks.invoke.mockImplementation(async(cmd:string)=>cmd==='recover_lzop_members'?{outputDirectory:'D:/out/source.部分取回-123',recoveredFiles:2,recoveredBytes:10,entries:report().entries.slice(0,2),stopReason:report().stopReason,partial:true}:null)
 await w.get('[data-testid="lzop-recovery-output"]').setValue('D:/out');await w.get('[data-testid="lzop-recovery-confirm"]').trigger('click');await flushPromises()
 expect(mocks.invoke).toHaveBeenCalledWith('recover_lzop_members',expect.objectContaining({filePath:'C:/source.lzo.003',outputDirectory:'D:/out',selected:[0,1],revision:'a'.repeat(64)}))
 expect(w.get('[data-testid="lzop-recovery-result"]').text()).toContain('已部分取回 2 个文件');expect(w.find('[data-testid="lzop-recovery-confirm"]').exists()).toBe(false)
 expect(mocks.invoke).toHaveBeenCalledWith('save_task_history',{record:expect.objectContaining({name:expect.stringContaining('部分取回'),outputPath:'D:/out/source.部分取回-123',executionConfig:expect.objectContaining({kind:'archive-recovery',recoveredFiles:2,stopReason:report().stopReason})})});w.unmount()
})
it('lzop174 a changed source requires recheck and never auto-retries recovery',async()=>{
 const w=create();await flushPromises();mocks.invoke.mockImplementation(async(cmd:string)=>{if(cmd==='recover_lzop_members')throw Error('来源已变化，请重新检查');if(cmd==='scan_lzop_recovery')return {...report(),revision:'b'.repeat(64)}})
 await w.get('[data-testid="lzop-recovery-confirm"]').trigger('click');await flushPromises();expect(w.get('[data-testid="lzop-recovery-error"]').text()).toContain('重新检查');expect(w.find('[data-testid="lzop-recovery-result"]').exists()).toBe(false)
 await w.get('[data-testid="lzop-recovery-recheck"]').trigger('click');await flushPromises();expect(mocks.invoke.mock.calls.filter(c=>c[0]==='recover_lzop_members')).toHaveLength(1);w.unmount()
})
it('lzop174 cancellation keeps progress meaningful and persists a cancelled operation',async()=>{
 let reject:((e:Error)=>void)|undefined
 mocks.invoke.mockImplementation((cmd:string)=>{if(cmd==='recover_lzop_members')return new Promise((_,r)=>{reject=r});if(cmd==='cancel_zip_repair'){reject?.(Error('LZOP recovery cancelled'))}return Promise.resolve()})
 const w=create();await flushPromises();await w.get('[data-testid="lzop-recovery-confirm"]').trigger('click');await flushPromises();const args=mocks.invoke.mock.calls.find(c=>c[0]==='recover_lzop_members')![1];mocks.listener?.({payload:{operationId:args.operationId,phase:'recovering',bytes:5,total:10}});await flushPromises();expect(w.get('[data-testid="lzop-recovery-progress"]').text()).toContain('50%')
 await w.get('[data-testid="lzop-recovery-cancel"]').trigger('click');await flushPromises();expect(w.find('[data-testid="lzop-recovery-result"]').exists()).toBe(false);expect(mocks.invoke).toHaveBeenCalledWith('save_task_history',{record:expect.objectContaining({status:'cancelled',executionConfig:expect.objectContaining({recoveredFiles:0})})});w.unmount()
})
it.each([false,true])('lzop174 no recoverable members or complete source never enables partial copy: complete=%s',async complete=>{const w=create({...report(),complete,entries:[]});await flushPromises();expect(w.get('[data-testid="lzop-recovery-confirm"]').attributes('disabled')).toBeDefined();expect(w.find('[data-testid="lzop-recovery-members"]').exists()).toBe(false);w.unmount()})
it('lzop174 history config remains a separate operation and rejects invalid counts',()=>{const config={schemaVersion:1 as const,kind:'archive-recovery' as const,outputDirectory:'D:/result',recoveredFiles:2,stopReason:'后续未确认',revision:'a'.repeat(64)};expect(copyExecutionConfig(config)).toEqual(config);expect(copyExecutionConfig({...config,recoveredFiles:-1})).toBeNull();expect(restoredExtractionConfig({taskType:'decompression',executionConfig:config}as TaskHistoryRecord)).toBeNull()})
