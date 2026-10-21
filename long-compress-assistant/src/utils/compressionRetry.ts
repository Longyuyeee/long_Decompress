import { invoke } from '@tauri-apps/api/tauri'
import { useCompressionStore, type FileObject } from '@/stores/compression'
import { useTaskStore, type Task } from '@/stores/task'
import { copyExecutionConfig } from '@/types/executionConfig'
import { describeTaskFailure } from './taskFailure'

/** Prepare in the existing workspace; never restart or mutate the terminal task. */
export async function prepareCompressionRetryDraft(task: Task): Promise<string> {
  if (task.type !== 'compression' || (task.workloadKind && task.workloadKind !== 'archive') || !['failed','cancelled'].includes(task.status)) throw new Error('请选择已结束的归档压缩任务。')
  const store = useCompressionStore(), tasks = useTaskStore()
  const existing = store.groups.find(group => group.retryOfTaskId === task.id)
  if (existing) { existing.expanded = true; return existing.id }
  const category = describeTaskFailure({status:task.status as 'failed'|'cancelled',errorMessage:task.error})?.category
  if (category === 'rollback-incomplete') throw new Error('请先保留并核对原输出和恢复目录，再从历史中调整配置。')
  const config = copyExecutionConfig(task.executionConfig)
  if (config?.kind !== 'archive-compression') throw new Error('此旧任务没有完整压缩配置，请到历史中重新设置。')
  const saved = task.historySaveError ? await tasks.retryHistoryPersistence(task.id) : await tasks.waitForHistoryPersistence(task.id)
  if (!saved) throw new Error('原任务历史尚未保存成功，请先重试保存；未创建新任务。')
  const files: FileObject[] = []
  for (const source of task.sourceFiles) {
    const info = await invoke<{name:string;size:number;is_dir:boolean}>('get_file_info',{path:source})
    files.push({path:source,name:info.name,size:info.size,isDirectory:info.is_dir,type:info.is_dir?'directory':'file'})
  }
  if (!tasks.tasks.includes(task) || !['failed','cancelled'].includes(task.status)) throw new Error('原任务已变化，请重新打开。')
  const settings = {...config.settings,password:'',deleteAfter:false}
  if (category === 'output-conflict') settings.filename = (settings.filename || task.name.replace(/\.[^.]+$/, '')) + '_重试'
  const concurrent = store.groups.find(group => group.retryOfTaskId === task.id)
  if (concurrent) { concurrent.expanded = true; return concurrent.id }
  const id = store.addHistoryDraft(files,task.name,settings,config.outputDirectory,task.id)
  const group = store.groups.find(group=>group.id===id)!
  group.draftPasswordRequired = config.passwordRequired
  return id
}
