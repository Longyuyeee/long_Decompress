import { isZpaqSnapshot, snapshotArguments, snapshotKey, type ZpaqSnapshot } from '@/types/zpaqSnapshot'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { ask } from '@tauri-apps/api/dialog'
import { useTaskStore } from './task'
import type { TaskHistoryRecord } from '@/types/taskHistory'
import type { ArchiveConversionConfigV1 } from '@/types/executionConfig'
import type { PublishedArchiveOutput } from '@/types/archiveOutput'
import type { ConversionDraft, ConversionPatch, ConversionSettings } from '@/types/archiveConversion'
import { conversionOutputPath, defaultConversionSettings, normalizeConversionSettings } from '@/utils/archiveConversionDraft'
import { parentDirectory } from '@/utils/resultLocation'
import { archiveGroupKey, archiveSourceFormat, detectArchiveGroup } from '@/utils/archiveSource'
import { isPotentialSplitArchivePath, supportsNativePassword, supportsArchiveSplit, supportsFilenameEncryption } from '@/utils/compressionFormat'
import { supportsZipPasswordEncoding, zipPasswordEncodingForSource } from '@/utils/zipPasswordSource'

export const conversionFormats = [
  ['zpaq', 'ZPAQ'],
  ['zip', 'ZIP'], ['zipx', 'ZIPX（XZ）'], ['7z', '7Z'], ['rar', 'RAR（需要 WinRAR）'], ['tar', 'TAR'], ['tar.gz', 'TGZ'],
  ['tar.bz2', 'TBZ'], ['tar.xz', 'TXZ'], ['tar.zst', 'TZST'],
  ['tar.lz4', 'TAR.LZ4（快速）'], ['tar.br', 'TAR.BR（Brotli）'],
  ['tar.lz', 'TAR.LZIP'], ['lzma', 'LZMA（单文件）'],
  ['tar.lzma', 'TAR.LZMA（多文件）'],
  ['lzma.aes', 'LZMA.AES（单文件加密）'],
  ['tar.lzma.aes', 'TAR.LZMA.AES（多文件加密）'],
  ['lz4.aes', 'LZ4.AES（单文件加密）'],
  ['br.aes', 'BR.AES（单文件加密）'],
  ['lz.aes', 'LZIP.AES（单文件加密）'],
  ['tar.lz4.aes', 'TAR.LZ4.AES（加密）'],
  ['tar.br.aes', 'TAR.BR.AES（加密）'],
  ['tar.lz.aes', 'TAR.LZIP.AES（加密）'],
  ['tar.aes', 'TAR.AES（加密）'], ['tar.gz.aes', 'TGZ.AES（加密）'],
  ['tar.bz2.aes', 'TBZ.AES（加密）'], ['tar.xz.aes', 'TXZ.AES（加密）'], ['tar.zst.aes', 'TZST.AES（加密）'], ['wim', 'WIM'],
].map(([value, label]) => ({ value, label }))
const pathKey = (path: string) => path.replace(/\\/g, '/').toLowerCase()
export const useArchiveConversionStore = defineStore('archiveConversion', () => {
  const tasks = useTaskStore()
  const visible = ref(false)
  const items = ref<ConversionDraft[]>([])
  const defaults = ref(defaultConversionSettings())
  const activeId = ref('')
  const error = ref('')
  const busy = ref(false)
  const phase = ref('')
  const runningTaskId = ref('')
  const stopRequested = ref(false)
  const preparing = ref(false)
  const batchNotice = ref('')
  const rarSupport = ref<{ available: boolean; message?: string } | null>(null)
  let rarCheck: Promise<void> | undefined
  async function refreshRarSupport() {
    if (rarCheck) return rarCheck
    rarCheck = invoke<{ available: boolean; message?: string }>('check_rar_compression_support')
      .then(support => { rarSupport.value = support || { available: false } })
      .catch(failure => { rarSupport.value = { available: false, message: String(failure) } })
      .finally(() => { rarCheck = undefined })
    return rarCheck
  }
  const sourceResolutions = new Map<string, Promise<void>>()
  const active = computed(() => items.value.find(item => item.id === activeId.value))
  function effectiveSettings(item: ConversionDraft): ConversionSettings {
    const settings = item.settings || defaults.value
    return !item.settings && item.defaultFormat ? { ...settings, format: item.defaultFormat } : { ...settings }
  }
  function outputFor(item: ConversionDraft) { return conversionOutputPath(item.sourcePath, effectiveSettings(item), item.intent) }
  function taskFor(item: ConversionDraft) { return tasks.tasks.find(task => task.id === item.taskId) }
  function passwordRequirements(item: ConversionDraft) {
    const legacy = !!item.passwordRequired && item.sourcePasswordRequired === undefined && item.targetPasswordRequired === undefined
    return { source: item.sourcePasswordRequired ?? legacy, target: item.targetPasswordRequired ?? legacy }
  }
  const task = computed(() => active.value ? taskFor(active.value) : undefined)
  const contextSources = computed(() => items.value.map(item => item.sourcePath))
  const contextIntent = computed(() => active.value?.intent || 'convert')
  const pending = computed(() => items.value.filter(item => !item.taskId))
  const selected = computed(() => pending.value.filter(item => item.selected))
  const currentSettings = computed(() => active.value ? effectiveSettings(active.value) : defaults.value)
  function updateItem(id: string, patch: ConversionPatch) {
    const item = items.value.find(item => item.id === id)
    if (!item || item.taskId || busy.value || preparing.value) return
    item.settings = normalizeConversionSettings(effectiveSettings(item), patch)
    if (patch.outputDirectory !== undefined && item.settings.outputPath) {
      const filename = item.settings.outputPath.split(/[\\/]/).pop()
      const directory = patch.outputDirectory || parentDirectory(item.sourcePath)
      item.settings.outputPath = directory.replace(/[\\/]$/, '') + '/' + filename
    }
    const requirements = passwordRequirements(item)
    item.sourcePasswordRequired = patch.sourcePassword !== undefined ? false : requirements.source
    item.targetPasswordRequired = patch.password !== undefined || !supportsNativePassword(item.settings.format) ? false : requirements.target
    item.passwordRequired = item.sourcePasswordRequired || item.targetPasswordRequired
    item.error = undefined
  }
  function field<K extends keyof ConversionSettings>(key: K) {
    return computed({ get: () => key === 'outputPath' && active.value ? outputFor(active.value) as ConversionSettings[K] : currentSettings.value[key],
      set: (value: ConversionSettings[K]) => {
        if (active.value) updateItem(active.value.id, { [key]: value })
        else defaults.value = normalizeConversionSettings(defaults.value, { [key]: value })
      } })
  }
  const sourcePath = computed({ get: () => active.value?.sourcePath || '', set: (path: string) => selectSource(path) })
  const sourcePassword = field('sourcePassword'), password = field('password'), outputPath = field('outputPath')
  const sourceZipPasswordEncoding = field('sourceZipPasswordEncoding')
  const format = field('format'), zipEncryption = field('zipEncryption'), zipPasswordEncoding = field('zipPasswordEncoding')
  const level = field('level'), splitMiB = field('splitMiB'), encryptFilenames = field('encryptFilenames')
  const supportsPassword = computed(() => supportsNativePassword(format.value))
  function addSources(paths: string[], intent: ConversionDraft['intent'] = 'convert') {
    const added: ConversionDraft[] = []
    for (const path of paths.map(path => path.trim()).filter(Boolean)) {
      const existing = items.value.find(item => archiveGroupKey(item.sourcePath) === archiveGroupKey(path) && !item.taskId)
      if (existing) { existing.selected = true; queueSourceResolution(existing, path); continue }
      const sourceFormat = archiveSourceFormat(path)
      const item: ConversionDraft = { id: 'conversion-draft-' + crypto.randomUUID(), sourcePath: path, intent,
        defaultFormat: intent === 'reconfigure' ? conversionFormats.some(option => option.value === sourceFormat) ? sourceFormat : '7z' : undefined,
        expanded: false, selected: true }
      items.value.push(item); added.push(item)
      queueSourceResolution(item, path)
    }
    if (!active.value && items.value.length) activeId.value = items.value[0].id
    return added
  }
  function queueSourceResolution(item: ConversionDraft, reference = item.sourceReference || item.sourcePath) {
    const original = item.sourcePath
    const resolution = (sourceResolutions.get(item.id) || Promise.resolve()).then(async () => {
      const current = items.value.find(current => current.id === item.id)
      if (!current || current.sourcePath !== original || current.taskId) return
      item = current
      try {
        const group = await detectArchiveGroup(reference)
        if (!items.value.includes(item) || item.sourcePath !== original || item.taskId) return
        // Undefined is only possible in browser previews without native IPC.
        if (!group) return
        item.sourceFormat = group.archive_format?.toLowerCase() || undefined
        if (!group.is_split) {
          if (isPotentialSplitArchivePath(reference)) throw new Error('没有找到完整卷组，请重新选择来源。')
          item.sourceIssue = undefined
          if (item.intent === 'reconfigure' && !item.settings && item.defaultFormat !== undefined && item.sourceFormat && conversionFormats.some(option => option.value === item.sourceFormat)) item.defaultFormat = item.sourceFormat
          return
        }
        if (!group.first_part) throw new Error('没有找到完整卷组，请重新选择来源。')
        item.sourcePath = group.first_part
        item.sourceReference = reference
        item.sourceParts = group.parts
        if (item.intent === 'reconfigure' && !item.settings && item.defaultFormat !== undefined) {
          const format = item.sourceFormat || archiveSourceFormat(group.first_part)
          if (format && conversionFormats.some(option => option.value === format)) item.defaultFormat = format
        }
        const duplicate = items.value.find(current => current.id !== item.id && !current.taskId
          && archiveGroupKey(current.sourcePath) === archiveGroupKey(item.sourcePath)
          && snapshotKey(effectiveSettings(current).sourceZpaqSnapshot) === snapshotKey(effectiveSettings(item).sourceZpaqSnapshot))
        if (duplicate) {
          duplicate.selected ||= item.selected
          duplicate.settings ||= item.settings
          if (activeId.value === item.id) activeId.value = duplicate.id
          items.value = items.value.filter(current => current.id !== item.id)
          return
        }
        const missing = group.missing_parts || []
        item.sourceIssue = group.is_complete === false || missing.length
          ? '分卷不完整，请补齐：' + (missing.map(path => path.split(/[\\/]/).pop()).join('、') || '缺失分卷') : undefined
        if (item.sourceIssue) item.expanded = true
      } catch (failure) { if (items.value.includes(item) && item.sourcePath === original && !item.taskId) { item.sourceFormat = undefined; item.sourceIssue = String(failure); item.expanded = true } }
    })
    sourceResolutions.set(item.id, resolution)
    void resolution.finally(() => { if (sourceResolutions.get(item.id) === resolution) sourceResolutions.delete(item.id) })
  }
  function chooseContextSource(path: string) {
    if (busy.value) return
    const item = items.value.find(item => archiveGroupKey(item.sourcePath) === archiveGroupKey(path))
    if (item) { activeId.value = item.id; item.expanded = true }
  }
  function selectSource(path: string) {
    if (busy.value || !path.trim()) return
    addSources([path], contextIntent.value)
    chooseContextSource(path)
  }
  function openFromExplorer(paths: string[], intent: ConversionDraft['intent']) {
    addSources(paths, intent)
    visible.value = true
  }
  function show() { visible.value = true }
  function openSnapshot(path: string, snapshot: ZpaqSnapshot, sourcePassword = '') {
    const existing = pending.value.find(item => archiveGroupKey(item.sourcePath) === archiveGroupKey(path)
      && snapshotKey(effectiveSettings(item).sourceZpaqSnapshot) === snapshotKey(snapshot))
    if (existing) { existing.expanded = true; existing.selected = true; activeId.value = existing.id; visible.value = true; return }
    const item: ConversionDraft = { id: 'conversion-draft-' + crypto.randomUUID(), sourcePath: path,
      sourceFormat: 'zpaq', intent: 'convert', expanded: true, selected: true,
      settings: { ...defaults.value, outputPath: '', sourcePassword, sourceZpaqSnapshot: { ...snapshot } } }
    items.value.push(item); queueSourceResolution(item); activeId.value = item.id; visible.value = true
  }
  async function applySelected(patch: ConversionPatch) {
    if (busy.value || preparing.value) return '正在读取或转换，请稍后修改。'
    batchNotice.value = ''
    const chosen = [...selected.value]
    const sourceEncodingChanged = Object.hasOwn(patch, 'sourceZipPasswordEncoding')
    if (sourceEncodingChanged) {
      preparing.value = true
      try {
        await Promise.all(chosen.map(item => sourceResolutions.get(item.id)))
        for (const item of chosen) if (items.value.includes(item) && !item.taskId) queueSourceResolution(item)
        await Promise.all(chosen.map(item => sourceResolutions.get(item.id)))
      } finally { preparing.value = false }
    }
    const targets = chosen.filter(item => items.value.includes(item) && item.selected && !item.taskId)
    const methodOnly = !sourceEncodingChanged && patch.zipMethod !== undefined && Object.entries(patch).every(([field, value]) => field === 'zipMethod' || value === undefined)
    if (methodOnly && !targets.some(item => effectiveSettings(item).format === 'zip')) return '所选任务中没有 ZIP 任务，请选择 ZIP 格式或保持各自方法。'
    if (patch.rarRecoveryPercent && targets.some(item => (patch.format || effectiveSettings(item).format) !== 'rar')) return '恢复保护只适用于 RAR，请调整所选任务的目标格式。'
    if (patch.rarRecoveryVolumes && targets.some(item => (patch.format || effectiveSettings(item).format) !== 'rar')) return '恢复卷只适用于 RAR 分包，请调整所选任务的目标格式。'
    if (patch.encryptFilenames && targets.some(item => !supportsFilenameEncryption(patch.format || effectiveSettings(item).format))) {
      return '文件名加密只支持 7Z/RAR，请调整所选任务的目标格式。'
    }
    let applied = 0, skippedSource = 0, skippedTarget = 0
    for (const item of targets) {
      const filtered = { ...patch }, format = patch.format || effectiveSettings(item).format
      if (filtered.zipMethod !== undefined && format !== 'zip') delete filtered.zipMethod
      if (filtered.zipPasswordEncoding !== undefined && !['zip','zipx'].includes(format)) { delete filtered.zipPasswordEncoding; skippedTarget++ }
      if (sourceEncodingChanged && (!item.sourceFormat || !supportsZipPasswordEncoding(item.sourcePath, item.sourceFormat))) { delete filtered.sourceZipPasswordEncoding; skippedSource++ }
      if (!Object.keys(filtered).length) continue
      updateItem(item.id, filtered); applied++
    }
    if (skippedSource || skippedTarget) batchNotice.value = `${skippedSource ? `${skippedSource} 项原包不适用读取编码或格式未确认，未修改。` : ''}${skippedTarget ? `${skippedTarget} 项目标非 ZIP/ZIPX，未修改新包编码。` : ''}`
    if (!applied && (sourceEncodingChanged || patch.zipPasswordEncoding !== undefined)) return '所选任务没有适用的原包读取或新包 ZIP 密码编码设置。' + batchNotice.value
    return ''
  }
  function updateDefaults(patch: ConversionPatch) {
    if (busy.value || preparing.value) return
    defaults.value = normalizeConversionSettings(defaults.value, patch)
    if (patch.format !== undefined) for (const item of pending.value) if (!item.settings) item.defaultFormat = undefined
  }
  function remove(id: string) {
    if (busy.value || preparing.value) return
    items.value = items.value.filter(item => item.id !== id)
    if (!active.value) activeId.value = items.value[0]?.id || ''
  }
  function replaceSource(id: string, path: string) {
    const item = items.value.find(item => item.id === id)
    if (!item || busy.value || preparing.value || item.taskId) return
    if (item.settings) item.settings.sourceZpaqSnapshot = undefined
    item.sourcePath = path; item.sourceIssue = undefined; item.error = undefined
    item.sourceParts = undefined
    item.sourceReference = undefined
    item.sourceFormat = undefined
    queueSourceResolution(item)
  }
  function restore(record: TaskHistoryRecord) {
    if (busy.value || record.executionConfig?.kind !== 'archive-conversion') return
    const existing = pending.value.find(item => item.retryOfTaskId === record.id)
    if (existing) {
      existing.expanded = existing.selected = true; activeId.value = existing.id; visible.value = true
      return
    }
    const config = record.executionConfig
    const source = record.sourcePaths[0]
    if (!source) return
    const item: ConversionDraft = { id: 'conversion-draft-' + crypto.randomUUID(), sourcePath: source,
      sourceParts: record.sourcePaths.length > 1 ? [...record.sourcePaths] : undefined,
      intent: 'convert', expanded: true, selected: true, retryOfTaskId: record.id,
      targetPasswordRequired: config.passwordRequired, sourcePasswordRequired: false, passwordRequired: config.passwordRequired,
      settings: { ...defaultConversionSettings(), sourceZpaqSnapshot: config.sourceZpaqSnapshot, outputPath: '', format: config.format,
      ...(config.sourceZipPasswordEncoding ? { sourceZipPasswordEncoding: config.sourceZipPasswordEncoding } : {}),
      zipMethod: config.zipMethod ?? 'deflate',
      zipEncryption: config.zipEncryption ?? 'aes256', zipPasswordEncoding: config.zipPasswordEncoding ?? 'utf8',
      zipSplitMode: config.zipSplitMode ?? 'numeric', rarRecoveryPercent: config.rarRecoveryPercent ?? 0,
      rarRecoveryVolumes: config.rarRecoveryVolumes ?? 0,
      level: config.level, splitMiB: config.splitSize / 1048576, encryptFilenames: config.encryptFilenames } }
    const occupied = new Set([...pending.value.map(outputFor), record.outputPath].map(pathKey))
    const proposed = outputFor(item)
    for (let suffix = 2; occupied.has(pathKey(outputFor(item))); suffix++) {
      item.settings!.outputPath = proposed.slice(0, -(config.format.length + 1)) + '_' + suffix + '.' + config.format
    }
    items.value.push(item)
    queueSourceResolution(item)
    activeId.value = item.id; error.value = ''; visible.value = true
  }
  function validate(item: ConversionDraft) {
    const settings = effectiveSettings(item), output = outputFor(item)
    if (item.sourceIssue) return item.sourceIssue
    if (settings.sourceZpaqSnapshot && !isZpaqSnapshot(settings.sourceZpaqSnapshot)) return '快照标识无效，请重新读取来源的历史快照。'
    const passwords = passwordRequirements(item)
    if (passwords.source) return '请重新填写原包密码，或明确选择原包无需密码。'
    if (passwords.target) return '请重新填写新包密码，或明确选择新包不加密。'
    if (!item.sourcePath.trim() || !output.trim()) return '请选择原压缩包和新包保存位置'
    if (!conversionFormats.some(format => format.value === settings.format)) return '请选择支持的目标格式'
    if (!Number.isInteger(settings.level) || settings.level < 0 || settings.level > 9) return '压缩强度应为 0～9'
    if (!Number.isFinite(settings.splitMiB) || settings.splitMiB < 0 || !Number.isSafeInteger(settings.splitMiB * 1048576)) return '请输入有效分卷大小'
    if (settings.encryptFilenames && (!supportsFilenameEncryption(settings.format) || !settings.password)) return '文件名加密需要选择 7Z/RAR 并填写新包密码'
    if (settings.format.endsWith('.aes') && !settings.password) return '此加密格式需要填写新包密码'
    if (settings.password && !supportsNativePassword(settings.format)) return '此格式不支持原生密码，请选择 ZIP、7Z、RAR 或加密包装格式'
    if (settings.splitMiB > 0 && !supportsArchiveSplit(settings.format)) return '此格式暂不支持分卷'
    if (['zip', 'zipx'].includes(settings.format) && settings.zipSplitMode === 'standard' && settings.splitMiB > 0 && (settings.splitMiB < 0.0625 || settings.splitMiB >= 4096)) return '标准 ZIP 每卷大小需要在 64 KiB 至 4 GiB（不含）之间'
    if (!output.toLowerCase().endsWith('.' + settings.format)) return '新包名称必须以 .' + settings.format + ' 结尾'
    if (pathKey(output) === pathKey(item.sourcePath)) return '请为新包设置不同的名称或保存位置'
    if (/[<>:"|?*]/.test(output.split(/[\\/]/).pop() || '')) return '新包名称包含不可用字符，请修改名称'
    return ''
  }
  async function execute(item: ConversionDraft, settings: ConversionSettings, outputPath: string, allowRarPasswordCli = false) {
    let unlisten: UnlistenFn | undefined
    const id = 'conversion-' + crypto.randomUUID()
    const sourceEncoding = item.sourceFormat ? zipPasswordEncodingForSource(item.sourcePath, settings.sourceZipPasswordEncoding, item.sourceFormat) : settings.sourceZipPasswordEncoding
    const config: ArchiveConversionConfigV1 = { schemaVersion: 1, kind: 'archive-conversion',
      ...(settings.sourceZpaqSnapshot ? { sourceZpaqSnapshot: { ...settings.sourceZpaqSnapshot } } : {}),
      ...(sourceEncoding ? { sourceZipPasswordEncoding: sourceEncoding } : {}),
      zipMethod: settings.zipMethod ?? 'deflate',
      zipEncryption: settings.zipEncryption, zipPasswordEncoding: ['zip','zipx'].includes(settings.format) ? settings.zipPasswordEncoding : 'utf8',
      zipSplitMode: settings.zipSplitMode ?? 'numeric',
      outputPath, format: settings.format, level: settings.level, splitSize: settings.splitMiB * 1048576,
      rarRecoveryPercent: settings.format === 'rar' ? (settings.rarRecoveryPercent ?? 0) : 0,
      rarRecoveryVolumes: settings.format === 'rar' ? (settings.rarRecoveryVolumes ?? 0) : 0,
      encryptFilenames: settings.encryptFilenames, passwordRequired: !!settings.password }
    try {
      phase.value = '步骤 1/2：读取原包'
      unlisten = await listen<{ taskId: string; phase: string }>('archive-conversion-phase', ({ payload }) => {
        if (payload.taskId === id) phase.value = payload.phase === 'writing' ? '步骤 2/2：生成新包' : '步骤 1/2：读取原包'
      })
      item.taskId = id; runningTaskId.value = id
      tasks.addTask({ id, name: '归档转换 · ' + item.sourcePath.split(/[\\/]/).pop(), type: 'compression', workloadKind: 'archive',
        sourceFiles: [item.sourcePath, ...(item.sourceParts || []).filter(path => pathKey(path) !== pathKey(item.sourcePath))], outputPath, format: config.format, executionConfig: config, retryOfTaskId: item.retryOfTaskId })
      tasks.updateTaskStatus(id, 'running')
      try {
        const output = await invoke<PublishedArchiveOutput>('convert_archive', { taskId: id, sourcePath: item.sourcePath,
          ...(config.sourceZipPasswordEncoding ? { sourceZipPasswordEncoding: config.sourceZipPasswordEncoding } : {}),
          sourcePassword: settings.sourcePassword || null, outputPath, ...snapshotArguments(settings.sourceZpaqSnapshot),
          options: { format: config.format, level: config.level, password: settings.password || null,
            zip_method: config.zipMethod ?? 'deflate', zip_encryption: config.zipEncryption, zip_password_encoding: config.zipPasswordEncoding,
            zip_split_mode: config.zipSplitMode, rar_recovery_percent: config.rarRecoveryPercent ?? 0,
            rar_recovery_volumes: config.splitSize > 0 ? (config.rarRecoveryVolumes ?? 0) : 0,
            split_size: config.splitSize || null, encrypt_filenames: config.encryptFilenames,
            preserve_paths: true, delete_after: false, verify_after: true, allow_insecure_password_cli: allowRarPasswordCli } })
        const current = taskFor(item)!
        current.archiveOutput = output; current.outputBytes = output.totalBytes; current.outputBytesEstimated = false
        tasks.updateTaskStatus(id, 'completed')
      } catch (failure) {
        const message = String(failure)
        if (/cancelled|canceled|已取消|操作取消/i.test(message)) {
          stopRequested.value = true; tasks.updateTaskStatus(id, 'cancelled')
        } else { item.error = message; item.expanded = true; tasks.failTask(id, message) }
      }
      await tasks.waitForHistoryPersistence(id)
    } catch (failure) { item.error = String(failure) }
    finally {
      unlisten?.(); runningTaskId.value = ''
      if (item.settings) { item.settings.sourcePassword = ''; item.settings.password = '' }
    }
  }
  async function start() {
    if (busy.value || preparing.value) return
    const batchIds = new Set(selected.value.map(item => item.id))
    preparing.value = true
    try {
      await Promise.all([...batchIds].map(id => sourceResolutions.get(id)))
      // Recheck the group on retry, including volumes added since the failure.
      for (const item of selected.value) if (batchIds.has(item.id)) queueSourceResolution(item)
      await Promise.all([...batchIds].map(id => sourceResolutions.get(id)))
    } finally { preparing.value = false }
    error.value = ''
    const targets = selected.value.filter(item => batchIds.has(item.id))
    if (!targets.length) { error.value = '请选择要转换的压缩包'; return }
    const outputs = new Set<string>()
    for (const item of targets) {
      item.error = validate(item) || undefined
      const key = pathKey(outputFor(item))
      if (outputs.has(key)) item.error = '所选任务的新包保存位置重复，请修改名称或目录'
      outputs.add(key)
      if (item.error) item.expanded = true
    }
    if (targets.some(item => item.error)) { error.value = targets.length === 1 ? targets[0].error! : '请先调整展开条目中的配置，再开始转换。'; return }
    let allowRarPasswordCli = false
    if (targets.some(item => effectiveSettings(item).format === 'rar')) {
      preparing.value = true
      try {
        await refreshRarSupport()
        if (!rarSupport.value?.available) {
          error.value = '未检测到 WinRAR/Rar.exe。请安装 WinRAR 后再开始，或将目标格式改为 ZIP/7Z。'
          return
        }
        if (targets.some(item => effectiveSettings(item).format === 'rar' && effectiveSettings(item).password)) {
          allowRarPasswordCli = await ask('RAR 编码器会在本机进程参数中短暂使用密码。是否继续创建加密 RAR？', { title: '加密 RAR', type: 'warning' })
          if (!allowRarPasswordCli) return
        }
      } catch (failure) { error.value = String(failure); return }
      finally { preparing.value = false }
    }
    const jobs = targets.map(item => {
      const output = outputFor(item), settings = { ...effectiveSettings(item), outputPath: output }
      item.settings = { ...settings }
      return { item, settings, output }
    })
    busy.value = true; stopRequested.value = false
    try {
      await tasks.initListeners()
      for (let index = 0; index < jobs.length; index++) {
        if (stopRequested.value) break
        const job = jobs[index]
        activeId.value = job.item.id
        await execute(job.item, job.settings, job.output, allowRarPasswordCli && job.settings.format === 'rar')
      }
      error.value = targets.length === 1 ? targets[0].error || '' : ''
    } catch (failure) { error.value = String(failure) }
    finally { busy.value = false; defaults.value.sourcePassword = ''; defaults.value.password = '' }
  }
  async function cancel() {
    if (!busy.value || stopRequested.value) return
    stopRequested.value = true
    const id = runningTaskId.value
    const current = tasks.tasks.find(task => task.id === id)
    if (!current || ['completed', 'failed', 'cancelled'].includes(current.status)) return
    try {
      await invoke('cancel_compression', { taskId: id })
      if (!['completed', 'failed', 'cancelled'].includes(current.status)) tasks.updateTaskStatus(id, 'cancelling')
    } catch (failure) { stopRequested.value = false; error.value = '取消请求失败：' + String(failure) }
  }
  return { openSnapshot, visible, items, defaults, pending, selected, activeId, sourcePath, sourcePassword, password, outputPath,
    format, zipEncryption, zipPasswordEncoding, sourceZipPasswordEncoding, level, splitMiB, encryptFilenames, error, busy, task, phase,
    supportsPassword, contextIntent, contextSources, stopRequested, preparing, batchNotice, rarSupport, refreshRarSupport, effectiveSettings, outputFor, taskFor, passwordRequirements,
    addSources, chooseContextSource, selectSource, openFromExplorer, show, restore, start, cancel,
    updateItem, applySelected, updateDefaults, remove, replaceSource }
})
