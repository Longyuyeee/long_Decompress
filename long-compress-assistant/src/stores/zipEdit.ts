import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useTaskStore } from './task'
import { resolveZipEdits } from '@/utils/zipEditPlan'
import type { ZipEditCatalog, ZipEditDraft, ZipEditInput, ZipEditChoice } from '@/types/zipEdit'
import type { PublishedArchiveOutput } from '@/types/archiveOutput'
import { pathKey } from '@/utils/resultLocation'
export interface ZipEditSession {
  sourcePath: string; catalog: ZipEditCatalog; drafts: ZipEditDraft[]; method: 'deflate' | 'store'; level: number
  outputPath: string; busy: boolean; editing: boolean; error: string; taskId?: string; saves: number
}
const suggestion = (source: string, count: number) => source.replace(/\.zip$/i, '') + `_已编辑${count > 1 ? count : ''}.zip`
export const useZipEditStore = defineStore('zipEdit', () => {
  const tasks = useTaskStore(), sessions = ref<Record<string, ZipEditSession>>({})
  const opening = ref(false), preparing = ref(false), openingError = ref<{ source: string; message: string } | null>(null)
  const sessionFor = (source: string) => sessions.value[pathKey(source)]
  const taskFor = (session?: ZipEditSession) => tasks.tasks.find(task => task.id === session?.taskId)
  const plan = (session: ZipEditSession) => resolveZipEdits(session.catalog, session.drafts)
  async function begin(source: string) {
    const current = sessionFor(source)
    if (current?.editing || current?.busy) return current
    if (opening.value) return
    opening.value = true; openingError.value = null
    try {
      const catalog = await invoke<ZipEditCatalog>('inspect_zip_edit', { sourcePath: source })
      const session: ZipEditSession = { sourcePath: source, catalog, drafts: [], method: 'deflate', level: 6,
        outputPath: suggestion(source, (current?.saves || 0) + 1), busy: false, editing: true, error: '', saves: current?.saves || 0 }
      sessions.value[pathKey(source)] = session
      return sessionFor(source)
    } catch (error) { openingError.value = { source, message: String(error) } }
    finally { opening.value = false }
  }
  async function add(source: string, paths: string[], directory: string, replacement?: string) {
    const session = await begin(source); if (!session || session.busy || preparing.value || !paths.length) return
    preparing.value = true; session.error = ''
    try {
      const inputs = await invoke<ZipEditInput[]>('prepare_zip_edit_inputs', { sources: paths, directory })
      if (sessionFor(source) !== session || !session.editing) return
      if (replacement && (inputs.length !== 1 || inputs[0].isDir)) throw new Error('请选择一个普通文件替换此条目')
      const batch = crypto.randomUUID()
      for (const input of inputs) {
        const targetPath = replacement || input.targetPath
        if (!session.drafts.some(draft => pathKey(draft.sourcePath) === pathKey(input.sourcePath) && draft.targetPath === targetPath && draft.sourceRevision === input.sourceRevision)) {
          session.drafts.push({ ...input, targetPath, id: crypto.randomUUID(), batch, choice: replacement ? 'replace' : 'choose' })
        }
      }
    } catch (error) { session.error = String(error) }
    finally { preparing.value = false }
  }
  function remove(source: string, id: string) {
    const session = sessionFor(source); if (!session || session.busy) return
    const draft = session.drafts.find(row => row.id === id); if (!draft) return
    session.drafts = session.drafts.filter(row => row.id !== id && !(draft.isDir && row.batch === draft.batch && row.targetPath.startsWith(draft.targetPath)))
  }
  function choose(source: string, id: string, choice: ZipEditChoice) {
    const session = sessionFor(source); if (!session || session.busy) return
    const draft = session.drafts.find(row => row.id === id); if (draft) draft.choice = choice
  }
  function discard(source: string) { const session = sessionFor(source); if (!session?.busy && !preparing.value) delete sessions.value[pathKey(source)] }
  async function refresh(source: string) {
    const session = sessionFor(source); if (!session || session.busy || opening.value) return
    opening.value = true; session.error = ''
    try { session.catalog = await invoke<ZipEditCatalog>('inspect_zip_edit', { sourcePath: source }) }
    catch (error) { session.error = String(error) }
    finally { opening.value = false }
  }
  async function save(source: string) {
    const session = sessionFor(source); if (!session || session.busy || preparing.value || opening.value) return
    const resolved = plan(session); if (resolved.unresolved || !resolved.changes.length) { session.error = '请先处理同名项，并确认至少一项有效修改'; return }
    if (!session.outputPath.trim()) { session.error = '请选择新 ZIP 保存位置'; return }
    session.busy = true; session.error = ''
    const previous = taskFor(session)
    const id = 'zip-edit-' + crypto.randomUUID()
    try {
      if (previous && ['failed','cancelled'].includes(previous.status) && !await tasks.waitForHistoryPersistence(previous.id)) throw new Error('原任务历史尚未保存成功，请先在任务面板重试保存；修改仍保留')
      await tasks.initListeners()
      tasks.addTask({ id, name: '编辑另存 · ' + (source.split(/[\\/]/).pop() || source), type: 'compression', workloadKind: 'archive', sourceFiles: [source], outputPath: session.outputPath.trim(), format: 'zip',
        ...(previous && ['failed','cancelled'].includes(previous.status) ? { retryOfTaskId: previous.id } : {}),
        executionConfig: { schemaVersion: 1, kind: 'archive-edit', outputPath: session.outputPath.trim() } })
      session.taskId = id; tasks.updateTaskStatus(id, 'compressing')
      const output = await invoke<PublishedArchiveOutput>('save_zip_edit', { taskId: id, sourcePath: source, revision: session.catalog.revision,
        changes: resolved.changes, outputPath: session.outputPath.trim(), method: session.method, level: session.level })
      const task = taskFor(session)!
      task.archiveOutput = output; task.outputPath = output.primaryPath; task.outputBytes = output.totalBytes
      session.drafts = []; session.editing = false; session.saves++; tasks.updateTaskStatus(id, 'completed')
    } catch (error) {
      const task = tasks.tasks.find(task => task.id === id)
      if (task) {
        if (task.status === 'cancelling' || String(error).includes('ZIP_EDIT_CANCELLED')) tasks.updateTaskStatus(id, 'cancelled')
        else tasks.failTask(id, String(error))
      } else session.error = String(error)
    } finally { session.busy = false }
  }
  async function cancel(source: string) { const session = sessionFor(source); if (session?.taskId && session.busy) await tasks.cancelTask(session.taskId) }
  return { sessions, opening, preparing, openingError, sessionFor, taskFor, plan, begin, add, remove, choose, discard, refresh, save, cancel }
})
