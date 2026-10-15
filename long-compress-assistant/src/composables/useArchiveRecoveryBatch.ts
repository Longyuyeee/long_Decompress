import { computed, onUnmounted, ref } from 'vue'
import { useTauriCommands, type ArchiveDiagnosticReport, type ZipRepairResult } from './useTauriCommands'
import { useTaskStore } from '@/stores/task'
import { generateId } from '@/utils'
import { parentDirectory } from '@/utils/resultLocation'
import { archiveSourceStem } from '@/utils/archiveSource'

export interface ArchiveRecoveryRow {
  id: string; path: string; selectedMembers: number; selected: boolean; expanded: boolean
  password: string; outputDirectory: string; extractDirectory: string
  state: 'pending' | 'checking' | 'checked' | 'repairing' | 'repaired' | 'error' | 'cancelled'
  report?: ArchiveDiagnosticReport; result?: ZipRepairResult; resultPassword?: string; queuedId?: string; error?: string
}
export function useArchiveRecoveryBatch() {
  const commands = useTauriCommands(), tasks = useTaskStore()
  const rows = ref<ArchiveRecoveryRow[]>([]), busy = ref(false), preparing = ref(false)
  const selected = computed(() => rows.value.filter(row => row.selected))
  const repairable = computed(() => selected.value.filter(row => row.report?.canRepair && !row.result))
  const queueable = computed(() => selected.value.filter(row => row.result && !row.queuedId))
  const completed = computed(() => rows.value.filter(row => row.report || row.error).length)
  let stopped = false, generation = 0
  let active: {id: string; kind: 'check' | 'repair'} | null = null
  async function load(paths: string[]) {
    const current = ++generation
    for (const row of rows.value) { row.password = ''; row.resultPassword = '' }
    rows.value = []
    preparing.value = true
    try {
      const groups = await commands.groupArchiveDiagnosisSources(paths)
      if (current !== generation) return
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
        try { row.report = await commands.diagnoseArchive(active.id, row.path, row.password); row.state = 'checked' }
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
          const result = await commands.repairArchiveBatchItem(active.id, row.path, row.outputDirectory || fallbackDirectory, row.report!.actualFormat, row.password)
          row.result = result; row.resultPassword = result.outputEncrypted ? row.password : ''; row.queuedId = undefined; row.state = 'repaired'; row.expanded = true
        } catch (error) { row.state = stopped || String(error).toLowerCase().includes('cancel') ? 'cancelled' : 'error'; row.error = row.state === 'error' ? String(error) : '' }
        finally { active = null }
      }
    } finally { busy.value = false }
  }
  function queue() {
    let count = 0
    for (const row of queueable.value) {
      const result = row.result!
      const id = generateId()
      const destination = row.extractDirectory || parentDirectory(row.path)
      const stem = archiveSourceStem(row.path)
      const peers = rows.value.filter(other => (other.extractDirectory || parentDirectory(other.path)).replace(/\\/g,'/').toLowerCase() === destination.replace(/\\/g,'/').toLowerCase() && archiveSourceStem(other.path).toLowerCase() === stem.toLowerCase())
      const outputPath = peers.length > 1 ? `${destination.replace(/[\\/]$/,'')}/${stem}-恢复-${peers.indexOf(row)+1}` : destination
      tasks.addTask({id,name:result.outputPath.split(/[\\/]/).pop() || result.outputPath,type:'decompression',workloadKind:'archive',sourceFiles:[result.outputPath],outputPath,extractToSubfolder:true,configurationMode:'individual',recycleSourceAfterExtract:false,password:row.resultPassword || undefined})
      row.queuedId = id; count++
    }
    return count
  }
  function update(row: ArchiveRecoveryRow, patch: Partial<Pick<ArchiveRecoveryRow,'password'|'outputDirectory'|'extractDirectory'>>) {
    if (busy.value) return
    if (patch.password !== undefined && patch.password !== row.password) { row.report = undefined; row.error = ''; row.state = row.result ? 'repaired' : 'pending' }
    Object.assign(row, patch)
  }
  onUnmounted(() => { generation++; void stop(); for (const row of rows.value) { row.password = ''; row.resultPassword = '' } })
  return {rows,busy,preparing,selected,repairable,queueable,completed,load,check,repair,stop,queue,update}
}
