import { snapshotKey } from '@/types/zpaqSnapshot'
import { defineStore } from 'pinia'
import { computed, ref, watch, type WatchStopHandle } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useCompressionStore, type CompressionOptions, type FileObject, type CompressionGroup } from './compression'
import { useTaskStore } from './task'
import { useArchiveConversionStore } from './archiveConversion'
import { useDecompressionPreferences } from './decompressionPreferences'
import type { ConversionDraft, ConversionSettings } from '@/types/archiveConversion'
import { archiveGroupKey } from '@/utils/archiveSource'
import type { ZipPasswordEncoding } from '@/types/zipPassword'

export const PENDING_DRAFTS_KEY = 'long-pending-archive-drafts-v1'
interface ExtractionDraft {
  zpaqSnapshot?: import('@/types/zpaqSnapshot').ZpaqSnapshot
  zipPasswordEncoding?: ZipPasswordEncoding
  id: string; name: string; sourceFiles: string[]; outputPath: string
  extractToSubfolder?: boolean; recycleSourceAfterExtract?: boolean
  configurationMode?: 'global' | 'individual'; fileFilter?: string; selectedEntries?: string[]
  retryOfTaskId?: string; draftPasswordRequired?: boolean
}
interface DraftSnapshot {
  version: 1; savedAt: string
  compression: { files: FileObject[]; groups: CompressionGroup[]; defaults: CompressionOptions; outputDirectory: string }
  extraction: { items: ExtractionDraft[]; defaults: { outputPath: string; sameDirectory: boolean; extractToSubfolder: boolean; recycleSource: boolean } }
  conversion: { items: ConversionDraft[]; defaults: ConversionSettings }
}
const key = (path: string) => path.replace(/\\/g, '/').toLowerCase()
const countSnapshot = (snapshot: DraftSnapshot | null) => snapshot
  ? snapshot.compression.files.length + snapshot.compression.groups.length + snapshot.extraction.items.length + snapshot.conversion.items.length : 0
const withoutPasswords = <T extends { password: string }>(settings: T): T => ({ ...settings, password: '' })
const cleanConversion = (settings: ConversionSettings): ConversionSettings => ({ ...settings, password: '', sourcePassword: '' })
function parseSnapshot(raw: string | null): DraftSnapshot | null {
  if (!raw) return null
  try {
    const value = JSON.parse(raw) as DraftSnapshot
    if (value.version !== 1 || !Array.isArray(value.compression?.files) || !Array.isArray(value.compression?.groups)
      || !Array.isArray(value.extraction?.items) || !Array.isArray(value.conversion?.items)
      || !value.compression.defaults || !value.extraction.defaults || !value.conversion.defaults) return null
    if (!value.compression.files.every(file => typeof file.path === 'string')
      || !value.compression.groups.every(group => Array.isArray(group.files) && group.files.every(file => typeof file.path === 'string'))
      || !value.extraction.items.every(item => Array.isArray(item.sourceFiles) && item.sourceFiles.every(path => typeof path === 'string'))
      || !value.conversion.items.every(item => typeof item.sourcePath === 'string')) return null
    return value
  } catch { return null }
}
function merge<T>(older: T[], current: T[], identify: (item: T) => string): T[] {
  return [...new Map([...older, ...current].map(item => [identify(item), item])).values()]
}

export const usePendingDraftStore = defineStore('pendingDrafts', () => {
  const compression = useCompressionStore(), tasks = useTaskStore(), conversion = useArchiveConversionStore()
  const extraction = useDecompressionPreferences()
  const recovery = ref<DraftSnapshot | null>(null)
  const busy = ref(false), feedback = ref(''), saveError = ref('')
  const count = computed(() => countSnapshot(recovery.value))
  let initialized = false, stop: WatchStopHandle | undefined, timer: ReturnType<typeof setTimeout> | undefined
  function snapshot(): DraftSnapshot {
    const cleanFile = (file: FileObject): FileObject => ({ name: file.name, path: file.path, size: file.size,
      type: file.type, isDirectory: file.isDirectory, expanded: file.expanded, outputPath: file.outputPath,
      settings: file.settings ? withoutPasswords(file.settings) : undefined,
      draftPasswordRequired: file.draftPasswordRequired || !!compression.getEffectiveSettings(file.settings).password })
    return {
      version: 1, savedAt: new Date().toISOString(),
      compression: { defaults: withoutPasswords(compression.globalSettings), outputDirectory: compression.globalOutputPath,
        files: compression.selectedFiles.filter(file => !file.taskId).map(cleanFile),
        groups: compression.groups.filter(group => !group.taskId).map(group => ({ id: group.id, name: group.name,
          themeColor: group.themeColor, expanded: group.expanded, retryOfTaskId: group.retryOfTaskId,
          files: group.files.map(cleanFile), outputPath: group.outputPath,
          settings: group.settings ? withoutPasswords(group.settings) : undefined,
          draftPasswordRequired: group.draftPasswordRequired || !!compression.getEffectiveSettings(group.settings).password })) },
      extraction: { defaults: { outputPath: extraction.outputPath, sameDirectory: extraction.sameDirectory,
        extractToSubfolder: extraction.extractToSubfolder, recycleSource: extraction.recycleSource },
        items: tasks.tasks.filter(task => task.type === 'decompression' && (!task.workloadKind || task.workloadKind === 'archive') && task.status === 'pending')
          .map(task => ({ id: task.id, name: task.name, sourceFiles: [...task.sourceFiles], outputPath: task.outputPath,
            extractToSubfolder: task.extractToSubfolder, recycleSourceAfterExtract: task.recycleSourceAfterExtract,
            configurationMode: task.configurationMode, fileFilter: task.fileFilter, selectedEntries: task.selectedEntries,
            zipPasswordEncoding: task.zipPasswordEncoding, zpaqSnapshot: task.zpaqSnapshot,
            retryOfTaskId: task.retryOfTaskId, draftPasswordRequired: task.draftPasswordRequired || !!task.password })) },
      conversion: { defaults: cleanConversion(conversion.defaults), items: conversion.pending.map(item => ({
        id: item.id, sourcePath: item.sourcePath, sourceParts: item.sourceParts, sourceReference: item.sourceReference, intent: item.intent, defaultFormat: item.defaultFormat, expanded: item.expanded, selected: item.selected,
        settings: item.settings ? cleanConversion(item.settings) : undefined, retryOfTaskId: item.retryOfTaskId,
        sourcePasswordRequired: conversion.passwordRequirements(item).source || !!conversion.effectiveSettings(item).sourcePassword,
        targetPasswordRequired: conversion.passwordRequirements(item).target || !!conversion.effectiveSettings(item).password,
        passwordRequired: item.passwordRequired || !!conversion.effectiveSettings(item).password || !!conversion.effectiveSettings(item).sourcePassword })) },
    }
  }
  function flush() {
    if (!initialized) return
    if (timer) clearTimeout(timer)
    timer = undefined
    const current = snapshot(), older = recovery.value
    if (older) {
      const hasCompression = !!(current.compression.files.length || current.compression.groups.length)
      const hasExtraction = !!current.extraction.items.length, hasConversion = !!current.conversion.items.length
      if (!current.compression.files.length && !current.compression.groups.length) {
        current.compression.defaults = older.compression.defaults; current.compression.outputDirectory = older.compression.outputDirectory
      }
      if (!current.extraction.items.length) current.extraction.defaults = older.extraction.defaults
      if (!current.conversion.items.length) current.conversion.defaults = older.conversion.defaults
      // Preserve the unopened recovery offer while the user adds new work in this session.
      current.compression.files = merge(older.compression.files, current.compression.files, file => key(file.path))
      current.compression.groups = merge(older.compression.groups, current.compression.groups, group => group.id)
      current.extraction.items = merge(older.extraction.items, current.extraction.items, item => item.id)
      current.conversion.items = merge(older.conversion.items, current.conversion.items, item => item.id)
      // Resolve inherited settings against the old defaults before mixing two sessions.
      for (const file of current.compression.files) if (hasCompression && older.compression.files.includes(file) && !file.settings) { file.settings = { ...older.compression.defaults }; file.outputPath ??= older.compression.outputDirectory }
      for (const group of current.compression.groups) if (hasCompression && older.compression.groups.includes(group) && !group.settings) { group.settings = { ...older.compression.defaults }; group.outputPath ??= older.compression.outputDirectory }
      for (const item of current.conversion.items) if (hasConversion && older.conversion.items.includes(item) && !item.settings) item.settings = {
        ...older.conversion.defaults, format: item.defaultFormat || older.conversion.defaults.format }
      for (const item of current.extraction.items) if (hasExtraction && older.extraction.items.includes(item)) item.configurationMode = 'individual'
    }
    try {
      if (countSnapshot(current)) localStorage.setItem(PENDING_DRAFTS_KEY, JSON.stringify(current))
      else localStorage.removeItem(PENDING_DRAFTS_KEY)
      saveError.value = ''
    } catch { saveError.value = '待处理任务暂时无法保存，请保持软件打开。' }
  }
  function initialize() {
    if (initialized) return
    initialized = true
    try { recovery.value = parseSnapshot(localStorage.getItem(PENDING_DRAFTS_KEY)) }
    catch { saveError.value = '暂时无法读取上次的待处理任务。' }
    stop = watch(() => JSON.stringify(snapshot(), (name, value) => name === 'savedAt' ? undefined : value), () => {
      if (timer) clearTimeout(timer)
      timer = setTimeout(flush, 200)
    })
    window.addEventListener('beforeunload', flush)
    window.addEventListener('pagehide', flush)
  }
  function discard() { if (busy.value) return; recovery.value = null; feedback.value = ''; flush() }
  async function restore() {
    if (busy.value || !recovery.value) return
    busy.value = true; feedback.value = ''
    const saved = recovery.value
    try {
      const paths = [...new Set([...saved.compression.files.map(file => file.path),
        ...saved.compression.groups.flatMap(group => group.files.map(file => file.path)),
        ...saved.extraction.items.flatMap(item => item.sourceFiles), ...saved.conversion.items.map(item => item.sourcePath)])]
      const missing = new Set<string>()
      let next = 0
      await Promise.all(Array.from({ length: Math.min(4, paths.length) }, async () => {
        while (next < paths.length) {
          const path = paths[next++]
          try { if (!await invoke('get_file_info', { path })) missing.add(path) } catch { missing.add(path) }
        }
      }))
      const hadCompression = compression.selectedFiles.length + compression.groups.length > 0
      const hadExtraction = tasks.tasks.some(task => task.type === 'decompression' && task.status === 'pending')
      const hadConversion = conversion.pending.length > 0
      if (!hadCompression) compression.restoreArchiveDefaults({ ...saved.compression.defaults, password: '' }, saved.compression.outputDirectory)
      if (!hadExtraction) Object.assign(extraction, saved.extraction.defaults)
      if (!hadConversion) conversion.defaults = cleanConversion(saved.conversion.defaults)
      let restored = 0
      const occupied = new Set([...compression.selectedFiles.map(file => key(file.path)), ...compression.groups.flatMap(group => group.files.map(file => key(file.path)))])
      const hydrateFile = (file: FileObject): FileObject => ({ ...file, taskId: undefined,
        settings: file.settings ? withoutPasswords(file.settings) : hadCompression ? { ...saved.compression.defaults, password: '' } : undefined,
        outputPath: file.outputPath ?? (hadCompression ? saved.compression.outputDirectory : undefined),
        sourceIssue: missing.has(file.path) ? '来源不存在或无法访问' : undefined })
      for (const file of saved.compression.files) {
        if (occupied.has(key(file.path))) continue
        compression.selectedFiles.push(hydrateFile(file)); occupied.add(key(file.path)); restored++
      }
      for (const group of saved.compression.groups) {
        if (!group.files.length || compression.groups.some(current => current.name === group.name
          && current.files.map(file => key(file.path)).sort().join('|') === group.files.map(file => key(file.path)).sort().join('|'))) continue
        compression.groups.push({ ...group, id: 'recovered-' + crypto.randomUUID(), taskId: undefined,
          files: group.files.map(hydrateFile), expanded: group.expanded || group.files.some(file => missing.has(file.path)),
          settings: group.settings ? withoutPasswords(group.settings) : hadCompression ? { ...saved.compression.defaults, password: '' } : undefined,
          outputPath: group.outputPath ?? (hadCompression ? saved.compression.outputDirectory : undefined) })
        group.files.forEach(file => occupied.add(key(file.path))); restored++
      }
      const extractSources = new Set(tasks.tasks.filter(task => task.type === 'decompression' && task.status === 'pending').map(task => task.sourceFiles.map(key).join('|') + ':' + snapshotKey(task.zpaqSnapshot)))
      for (const item of saved.extraction.items) {
        const sourceKey = item.sourceFiles.map(key).join('|') + ':' + snapshotKey(item.zpaqSnapshot)
        if (extractSources.has(sourceKey)) continue
        tasks.addTask({ ...item, id: 'recovered-' + crypto.randomUUID(), type: 'decompression',
          configurationMode: hadExtraction ? 'individual' : item.configurationMode,
          sourceIssue: item.sourceFiles.some(path => missing.has(path)) ? '来源不存在或无法访问，请重新选择。' : undefined })
        extractSources.add(sourceKey); restored++
      }
      for (const item of saved.conversion.items) {
        if (conversion.pending.some(current => archiveGroupKey(current.sourcePath) === archiveGroupKey(item.sourcePath) && snapshotKey(conversion.effectiveSettings(current).sourceZpaqSnapshot) === snapshotKey((item.settings || saved.conversion.defaults).sourceZpaqSnapshot))) continue
        conversion.items.push({ ...item, id: 'recovered-' + crypto.randomUUID(), taskId: undefined,
          settings: item.settings ? cleanConversion(item.settings) : hadConversion ? { ...cleanConversion(saved.conversion.defaults), format: item.defaultFormat || saved.conversion.defaults.format } : undefined,
          sourceIssue: missing.has(item.sourcePath) ? '来源不存在或无法访问，请重新选择。' : undefined })
        restored++
      }
      // Never restore the Explorer auto-start flag or resume an operation.
      compression.autoStartRequested = false
      recovery.value = null
      feedback.value = '已恢复 ' + restored + ' 项待处理任务' + (missing.size ? '，有 ' + missing.size + ' 个来源需要重新选择。' : '，可核对后开始。')
      flush()
    } catch { feedback.value = '恢复未完成，请重试；上次草稿仍保留。' }
    finally { busy.value = false }
  }
  function dispose() {
    flush(); stop?.(); stop = undefined; initialized = false
    window.removeEventListener('beforeunload', flush); window.removeEventListener('pagehide', flush)
  }
  return { count, recovery, busy, feedback, saveError, initialize, flush, discard, restore, dispose }
})
