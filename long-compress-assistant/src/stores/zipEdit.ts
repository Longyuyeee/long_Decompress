import type { ComparisonSource, ComparisonSourceRef } from '@/types/sourceComparison'
import { defineStore } from 'pinia'
import { ref, computed, watch, markRaw, type WatchStopHandle } from 'vue'
import { savedZipEdit, parseZipEdits, type SavedZipEdit } from '@/utils/zipEditDrafts'
import type { ZipPasswordEncoding } from '@/types/zipPassword'
import type { ArchiveReadOptions } from './app'
import { invoke } from '@tauri-apps/api/tauri'
import { useTaskStore } from './task'
import { resolveZipWorkspace, previewZipStructure } from '@/utils/zipWorkspace'
import type { ZipEditCatalog, ZipEditDraft, ZipEditInput, ZipEditChoice, ZipStructureOperation, ZipEditCredentials } from '@/types/zipEdit'
import type { PublishedArchiveOutput } from '@/types/archiveOutput'
import { pathKey } from '@/utils/resultLocation'
export interface ZipEditSession extends ZipEditCredentials {
  comparisonPasswords?: Record<string,{password:string;encoding?:ZipEditCredentials['sourceEncoding']}>;
  sourcePath: string; catalog: ZipEditCatalog; drafts: ZipEditDraft[]; method: 'deflate' | 'store'; level: number
  outputPath: string; busy: boolean; editing: boolean; error: string; taskId?: string; saves: number; operations: ZipStructureOperation[]
  sourceIssue: string; inputIssues: Record<string,string>; retryOfTaskId?: string
}
const suggestion = (source: string, count: number) => source.replace(/\.zip$/i, '') + `_已编辑${count > 1 ? count : ''}.zip`
export const useZipEditStore = defineStore('zipEdit', () => {
  const tasks = useTaskStore(), sessions = ref<Record<string, ZipEditSession>>({})
  const opening = ref(false), preparing = ref(false), openingError = ref<{ source: string; message: string } | null>(null)
  const sessionFor = (source: string) => sessions.value[pathKey(source)]
  const taskFor = (session?: ZipEditSession) => tasks.tasks.find(task => task.id === session?.taskId)
  const plan = (session: ZipEditSession) => resolveZipWorkspace(session.catalog, session.drafts, session.operations)
  const recovery=ref<SavedZipEdit[]>([]), recoveryVisible=ref(false), recoveryBusy=ref(false), saveError=ref(''), recoveryFeedback=ref('')
  const hasDraft=(row:ZipEditSession)=>row.editing&&!!(row.drafts.length||row.operations.length||row.outputMode!=='keep')
  const draftCount=computed(()=>new Set([...recovery.value.map(row=>pathKey(row.sourcePath)),...Object.values(sessions.value).filter(hasDraft).map(row=>pathKey(row.sourcePath))]).size)
  const allDrafts=computed(()=>[...recovery.value.filter(row=>!sessionFor(row.sourcePath)).map(row=>({sourcePath:row.sourcePath,savedAt:row.savedAt,recovered:true,changes:row.drafts.length+row.operations.length})),...Object.values(sessions.value).filter(hasDraft).map(row=>({sourcePath:row.sourcePath,savedAt:'当前会话',recovered:false,changes:row.drafts.length+row.operations.length}))])
  let initialized=false, initialization:Promise<void>|undefined, stop:WatchStopHandle|undefined,timer:ReturnType<typeof setTimeout>|undefined,writeQueue=Promise.resolve(true)
  function flush():Promise<boolean>{
    if(!initialized)return Promise.resolve(true)
    if(timer)clearTimeout(timer);timer=undefined
    const current=new Set(Object.keys(sessions.value)),saved=[...recovery.value.filter(row=>!current.has(pathKey(row.sourcePath))),...Object.values(sessions.value).filter(hasDraft).map(savedZipEdit)]
    const data=saved.length?JSON.stringify({version:1,savedAt:new Date().toISOString(),sessions:saved}):null
    writeQueue=writeQueue.then(async()=>{try{await invoke('save_zip_edit_drafts',{data});saveError.value='';return true}catch{saveError.value='ZIP 编辑草稿暂时无法保存，请保持软件打开并重试保存';return false}})
    return writeQueue
  }
  function initialize(){
    if(initialization)return initialization
    initialization=(async()=>{
      try{recovery.value=parseZipEdits(await invoke<string|null>('load_zip_edit_drafts'))}catch(error){saveError.value=String(error);initialization=undefined;return}
      initialized=true;stop=watch(sessions,()=>{if(timer)clearTimeout(timer);timer=setTimeout(()=>{void flush()},300)},{deep:true})
      window.addEventListener('pagehide',onPageHide)
    })();return initialization
  }
  function onPageHide(){void flush()}
  function dispose(){if(timer)clearTimeout(timer);void flush();stop?.();window.removeEventListener('pagehide',onPageHide);initialized=false;initialization=undefined}
  async function checkSources(source:string){
    const session=sessionFor(source);if(!session||session.busy)return
    try{const catalog=await invoke<ZipEditCatalog>('inspect_zip_edit',{sourcePath:source});session.sourceIssue=catalog.revision===session.catalog.revision?'':'原包已变化，请读取当前原包并确认后继续'}catch{session.sourceIssue='原包缺失或无法访问，请重新定位'}
    try{const issues=await invoke<Array<{index:number;issue:string}>>('check_zip_edit_inputs',{inputs:session.drafts.map(row=>({sourcePath:row.sourcePath,targetPath:row.targetPath,isDir:row.isDir,size:row.size,sourceRevision:row.sourceRevision,comparison:row.comparison}))});session.inputIssues=Object.fromEntries((issues||[]).filter(row=>session.drafts[row.index]).map(row=>[session.drafts[row.index].id,row.issue]))}catch(error){session.error='来源检查未完成：'+String(error)}
  }
  async function restoreDraft(source:string):Promise<{path?:string;published?:string}> {
    await initialize();const current=sessionFor(source)
    if(current?.editing){recoveryFeedback.value='已返回当前会话编辑，未覆盖现有修改';return{path:source}}
    const saved=recovery.value.find(row=>pathKey(row.sourcePath)===pathKey(source));if(!saved||recoveryBusy.value)return{}
    recoveryBusy.value=true;recoveryFeedback.value=''
    try{
      const receipt=saved.taskId?await invoke<{status:string;output?:PublishedArchiveOutput}|null>('zip_edit_task_receipt',{taskId:saved.taskId,sourcePath:saved.sourcePath}):null
      if(receipt?.status==='completed'){
        recovery.value=recovery.value.filter(row=>row!==saved);await flush();recoveryFeedback.value='此编辑已保存，已返回原结果，没有重新执行';return{published:receipt.output?.primaryPath||saved.outputPath}
      }
      const session:ZipEditSession={...saved,catalog:markRaw(saved.catalog),sourcePassword:'',outputPassword:'',busy:false,editing:true,error:'',sourceIssue:'',inputIssues:{},taskId:undefined,retryOfTaskId:receipt&&['failed','cancelled'].includes(receipt.status)?saved.taskId:undefined}
      sessions.value[pathKey(source)]=session;recovery.value=recovery.value.filter(row=>row!==saved)
      await checkSources(source);await flush();recoveryFeedback.value='已恢复编辑清单，请核对来源与密码后另存';return{path:source}
    }catch(error){recoveryFeedback.value='恢复未完成，上次草稿仍保留：'+String(error);return{}}finally{recoveryBusy.value=false}
  }
  async function discardDraft(source:string){const session=sessionFor(source);if(session?.busy)return;delete sessions.value[pathKey(source)];recovery.value=recovery.value.filter(row=>pathKey(row.sourcePath)!==pathKey(source));await flush()}
  async function relocateSource(source:string,path:string){
    let session=sessionFor(source);if(!session){await restoreDraft(source);session=sessionFor(source)}if(!session||session.busy)return
    await invoke<ZipEditCatalog>('inspect_zip_edit',{sourcePath:path})
    if(pathKey(path)!==pathKey(source)&&sessionFor(path))throw new Error('该位置已有编辑，请先处理当前草稿')
    delete sessions.value[pathKey(source)];session.sourcePath=path;sessions.value[pathKey(path)]=session;await checkSources(path);await flush();return path
  }
  async function relinkInput(source:string,id:string,path?:string){
    const session=sessionFor(source),draft=session?.drafts.find(row=>row.id===id);if(!session||!draft||session.busy)return
    const oldRoot=draft.sourcePath.replace(/\\/g,'/').replace(/\/$/,''),target=path||draft.sourcePath
    const fresh=await invoke<ZipEditInput[]>('prepare_zip_edit_inputs',{sources:[target],directory:''}),root=fresh[0]
    if(!root||root.isDir!==draft.isDir)throw new Error('请选择相同类型的文件或目录')
    if(!draft.isDir)Object.assign(draft,{sourcePath:root.sourcePath,size:root.size,sourceRevision:root.sourceRevision})
    else{
      const newRoot=root.sourcePath.replace(/\\/g,'/').replace(/\/$/,'');const byRelative=new Map(fresh.map(row=>[pathKey(row.sourcePath.replace(/\\/g,'/').slice(newRoot.length)),row]))
      for(const row of session.drafts.filter(row=>row.id===id||(row.batch===draft.batch&&pathKey(row.sourcePath).startsWith(pathKey(oldRoot)+'/')))){
        const suffix=row.sourcePath.replace(/\\/g,'/').slice(oldRoot.length),replacement=byRelative.get(pathKey(suffix))
        if(replacement&&replacement.isDir===row.isDir)Object.assign(row,{sourcePath:replacement.sourcePath,size:replacement.size,sourceRevision:replacement.sourceRevision})
        else row.sourcePath=newRoot+suffix
      }
    }
    session.error='';await checkSources(source);await flush()
  }
  const credentialsFor=(session:ZipEditSession):ZipEditCredentials=>({sourcePassword:session.sourcePassword,sourceEncoding:session.sourceEncoding,outputMode:session.outputMode,outputPassword:session.outputPassword,outputEncoding:session.outputEncoding})
  function resultReadOptions(session:ZipEditSession):ArchiveReadOptions{
    const password=session.outputMode==='keep'?(session.catalog.encrypted?session.sourcePassword:''):session.outputMode==='none'?'':session.outputPassword
    return password?{password,zipPasswordEncoding:session.outputEncoding}:{}
  }

  async function begin(source: string, credentials?: {password?:string;encoding?:ZipPasswordEncoding}) {
    if(initialization)await initialization
    let current = sessionFor(source)
    if(!current&&recovery.value.some(row=>pathKey(row.sourcePath)===pathKey(source))){await restoreDraft(source);current=sessionFor(source)}
    if (current?.editing || current?.busy) {if(credentials?.password&&!current.sourcePassword){current.sourcePassword=credentials.password;current.sourceEncoding=credentials.encoding}return current}
    if (opening.value) return
    opening.value = true; openingError.value = null
    try {
      const catalog = await invoke<ZipEditCatalog>('inspect_zip_edit', { sourcePath: source })
      const session: ZipEditSession = { sourcePath: source, catalog:markRaw(catalog), sourcePassword:credentials?.password||'', sourceEncoding:credentials?.encoding, outputMode:'keep',outputPassword:'',outputEncoding:'utf8',sourceIssue:'',inputIssues:{},drafts: [], method: 'deflate', level: 6,
        operations: [], outputPath: suggestion(source, (current?.saves || 0) + 1), busy: false, editing: true, error: '', saves: current?.saves || 0 }
      sessions.value[pathKey(source)] = session
      return sessionFor(source)
    } catch (error) { openingError.value = { source, message: String(error) } }
    finally { opening.value = false }
  }
  async function beginComparison(parts:{source:ComparisonSourceRef;paths?:string[];encoding?:ZipEditCredentials['sourceEncoding']}[],passwords:NonNullable<ZipEditSession['comparisonPasswords']>) {
    await initialize();preparing.value=true
    try {
      const prepared=await invoke<{sourcePath:string;inputs:ZipEditInput[]}>('comparison_package_prepare',{parts,jobId:crypto.randomUUID()})
      const session=await begin(prepared.sourcePath);if(!session)throw Error(openingError.value?.message||'无法准备组包清单')
      session.comparisonPasswords=passwords
      session.drafts=prepared.inputs.map((input,index)=>({...input,id:crypto.randomUUID(),batch:input.comparison!.source.path,order:index+1,choice:'choose'}))
      session.outputMode=prepared.inputs.some(input=>input.comparison?.source.encrypted)?'aes256':'none'
      const base=parts[0]!.source.path.replace(/\\/g,'/'),slash=base.lastIndexOf('/')
      session.outputPath=base.slice(0,slash+1)+'所选资料_新包.zip'
      await flush();return session.sourcePath
    }finally{preparing.value=false}
  }
  async function acceptComparisonSource(source:string,origin:string,replacement?:string){
    const session=sessionFor(source);if(!session||session.busy)return
    const current=await invoke<ComparisonSource>('comparison_read',{path:replacement||origin,jobId:crypto.randomUUID()})
    const references=session.drafts.filter(row=>row.comparison?.source.path===origin)
    const updates=references.map(row=>{const entry=current.entries.find(entry=>entry.path===row.comparison!.path&&(entry.selectable||entry.directory)&&entry.directory===row.isDir);if(!entry)throw Error('原选路径已缺失或类型变化，请撤销该项后重新选择：'+row.comparison!.path);return{row,entry}})
    for(const {row,entry} of updates){row.comparison={source:{...current.source},path:entry.path,encoding:row.comparison!.encoding};row.sourcePath=current.source.path;row.sourceRevision=current.source.revision;row.size=entry.size}
    if(origin!==current.source.path&&session.comparisonPasswords?.[origin]){session.comparisonPasswords[current.source.path]=session.comparisonPasswords[origin]!;delete session.comparisonPasswords[origin]}
    await checkSources(source);await flush()
  }
  async function add(source: string, paths: string[], directory: string, replacement?: string, logicalDirectory = false) {
    const session = await begin(source); if (!session || session.busy || preparing.value || !paths.length) return
    preparing.value = true; session.error = ''
    try {
      const nodes=plan(session).nodes
      if (replacement) {
        const node=nodes.find(node=>node.originalPath===replacement||node.originalAlias===replacement)||nodes.find(node=>node.path===replacement)
        if (!node||node.isDir) throw new Error('此原条目已从新包移除，请先撤销对应删除，或在整理窗口选择当前内容')
        replacement=node.path
      }
      if (directory && !logicalDirectory) {
        const original=directory.replace(/\/$/,'')+'/'
        if (session.catalog.entries.some(entry=>entry.path===original&&entry.isDir)) {
          const current=nodes.find(node=>node.originalPath===original&&node.isDir)
          if (!current) throw new Error('此原目录已从新包移除，请先撤销删除，或在整理窗口选择新位置')
          directory=current.path
        }
      }
      const inputs = await invoke<ZipEditInput[]>('prepare_zip_edit_inputs', { sources: paths, directory })
      if (sessionFor(source) !== session || !session.editing) return
      if (replacement && (inputs.length !== 1 || inputs[0].isDir)) throw new Error('请选择一个普通文件替换此条目')
      if (session.drafts.length + inputs.length > 10_000) throw new Error('本次编辑最多添加 10000 项，请分批另存整理')
      const batch = crypto.randomUUID(); let order = nextOrder(session)
      for (const input of inputs) {
        const targetPath = replacement || input.targetPath
        if (!session.drafts.some(draft => pathKey(draft.sourcePath) === pathKey(input.sourcePath) && draft.targetPath === targetPath && draft.sourceRevision === input.sourceRevision)) {
          session.drafts.push({ ...input, targetPath, id: crypto.randomUUID(), batch, order: order++, choice: replacement ? 'replace' : 'choose' })
        }
      }
    } catch (error) { session.error = String(error) }
    finally { preparing.value = false;void flush() }
  }
  const nextOrder = (session: ZipEditSession) => Math.max(-1, ...session.drafts.map((draft,index) => draft.order ?? index), ...session.operations.map(operation => operation.order)) + 1
  function organize(source: string, kind: ZipStructureOperation['kind'], targets: ZipStructureOperation['targets']) {
    const session = sessionFor(source); if (!session || session.busy || preparing.value) return false
    if(session.operations.length>=10000){session.error='本次整理已达 10000 个操作，请先另存后继续';return false}
    const operation: ZipStructureOperation = { id: crypto.randomUUID(), order: nextOrder(session), kind, targets }
    const preview = previewZipStructure(plan(session).nodes, operation)
    if (preview.issue) { session.error = preview.issue; return false }
    session.error = ''; session.operations.push(operation);void flush(); return true
  }
  function undoOperation(source: string, id: string) { const session = sessionFor(source); if (session && !session.busy) session.operations = session.operations.filter(operation => operation.id !== id) }
  function remove(source: string, id: string) {
    const session = sessionFor(source); if (!session || session.busy) return
    const draft = session.drafts.find(row => row.id === id); if (!draft) return
    session.drafts = session.drafts.filter(row => row.id !== id && !(draft.isDir && row.batch === draft.batch && row.targetPath.startsWith(draft.targetPath)))
  }
  function choose(source: string, id: string, choice: ZipEditChoice) {
    const session = sessionFor(source); if (!session || session.busy) return
    const draft = session.drafts.find(row => row.id === id); if (draft) draft.choice = choice
  }
  function discard(source: string) { const session = sessionFor(source); if (!session?.busy && !preparing.value) delete sessions.value[pathKey(source)];void flush() }
  async function refresh(source: string) {
    const session = sessionFor(source); if (!session || session.busy || opening.value) return
    opening.value = true; session.error = ''
    try { session.catalog = markRaw(await invoke<ZipEditCatalog>('inspect_zip_edit', { sourcePath: source })); session.sourceIssue='';await checkSources(source);await flush() }
    catch (error) { session.error = String(error) }
    finally { opening.value = false }
  }
  async function save(source: string) {
    const session = sessionFor(source); if (!session || session.busy || preparing.value || opening.value) return
    const resolved = plan(session); if (resolved.unresolved || !(resolved.hasChanges||session.outputMode!=='keep')) { session.error = '请先处理同名项，并确认至少一项有效修改'; return }
    if (!session.outputPath.trim()) { session.error = '请选择新 ZIP 保存位置'; return }
    if(session.sourceIssue||Object.keys(session.inputIssues).length){session.error='请先处理下方来源变化，修改清单仍保留';return}
    if(session.catalog.encrypted&&!session.sourcePassword){session.error='请填写原包密码，或从保险箱选择';return}
    if(['aes256','zipcrypto'].includes(session.outputMode)&&!session.outputPassword){session.error='请填写新包密码，或明确选择新包不加密';return}

    session.busy = true; session.error = ''
    const previous = taskFor(session)
    const id = 'zip-edit-' + crypto.randomUUID()
    try {
      if (previous && ['failed','cancelled'].includes(previous.status) && !await tasks.waitForHistoryPersistence(previous.id)) throw new Error('原任务历史尚未保存成功，请先在任务面板重试保存；修改仍保留')
      await tasks.initListeners()
      tasks.addTask({ id, name: session.drafts.some(row=>row.comparison) ? '对比组包 · '+session.outputPath.split(/[\\/]/).pop() : '编辑另存 · '+(source.split(/[\\/]/).pop()||source), type: 'compression', workloadKind: 'archive', sourceFiles: [source], outputPath: session.outputPath.trim(), format: 'zip',
        ...(previous && ['failed','cancelled'].includes(previous.status) ? { retryOfTaskId: previous.id } : session.retryOfTaskId ? {retryOfTaskId:session.retryOfTaskId} : {}),
        executionConfig: { schemaVersion: 1, kind: 'archive-edit', outputPath: session.outputPath.trim() } })
      session.taskId = id; tasks.updateTaskStatus(id, 'compressing');await flush()
      const output = await invoke<PublishedArchiveOutput>('save_zip_edit', { taskId: id, sourcePath: source, revision: session.catalog.revision,
        changes: resolved.changes, originalEdits: resolved.originalEdits, outputPath: session.outputPath.trim(), method: session.method, level: session.level,credentials:credentialsFor(session),comparisonCredentials:Object.entries(session.comparisonPasswords||{}).map(([path,value])=>({path,...value})) })
      const task = taskFor(session)!
      task.archiveOutput = output; task.outputPath = output.primaryPath; task.outputBytes = output.totalBytes
      session.drafts = []; session.operations = []; session.editing = false; session.saves++; tasks.updateTaskStatus(id, 'completed');await flush()
    } catch (error) {
      const task = tasks.tasks.find(task => task.id === id)
      if (task) {
        if (task.status === 'cancelling' || String(error).includes('ZIP_EDIT_CANCELLED')) tasks.updateTaskStatus(id, 'cancelled')
        else tasks.failTask(id, String(error))
      } else session.error = String(error)
    } finally { session.busy = false;if(session.editing){await checkSources(source);await flush()} }
  }
  async function cancel(source: string) { const session = sessionFor(source); if (session?.taskId && session.busy) await tasks.cancelTask(session.taskId) }
  return { recovery, recoveryVisible, recoveryBusy, recoveryFeedback, saveError, draftCount, allDrafts, initialize, flush, dispose, restoreDraft, discardDraft, relocateSource, relinkInput, checkSources, resultReadOptions, sessions, opening, preparing, openingError, sessionFor, taskFor, plan, begin, beginComparison, acceptComparisonSource, add, remove, choose, organize, undoOperation, discard, refresh, save, cancel }
})
