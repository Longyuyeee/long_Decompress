import { computed, ref, watch } from 'vue'
import { defineStore, storeToRefs } from 'pinia'
import { invoke } from '@tauri-apps/api/tauri'
import { useTauriCommands, type ArchiveDiagnosticReport, type ZipRepairResult } from './useTauriCommands'
import { useTaskStore } from '@/stores/task'
import { generateId } from '@/utils'
import { parentDirectory } from '@/utils/resultLocation'
import { archiveSourceStem } from '@/utils/archiveSource'
import { isZipPasswordEncoding, zipPasswordEncodingArguments, type ZipPasswordEncoding } from '@/types/zipPassword'
import { supportsZipPasswordEncoding, zipPasswordEncodingForSource } from '@/utils/zipPasswordSource'

export interface ArchiveRecoveryRow {
  id: string; path: string; selectedMembers: number; selected: boolean; expanded: boolean
  password: string; outputDirectory: string; extractDirectory: string
  state: 'pending' | 'checking' | 'checked' | 'repairing' | 'repaired' | 'error' | 'cancelled'
  report?: ArchiveDiagnosticReport; result?: ZipRepairResult; resultPassword?: string; queuedId?: string; error?: string
  needsRecheck?: boolean; resultPasswordRequired?: boolean
  zipPasswordEncoding?: ZipPasswordEncoding; resultZipPasswordEncoding?: ZipPasswordEncoding; sourceFormat?: string
}
export const ARCHIVE_RECOVERY_DRAFT_KEY = 'long-archive-recovery-workspace-v1'
interface RecoverySnapshot { version: 1; paths: string[]; rootDirectory: string; rows: ArchiveRecoveryRow[] }
const pathKey = (path:string) => path.replace(/\\/g,'/').toLowerCase()
function readSnapshot(): RecoverySnapshot | null {
  const raw = localStorage.getItem(ARCHIVE_RECOVERY_DRAFT_KEY)
  if (!raw) return null
  const saved = JSON.parse(raw) as RecoverySnapshot
  if (saved.version !== 1 || !Array.isArray(saved.paths) || !saved.paths.every(path => typeof path === 'string') || typeof saved.rootDirectory !== 'string'
    || !Array.isArray(saved.rows) || !saved.rows.length || !saved.rows.every(row => row && typeof row.id === 'string' && typeof row.path === 'string'
      && typeof row.outputDirectory === 'string' && typeof row.extractDirectory === 'string'
      && (row.zipPasswordEncoding === undefined || isZipPasswordEncoding(row.zipPasswordEncoding))
      && (row.resultZipPasswordEncoding === undefined || isZipPasswordEncoding(row.resultZipPasswordEncoding))
      && (!row.result || typeof row.result.outputPath === 'string') && (!row.report || Array.isArray(row.report.issues)))) throw new Error('检查修复草稿无法读取，可丢弃后重新选择压缩包')
  return saved
}
export const useArchiveRecoveryStore = defineStore('archiveRecovery', () => {
  const commands = useTauriCommands(), tasks = useTaskStore()
  const rows = ref<ArchiveRecoveryRow[]>([]), busy = ref(false), preparing = ref(false)
  const sourcePaths = ref<string[]>([]), rootDirectory = ref(''), saveError = ref('')
  const recovery = ref<RecoverySnapshot | null>(null)
  try { recovery.value = readSnapshot() } catch (error) { saveError.value = String(error) }
  const recoveryCount = computed(() => recovery.value?.rows.length || 0)
  function persist() {
    // Keep the unopened previous-session offer until the user restores/discards it.
    if (recovery.value) return
    try {
      if (!rows.value.length) localStorage.removeItem(ARCHIVE_RECOVERY_DRAFT_KEY)
      else localStorage.setItem(ARCHIVE_RECOVERY_DRAFT_KEY, JSON.stringify({version:1, paths:sourcePaths.value, rootDirectory:rootDirectory.value,
        rows:rows.value.map(({password: _password, resultPassword: _resultPassword, ...row}) => row)}))
      saveError.value = ''
    } catch { saveError.value = '检查修复进度未能保存，请勿依赖重启恢复' }
  }
  watch([rows, sourcePaths, rootDirectory], persist, {deep:true,flush:'sync'})
  function restore() {
    if (busy.value || preparing.value || !recovery.value) return
    const saved = recovery.value
    const current = new Set(rows.value.map(row => pathKey(row.path)))
    const restored = saved.rows.filter(row => !current.has(pathKey(row.path))).map(row => ({...row,password:'',resultPassword:'',
      resultPasswordRequired:!!row.result?.outputEncrypted,needsRecheck:!!row.report,
      state:(row.result ? 'repaired' : 'pending') as ArchiveRecoveryRow['state']}))
    recovery.value = null
    sourcePaths.value = [...new Set([...saved.paths,...sourcePaths.value])]
    rootDirectory.value ||= saved.rootDirectory
    rows.value = [...restored,...rows.value]
    persist()
  }
  function discardPrevious() { if (busy.value || preparing.value) return; recovery.value = null; persist() }
  function clear() {
    if (busy.value || preparing.value) return
    for (const row of rows.value) { row.password = ''; row.resultPassword = '' }
    rows.value = []; sourcePaths.value = []; rootDirectory.value = ''; persist()
  }
  const selected = computed(() => rows.value.filter(row => row.selected))
  const repairable = computed(() => selected.value.filter(row => row.report?.canRepair && !row.needsRecheck && !row.result))
  const queueable = computed(() => selected.value.filter(row => row.result && !row.queuedId))
  const completed = computed(() => rows.value.filter(row => !row.needsRecheck && (row.report || row.error)).length)
  let stopped = false, generation = 0
  let active: {id: string; kind: 'check' | 'repair'} | null = null
  async function load(paths: string[]) {
    if (sourcePaths.value.length === paths.length && paths.every((path,index) => pathKey(path) === pathKey(sourcePaths.value[index]))) return
    if (busy.value || preparing.value) throw new Error('请等待当前检查修复结束或停止后再选择压缩包')
    const current = ++generation
    preparing.value = true
    try {
      const groups = await commands.groupArchiveDiagnosisSources(paths)
      if (current !== generation) return
      for (const row of rows.value) { row.password = ''; row.resultPassword = '' }
      sourcePaths.value = [...paths]
      rows.value = groups.map(group => ({id:generateId(),path:group.path,selectedMembers:group.selectedMembers,selected:true,expanded:false,password:'',outputDirectory:'',extractDirectory:'',state:'pending'}))
    } finally { if (current === generation) preparing.value = false }
  }
  async function stop() {
    stopped = true
    if (!active) return
    try {
      if (active.kind === 'check') await commands.cancelArchiveDiagnosis(active.id)
      else await commands.cancelZipRepair(active.id)
    } catch { /* the active command may have just finished */ }
  }
  async function check(items = selected.value) {
    if (busy.value || preparing.value || !items.length) return
    busy.value = true; stopped = false
    try {
      for (const row of [...items]) {
        if (stopped) break
        row.error = ''; row.report = undefined; row.state = 'checking'
        active = {id:`batch-check-${generateId()}`,kind:'check'}
        try {
          const encoding = row.sourceFormat ? zipPasswordEncodingForSource(row.path, row.zipPasswordEncoding, row.sourceFormat) : row.zipPasswordEncoding
          row.report = await commands.diagnoseArchive(active.id, row.path, row.password, ...zipPasswordEncodingArguments(encoding))
          row.sourceFormat = row.report.actualFormat; row.needsRecheck = false; row.state = 'checked'
        }
        catch (error) { row.state = stopped || String(error).toLowerCase().includes('cancel') ? 'cancelled' : 'error'; row.error = row.state === 'error' ? String(error) : '' }
        finally { active = null }
      }
    } finally { busy.value = false }
  }
  async function repair(fallbackDirectory: string) {
    if (busy.value || preparing.value || !repairable.value.length) return
    const targets = [...repairable.value]
    if (targets.some(row => !(row.outputDirectory || fallbackDirectory).trim())) throw new Error('请先选择恢复文件保存目录')
    busy.value = true; stopped = false
    try {
      for (const row of targets) {
        if (stopped) break
        row.error = ''; row.state = 'repairing'
        active = {id:`batch-repair-${generateId()}`,kind:'repair'}
        try {
          const format = row.report!.actualFormat
          const encoding = zipPasswordEncodingForSource(row.path, row.zipPasswordEncoding, format)
          const result = await commands.repairArchiveBatchItem(active.id, row.path, row.outputDirectory || fallbackDirectory, format, row.password, ...zipPasswordEncodingArguments(encoding))
          row.resultZipPasswordEncoding = result.outputEncrypted && supportsZipPasswordEncoding(row.path, format) ? encoding ?? 'utf8' : undefined
          row.result = result; row.resultPassword = result.outputEncrypted ? row.password : ''; row.resultPasswordRequired = false; row.queuedId = undefined; row.state = 'repaired'; row.expanded = true
        } catch (error) { row.state = stopped || String(error).toLowerCase().includes('cancel') ? 'cancelled' : 'error'; row.error = row.state === 'error' ? String(error) : '' }
        finally { active = null }
      }
    } finally { busy.value = false }
  }
  async function queue() {
    if (busy.value || preparing.value) return 0
    let count = 0
    const candidates = [...queueable.value]
    busy.value = true
    try { for (const row of candidates) {
      const result = row.result!
      if (row.resultPasswordRequired && !row.resultPassword) { row.error = '请填写恢复结果密码后再加入解压队列'; row.expanded = true; continue }
      try {
        for (const path of result.outputFiles?.length ? result.outputFiles : [result.outputPath]) await invoke('get_file_info',{path})
      } catch { row.error = '恢复结果已移动、删除或无法读取，请定位结果或重新检查并修复原包'; row.expanded = true; continue }
      const id = generateId()
      const destination = row.extractDirectory || parentDirectory(row.path)
      const stem = archiveSourceStem(row.path)
      const peers = rows.value.filter(other => (other.extractDirectory || parentDirectory(other.path)).replace(/\\/g,'/').toLowerCase() === destination.replace(/\\/g,'/').toLowerCase() && archiveSourceStem(other.path).toLowerCase() === stem.toLowerCase())
      const outputPath = peers.length > 1 ? `${destination.replace(/[\\/]$/,'')}/${stem}-恢复-${peers.indexOf(row)+1}` : destination
      tasks.addTask({id,name:result.outputPath.split(/[\\/]/).pop() || result.outputPath,type:'decompression',workloadKind:'archive',sourceFiles:[result.outputPath],outputPath,extractToSubfolder:true,configurationMode:'individual',recycleSourceAfterExtract:false,password:row.resultPassword || undefined,
        zipPasswordEncoding:result.outputEncrypted && supportsZipPasswordEncoding(result.outputPath,result.format) ? row.resultZipPasswordEncoding ?? 'utf8' : undefined})
      row.error = ''; row.queuedId = id; count++
    } } finally { busy.value = false }
    return count
  }
  function forgetResult(row:ArchiveRecoveryRow) { if (busy.value) return; row.result = undefined; row.resultPassword = ''; row.resultZipPasswordEncoding = undefined; row.queuedId = undefined; row.error = ''; row.needsRecheck = true; row.state = 'pending' }
  function update(row: ArchiveRecoveryRow, patch: Partial<Pick<ArchiveRecoveryRow,'password'|'outputDirectory'|'extractDirectory'|'zipPasswordEncoding'>>) {
    if (busy.value) return
    if ((patch.password !== undefined && patch.password !== row.password)
      || (Object.hasOwn(patch,'zipPasswordEncoding') && patch.zipPasswordEncoding !== row.zipPasswordEncoding)) { row.report = undefined; row.error = ''; row.state = row.result ? 'repaired' : 'pending' }
    Object.assign(row, patch)
  }
  return {rows,busy,preparing,selected,repairable,queueable,completed,sourcePaths,rootDirectory,saveError,recoveryCount,restore,discardPrevious,clear,load,check,repair,stop,queue,update,forgetResult}
})
export function useArchiveRecoveryBatch() {
  const store = useArchiveRecoveryStore()
  return {...storeToRefs(store),load:store.load,check:store.check,repair:store.repair,stop:store.stop,queue:store.queue,update:store.update,forgetResult:store.forgetResult}
}
