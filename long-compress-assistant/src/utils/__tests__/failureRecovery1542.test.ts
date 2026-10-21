import { beforeEach, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { useArchiveConversionStore } from '@/stores/archiveConversion'
import { useTaskStore } from '@/stores/task'
import { useCompressionStore } from '@/stores/compression'
import { createTaskHistoryRecord } from '@/types/taskHistory'
import { copyExecutionConfig, type ArchiveConversionConfigV1 } from '@/types/executionConfig'
import { describeTaskFailure, getTaskRecoveryGuidance } from '../taskFailure'
import { prepareCompressionRetryDraft } from '../compressionRetry'
import ConversionSettingsFields from '@/components/compression/ConversionSettingsFields.vue'
import ToastContainer from '@/components/ui/ToastContainer.vue'
import { useUIStore } from '@/stores/ui'
import TaskFailureHint from '@/components/tasks/TaskFailureHint.vue'

const mocks=vi.hoisted(()=>({invoke:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/dialog',()=>({ask:vi.fn(async()=>true),open:vi.fn()}))
beforeEach(()=>{setActivePinia(createPinia());mocks.invoke.mockReset();mocks.invoke.mockImplementation(async(cmd,args)=>{
  if(cmd==='detect_split_archive')return{is_split:false,archive_format:'zip'}
  if(cmd==='get_file_info')return{name:'source.txt',size:128,is_dir:args.path==='D:/output'}
})})
const draft=()=>{
  const store=useArchiveConversionStore();const item=store.addSources(['C:/source.zip'])[0]
  store.updateItem(item.id,{format:'zip',outputPath:'D:/output/new.zip',sourcePassword:'source-secret',password:'target-secret',splitMiB:1,zipSplitMode:'standard',sourceZipPasswordEncoding:'cp936'})
  return{store,item}
}

it.each(['output-conflict','rollback-incomplete','verification-failed'])('1542 keeps explicit %s evidence ahead of an unknown live snapshot',category=>{
  const tag=category==='verification-failed'?'Verifying':'Publishing'
  const record={status:'failed' as const,errorMessage:`[archive-output:${tag}:${category}] 实际原因`,failure:{schemaVersion:1 as const,category:'unknown',stage:'Extracting',evidence:'observed-stage' as const}}
  expect(describeTaskFailure(record)).toMatchObject({category,stage:tag,evidence:'recorded-marker',stageEvidence:'recorded-marker'})
})
it('1542 does not turn observed publishing plus a permission marker into proven output permission',()=>{
  const record={status:'failed' as const,errorMessage:'[archive-io:permission] 无法访问路径',failure:{schemaVersion:1 as const,category:'unknown',stage:'Publishing',evidence:'observed-stage' as const}}
  expect(describeTaskFailure(record)).toMatchObject({category:'permission',stageEvidence:'observed-stage'})
  expect(getTaskRecoveryGuidance(record)?.steps[0]).toContain('无法仅凭阶段确定')
})
it('1542 preserves the failed conversion row and both password requirements, then starts a linked task only after confirmation',async()=>{
  const {store,item}=draft();let attempt=0
  mocks.invoke.mockImplementation(async(cmd)=>{if(cmd==='detect_split_archive')return{is_split:false,archive_format:'zip'};if(cmd==='convert_archive'){if(!attempt++)throw '[archive-output:Publishing:output-conflict] 目标已存在';return{primaryPath:'D:/output/new_重试.zip',files:[],totalBytes:128,split:false}}})
  await store.start();const oldTask=useTaskStore().tasks[0];const before=JSON.stringify(createTaskHistoryRecord(oldTask));const row=JSON.stringify(item)
  const id=await store.prepareRetry(item.id);const retry=store.items.find(i=>i.id===id)!
  expect(store.items).toHaveLength(2);expect(JSON.stringify(item)).toBe(row);expect(retry.retryOfTaskId).toBe(oldTask.id)
  expect(retry.settings).toMatchObject({format:'zip',splitMiB:1,zipSplitMode:'standard',sourceZipPasswordEncoding:'cp936',sourcePassword:'',password:'',outputPath:'D:/output/new_重试.zip'})
  expect(store.passwordRequirements(retry)).toEqual({source:true,target:true})
  await store.start();expect(attempt).toBe(1)
  store.updateItem(retry.id,{sourcePassword:'correct-source'});await store.start();expect(attempt).toBe(1)
  store.updateItem(retry.id,{password:'correct-target'});await store.start();expect(attempt).toBe(2)
  expect(useTaskStore().tasks[1].retryOfTaskId).toBe(oldTask.id);expect(JSON.stringify(createTaskHistoryRecord(oldTask))).toBe(before)
  expect(JSON.stringify(createTaskHistoryRecord(useTaskStore().tasks[1]))).not.toMatch(/correct-source|correct-target/)
})
it('1542 reopens one existing pending retry, and explicit no-encryption is required before dropping an output password',async()=>{
  const{store,item}=draft();mocks.invoke.mockImplementation(async cmd=>{if(cmd==='convert_archive')throw '密码错误'})
  await store.start();const a=await store.prepareRetry(item.id),b=await store.prepareRetry(item.id)
  expect(a).toBe(b);expect(store.pending).toHaveLength(1)
  store.updateItem(a!,{sourcePassword:'fixed',password:'',encryptFilenames:false})
  expect(store.passwordRequirements(store.pending[0])).toEqual({source:false,target:false})
})
it('1542 refuses to create conversion retries while original history is still unsaved',async()=>{
  const {store,item}=draft();mocks.invoke.mockImplementation(async cmd=>{if(cmd==='convert_archive')throw '读取失败';if(cmd==='save_task_history')throw Error('disk offline')})
  await store.start();expect(await store.prepareRetry(item.id)).toBeUndefined();expect(store.items).toHaveLength(1);expect(store.error).toContain('历史尚未保存')
})
it('1542 carries source-password requirements through the history whitelist and restores the original destination folder',async()=>{
  const {store,item}=draft();mocks.invoke.mockImplementation(async cmd=>cmd==='convert_archive'?{primaryPath:'D:/output/new.zip',files:[],totalBytes:128,split:false}:undefined)
  await store.start();const record=createTaskHistoryRecord(useTaskStore().tasks[0]);expect(record.executionConfig).toMatchObject({sourcePasswordRequired:true,passwordRequired:true})
  store.restore(record);expect(store.passwordRequirements(store.pending[0])).toEqual({source:true,target:true});expect(store.outputFor(store.pending[0])).toMatch(/^D:\/output\//)
  expect(copyExecutionConfig({...record.executionConfig,sourcePasswordRequired:'yes'} as unknown as ArchiveConversionConfigV1)).toBeNull()
})
it('1542 prepares compression with original settings and a password reminder, preserving the failed record and clearing the reminder when edited normally',async()=>{
  const tasks=useTaskStore(), compression=useCompressionStore()
  tasks.addTask({id:'failed',name:'文件.zip',type:'compression',sourceFiles:['C:/source.txt'],outputPath:'D:/output/new.zip',executionConfig:{schemaVersion:1,kind:'archive-compression',outputDirectory:'D:/output',passwordRequired:true,settings:{format:'zip',level:8,filename:'new',splitArchive:true,splitSize:'1',zipSplitMode:'standard',keepStructure:true,verifyAfter:true,createSolidArchive:false}}})
  tasks.failTask('failed','[archive-output:Publishing:output-conflict] 输出已存在');await tasks.waitForHistoryPersistence('failed')
  const task=tasks.tasks[0],before=JSON.stringify(createTaskHistoryRecord(task))
  const id=await prepareCompressionRetryDraft(task);expect(await prepareCompressionRetryDraft(task)).toBe(id)
  expect(compression.groups).toHaveLength(1);const group=compression.groups[0]
  expect(group.settings).toMatchObject({level:8,filename:'new_重试',password:'',deleteAfter:false,splitSize:'1',zipSplitMode:'standard'})
  expect(group.draftPasswordRequired).toBe(true);expect(group.taskId).toBeUndefined();expect(JSON.stringify(createTaskHistoryRecord(task))).toBe(before)
  compression.updateGroupSettings(id,{...group.settings!,password:'correct'})
  expect(group.draftPasswordRequired).toBe(false)
})
it('1542 shows actionable reasons in place without claiming unknown damage or requiring a new dialog',async()=>{
  const tasks=useTaskStore();tasks.addTask({id:'notice',name:'a.zip',type:'compression',sourceFiles:[],outputPath:''})
  tasks.failTask('notice','[archive-inspection:unsupported] 当前不支持此方法')
  const wrapper=mount(TaskFailureHint,{props:{task:tasks.tasks[0]}});await flushPromises()
  expect(wrapper.text()).toContain('暂不支持此特性');expect(wrapper.text()).toContain('反复尝试密码不能解决');expect(wrapper.text()).not.toContain('[archive-inspection:')
  tasks.tasks[0].error='未知读取异常';await flushPromises();expect(wrapper.text()).toContain('原因尚未确定');wrapper.unmount()
})

it('1542 recognizes output conflicts before work starts and keeps the confirmed phase visible',()=>{
 const record={status:'failed' as const,errorMessage:'[archive-output:Pre-checking:output-conflict] 原文件未覆盖',failure:{schemaVersion:1 as const,category:'unknown',stage:'Compressing',evidence:'observed-stage' as const}}
 expect(describeTaskFailure(record)).toMatchObject({category:'output-conflict',stage:'Pre-checking',stageLabel:'预检'})
 expect(getTaskRecoveryGuidance(record)?.steps.join(' ')).toContain('新的名称')
})

it('1542 final polish clears a resolved validation message and labels each required password unambiguously',async()=>{
 const {store,item}=draft();mocks.invoke.mockImplementation(async cmd=>{if(cmd==='convert_archive')throw '[archive-output:Pre-checking:output-conflict] old.zip'})
 await store.start();const id=await store.prepareRetry(item.id);const retry=store.pending.find(row=>row.id===id)!
 await store.start();expect(store.error).toContain('原包密码')
 const wrapper=mount(ConversionSettingsFields,{props:{settings:store.effectiveSettings(retry),sourcePasswordRequired:true,targetPasswordRequired:true}})
 expect(wrapper.get('[data-testid="conversion-source-password"]').attributes('placeholder')).toBe('请重新填写原包密码')
 expect(wrapper.get('[data-testid="conversion-password"]').attributes('placeholder')).toContain('请重新填写新包密码')
 store.updateItem(retry.id,{sourcePassword:'fixed'});expect(store.error).toBe('');wrapper.unmount()
})
it('1542 final polish keeps a long conflict notification short while preserving the original record text',()=>{
 const ui=useUIStore(),message='[archive-output:Pre-checking:output-conflict] D:/'+ 'long-path/'.repeat(40)+'old.zip'
 ui.showToast('error',message);const wrapper=mount(ToastContainer)
 expect(wrapper.text()).toContain('请修改名称或保存位置');expect(wrapper.text()).not.toContain('[archive-output:');expect(wrapper.text()).not.toContain('long-path')
 expect(ui.toasts[0].message).toBe(message);wrapper.unmount()
})
