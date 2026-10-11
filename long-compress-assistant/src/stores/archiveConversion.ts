import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useTaskStore } from './task'
import type { TaskHistoryRecord } from '@/types/taskHistory'
import type { ArchiveConversionConfigV1 } from '@/types/executionConfig'
import type { PublishedArchiveOutput } from '@/types/archiveOutput'

export const conversionFormats = [
  ['zip', 'ZIP'], ['7z', '7Z'], ['tar', 'TAR'], ['tar.gz', 'TGZ'],
  ['tar.bz2', 'TBZ'], ['tar.xz', 'TXZ'], ['tar.zst', 'TZST'], ['wim', 'WIM'],
].map(([value, label]) => ({ value, label }))
export const useArchiveConversionStore = defineStore('archiveConversion', () => {
  const tasks = useTaskStore()
  const visible = ref(false)
  const sourcePath = ref('')
  const sourcePassword = ref('')
  const password = ref('')
  const outputPath = ref('')
  const format = ref('7z')
  const zipEncryption = ref<'aes256' | 'zipcrypto'>('aes256')
  const zipPasswordEncoding = ref<'utf8' | 'windows_local'>('utf8')
  const level = ref(6)
  const splitMiB = ref(0)
  const encryptFilenames = ref(false)
  const taskId = ref('')
  const retryOfTaskId = ref<string>()
  const error = ref('')
  const busy = ref(false)
  const phase = ref('')
  const task = computed(() => tasks.tasks.find(task => task.id === taskId.value))
  const supportsPassword = computed(() => ['zip', '7z'].includes(format.value))
  function show() { visible.value = true }
  function restore(record: TaskHistoryRecord) {
    if (busy.value || record.executionConfig?.kind !== 'archive-conversion') return
    const config = record.executionConfig
    sourcePath.value = record.sourcePaths[0] || ''
    outputPath.value = '' // Require a new destination, never overwrite the previous result.
    format.value = config.format
    zipEncryption.value = config.zipEncryption ?? 'aes256'
    zipPasswordEncoding.value = config.zipPasswordEncoding ?? 'utf8'
    level.value = config.level
    splitMiB.value = config.splitSize / 1048576
    encryptFilenames.value = config.encryptFilenames
    sourcePassword.value = ''; password.value = ''; error.value = ''; taskId.value = ''
    retryOfTaskId.value = record.id
    visible.value = true
  }
  async function start() {
    if (busy.value) return
    error.value = ''
    if (!sourcePath.value.trim() || !outputPath.value.trim()) { error.value = '请选择原压缩包和新包保存位置'; return }
    if (!conversionFormats.some(item => item.value === format.value)) { error.value = '请选择支持的目标格式'; return }
    if (!Number.isInteger(level.value) || level.value < 0 || level.value > 9) { error.value = '压缩强度应为 0～9'; return }
    if (!Number.isFinite(splitMiB.value) || splitMiB.value < 0 || !Number.isSafeInteger(splitMiB.value * 1048576)) { error.value = '请输入有效分卷大小'; return }
    if (encryptFilenames.value && (format.value !== '7z' || !password.value)) { error.value = '文件名加密需要选择 7Z 并填写新包密码'; return }
    if (!supportsPassword.value && (password.value || splitMiB.value > 0)) { error.value = '此格式不支持密码或分卷，请选择 ZIP 或 7Z'; return }
    if (!outputPath.value.toLowerCase().endsWith(`.${format.value}`)) { error.value = `新包名称必须以 .${format.value} 结尾`; return }
    const config: ArchiveConversionConfigV1 = { schemaVersion: 1, kind: 'archive-conversion',
      zipEncryption: zipEncryption.value, zipPasswordEncoding: zipPasswordEncoding.value,
      outputPath: outputPath.value.trim(), format: format.value, level: level.value,
      splitSize: splitMiB.value * 1048576, encryptFilenames: encryptFilenames.value, passwordRequired: !!password.value }
    const source = sourcePath.value.trim()
    const inputPassword = sourcePassword.value || null
    const targetPassword = password.value || null
    busy.value = true
    let unlisten: UnlistenFn | undefined
    try {
      await tasks.initListeners()
      const id = `conversion-${Date.now()}-${Math.random().toString(16).slice(2)}`
      taskId.value = id
      phase.value = '步骤 1/2：读取原包'
      unlisten = await listen<{ taskId: string; phase: string }>('archive-conversion-phase', ({ payload }) => {
        if (payload.taskId === id) phase.value = payload.phase === 'writing' ? '步骤 2/2：重建、校验并发布新包' : '步骤 1/2：读取原包'
      })
      tasks.addTask({ id, name: `归档转换 · ${source.split(/[\\/]/).pop()}`, type: 'compression', workloadKind: 'archive',
        sourceFiles: [source], outputPath: config.outputPath, format: config.format,
        executionConfig: config, retryOfTaskId: retryOfTaskId.value })
      tasks.updateTaskStatus(id, 'running')
      try {
        const output = await invoke<PublishedArchiveOutput>('convert_archive', {
          taskId: id, sourcePath: source, sourcePassword: inputPassword, outputPath: config.outputPath,
          options: { format: config.format, level: config.level, password: targetPassword,
            zip_encryption: config.zipEncryption, zip_password_encoding: config.zipPasswordEncoding,
            split_size: config.splitSize || null, encrypt_filenames: config.encryptFilenames,
            preserve_paths: true, delete_after: false, verify_after: true },
        })
        const current = tasks.tasks.find(item => item.id === id)!
        current.archiveOutput = output
        current.outputBytes = output.totalBytes
        current.outputBytesEstimated = false
        tasks.updateTaskStatus(id, 'completed')
      } catch (failure) {
        error.value = String(failure)
        if (/cancelled|canceled|已取消|操作取消/i.test(error.value)) { error.value = ''; tasks.updateTaskStatus(id, 'cancelled') }
        else tasks.failTask(id, error.value)
      }
      await tasks.waitForHistoryPersistence(id)
    } catch (failure) { error.value = String(failure) }
    finally { unlisten?.(); busy.value = false; sourcePassword.value = ''; password.value = '' }
  }
  async function cancel() {
    if (!busy.value || !task.value || task.value.status === 'cancelling') return
    try {
      await invoke('cancel_compression', { taskId: taskId.value })
      if (busy.value) tasks.updateTaskStatus(taskId.value, 'cancelling')
    } catch (failure) { error.value = `取消请求失败：${String(failure)}` }
  }
  return { visible, sourcePath, sourcePassword, password, outputPath, format, zipEncryption, zipPasswordEncoding, level, splitMiB,
    encryptFilenames, error, busy, task, phase, supportsPassword, show, restore, start, cancel }
})
