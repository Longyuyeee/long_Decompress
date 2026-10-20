import { defineStore } from 'pinia'
import type { ZipPasswordEncoding } from '@/types/zipPassword'
import { ref, computed } from 'vue'
import type { BatchConfigPatch } from '@/types/batchConfig'
import type { CompressionFormatId } from '@/utils/compressionFormat'
import type { CompressionAnalysisResult } from '@/composables/useTauriCommands'
import type { VideoCompressionPlan, VideoCompressionSettings } from '@/types/video'
import {
  createDefaultImageSettings,
  inferImageFormat,
  validateImageCandidate,
  type ImageCandidate,
  type ImageCompressionSettings,
} from '@/utils/imageCompressionWorkspace'
import {
  cloneVideoSettings,
  createDefaultVideoSettings,
  validateVideoCandidate,
  type VideoCandidate,
} from '@/utils/videoCompressionWorkspace'

export interface FileObject {
  name: string
  path: string
  size: number
  type: string
  isDirectory: boolean
  expanded?: boolean
  settings?: CompressionOptions
  outputPath?: string
  taskId?: string
  sourceIssue?: string
  draftPasswordRequired?: boolean
}

export interface CompressionOptions {
  /** Legacy compatibility only. New drafts and execution always disable this retired option. */
  fastPack?: boolean
  zipPasswordEncoding?: ZipPasswordEncoding
  wimMethod?: 'store' | 'xpress' | 'lzx' | 'lzms'
  zipMethod?: 'deflate' | 'zstd' | 'bzip2'
  zipEncryption?: 'aes256' | 'zipcrypto'
  zipSplitMode?: 'numeric' | 'standard'
  format: CompressionFormatId
  level: number
  password: string
  filename: string
  splitArchive: boolean
  splitSize: string
  keepStructure: boolean
  deleteAfter: boolean
  verifyAfter: boolean
  createSolidArchive: boolean
  encryptFilenames?: boolean
  rarRecoveryPercent?: number
  rarRecoveryVolumes?: number
}

export interface CompressionGroup {
  id: string
  retryOfTaskId?: string
  name: string
  files: FileObject[]
  themeColor: string
  expanded: boolean
  settings?: CompressionOptions
  outputPath?: string
  taskId?: string
  draftPasswordRequired?: boolean
}

export interface CompressionTask {
  id: string
  name: string
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled'
  progress: number
  startTime?: Date
  endTime?: Date
  error?: string
}

export interface CompressionHistory {
  id: string
  name: string
  timestamp: Date
  status: 'success' | 'error'
  size: number
}

export interface CompressionAnalysisState {
  wimMethod?: 'store' | 'xpress' | 'lzx' | 'lzms'
  zipMethod?: 'deflate' | 'zstd' | 'bzip2'
  status: 'idle' | 'running' | 'completed' | 'failed' | 'cancelled'
  analysisId?: string
  format?: CompressionFormatId
  level?: number
  result?: CompressionAnalysisResult
  actualSize?: number
  predictionErrorPercent?: number
  error?: string
}

export interface ImageCompressionItem {
  id: string
  retryOfTaskId?: string
  name: string
  path: string
  inputSize: number
  inputFormat: string
  width?: number
  height?: number
  previewUrl?: string
  status: 'inspecting' | 'ready' | 'rejected'
  progress: number
  expanded: boolean
  error?: string
  settings?: ImageCompressionSettings
  taskId?: string
  resultPreviewUrl?: string
}

export interface ImageCandidateRejection {
  name: string
  path: string
  reason: string
}

export interface VideoCompressionItem {
  id: string
  retryOfTaskId?: string
  outputDirectory?: string
  name: string
  path: string
  inputSize: number
  status: 'planning' | 'ready' | 'rejected'
  expanded: boolean
  planRevision: number
  plan?: VideoCompressionPlan
  error?: string
  settings?: VideoCompressionSettings
  taskId?: string
}

export interface VideoCandidateRejection {
  name: string
  path: string
  reason: string
}

export const useCompressionStore = defineStore('compression', () => {
  const selectedFiles = ref<FileObject[]>([])
  const groups = ref<CompressionGroup[]>([])
  const globalSettings = ref<CompressionOptions>({
    format: 'zip',
    level: 6,
    password: '',
    filename: '',
    splitArchive: false,
    splitSize: '1024',
    keepStructure: true,
    deleteAfter: false,
    verifyAfter: true,
    createSolidArchive: false
  })
  let archiveDefaultsInitialized = false

  const initializeArchiveDefaults = (format: CompressionOptions['format'], level: number, outputDirectory?: string) => {
    if (archiveDefaultsInitialized) return
    globalSettings.value.format = format
    globalSettings.value.level = Math.max(1, Math.min(9, level))
    if (outputDirectory !== undefined && !globalOutputPath.value) globalOutputPath.value = outputDirectory
    archiveDefaultsInitialized = true
  }
  const globalOutputPath = ref('')
  const restoreArchiveDefaults = (settings: CompressionOptions, outputDirectory: string) => {
    globalSettings.value = cloneSettings(settings)
    globalOutputPath.value = outputDirectory
    archiveDefaultsInitialized = true
  }
  const autoStartRequested = ref(false)
  const imageItems = ref<ImageCompressionItem[]>([])
  const imageGlobalSettings = ref<ImageCompressionSettings>(createDefaultImageSettings())
  const imageLastRejections = ref<ImageCandidateRejection[]>([])
  let nextImageDraftId = 0
  const videoItems = ref<VideoCompressionItem[]>([])
  const videoGlobalSettings = ref<VideoCompressionSettings>(createDefaultVideoSettings())
  const videoOutputDirectory = ref('')
  const videoLastRejections = ref<VideoCandidateRejection[]>([])
  let nextVideoDraftId = 0
  
  const compressionAnalysis = ref<Record<string, CompressionAnalysisState>>({})
  const estimatedSize = computed<Record<string, number>>(() => Object.fromEntries(
    Object.entries(compressionAnalysis.value)
      .filter(([, state]) => state.status === 'completed' && state.result)
      .map(([jobId, state]) => [jobId, state.result!.estimatedSize])
  ))

  const totalOriginalSize = computed(() => {
    return selectedFiles.value.reduce((acc, f) => acc + f.size, 0) + 
           groups.value.reduce((acc, g) => acc + g.files.reduce((ga, f) => ga + f.size, 0), 0)
  })
  const acceptedImageCount = computed(() => imageItems.value.filter(item => item.status !== 'rejected').length)

  const cloneImageSettings = (settings: ImageCompressionSettings): ImageCompressionSettings => ({ ...settings })

  const addImageCandidates = (candidates: ImageCandidate[]) => {
    const occupied = new Set(imageItems.value.map(item => item.path.replace(/\//g, '\\').toLocaleLowerCase()))
    const accepted: ImageCompressionItem[] = []
    const rejected: ImageCandidateRejection[] = []
    for (const candidate of candidates) {
      const result = validateImageCandidate(candidate)
      if (!result.accepted) {
        rejected.push({ name: candidate.name, path: candidate.path, reason: result.reason })
        continue
      }
      const key = candidate.path.replace(/\//g, '\\').toLocaleLowerCase()
      if (occupied.has(key)) continue
      occupied.add(key)
      const item: ImageCompressionItem = {
        id: `image-draft-${Date.now()}-${nextImageDraftId++}`,
        name: candidate.name,
        path: candidate.path,
        inputSize: Math.max(0, candidate.size || 0),
        inputFormat: inferImageFormat(candidate.name),
        status: 'inspecting',
        progress: 0,
        expanded: accepted.length === 0 && imageItems.value.length === 0,
      }
      imageItems.value.push(item)
      accepted.push(item)
    }
    imageLastRejections.value = rejected
    return { accepted, rejected }
  }

  const completeImageInspection = (id: string, details: { width: number, height: number, previewUrl: string }) => {
    const item = imageItems.value.find(candidate => candidate.id === id)
    if (!item) return
    item.width = details.width
    item.height = details.height
    item.previewUrl = details.previewUrl
    item.status = 'ready'
    item.error = undefined
  }

  const failImageInspection = (id: string, reason: string) => {
    const item = imageItems.value.find(candidate => candidate.id === id)
    if (!item) return
    item.status = 'rejected'
    item.error = reason
  }

  const getEffectiveImageSettings = (item: ImageCompressionItem) => item.settings || imageGlobalSettings.value
  const enableImageItemOverride = (id: string) => {
    const item = imageItems.value.find(candidate => candidate.id === id)
    if (item && !item.settings) item.settings = cloneImageSettings(imageGlobalSettings.value)
  }
  const disableImageItemOverride = (id: string) => {
    const item = imageItems.value.find(candidate => candidate.id === id)
    if (item) item.settings = undefined
  }
  const updateImageItemSettings = (id: string, settings: ImageCompressionSettings) => {
    const item = imageItems.value.find(candidate => candidate.id === id)
    if (item) item.settings = cloneImageSettings(settings)
  }
  const bindImageItemTask = (id: string, taskId: string) => {
    const item = imageItems.value.find(candidate => candidate.id === id)
    if (!item) return
    item.taskId = taskId
    item.resultPreviewUrl = undefined
  }
  const setImageResultPreview = (id: string, previewUrl: string) => {
    const item = imageItems.value.find(candidate => candidate.id === id)
    if (item) item.resultPreviewUrl = previewUrl
  }
  const removeImageItem = (id: string) => {
    imageItems.value = imageItems.value.filter(item => item.id !== id)
  }
  const clearImageDrafts = () => {
    imageItems.value = []
    imageLastRejections.value = []
  }

  const addVideoCandidates = (candidates: VideoCandidate[]) => {
    const occupied = new Set(videoItems.value.map(item => item.path.replace(/\//g, '\\').toLocaleLowerCase()))
    const accepted: VideoCompressionItem[] = []
    const rejected: VideoCandidateRejection[] = []
    for (const candidate of candidates) {
      const result = validateVideoCandidate(candidate)
      if (!result.accepted) {
        rejected.push({ name: candidate.name, path: candidate.path, reason: result.reason })
        continue
      }
      const key = candidate.path.replace(/\//g, '\\').toLocaleLowerCase()
      if (occupied.has(key)) continue
      occupied.add(key)
      const item: VideoCompressionItem = {
        id: `video-draft-${Date.now()}-${nextVideoDraftId++}`,
        name: candidate.name,
        path: candidate.path,
        inputSize: Math.max(0, candidate.size || 0),
        status: 'planning',
        expanded: false,
        planRevision: 1,
      }
      videoItems.value.push(item)
      accepted.push(item)
    }
    videoLastRejections.value = rejected
    return { accepted, rejected }
  }

  const queueVideoPlanning = (item: VideoCompressionItem) => {
    item.planRevision += 1
    item.status = 'planning'
    item.error = undefined
  }

  const completeVideoPlanning = (id: string, revision: number, plan: VideoCompressionPlan) => {
    const item = videoItems.value.find(candidate => candidate.id === id)
    if (!item || item.planRevision !== revision) return false
    item.plan = plan
    item.status = 'ready'
    item.error = undefined
    return true
  }

  const failVideoPlanning = (id: string, revision: number, reason: string) => {
    const item = videoItems.value.find(candidate => candidate.id === id)
    if (!item || item.planRevision !== revision) return false
    // Keep the last verified probe facts visible when only the derived plan
    // fails to refresh. A first-time probe still has no plan to display.
    item.status = 'rejected'
    item.error = reason
    return true
  }

  const getEffectiveVideoSettings = (item: VideoCompressionItem) => item.settings || videoGlobalSettings.value
  const updateVideoGlobalSettings = (settings: VideoCompressionSettings) => {
    videoGlobalSettings.value = cloneVideoSettings(settings)
    videoItems.value.filter(item => !item.settings).forEach(queueVideoPlanning)
  }
  const enableVideoItemOverride = (id: string) => {
    const item = videoItems.value.find(candidate => candidate.id === id)
    if (!item || item.settings) return
    item.settings = cloneVideoSettings(videoGlobalSettings.value)
  }
  const disableVideoItemOverride = (id: string) => {
    const item = videoItems.value.find(candidate => candidate.id === id)
    if (!item || !item.settings) return
    item.settings = undefined
    queueVideoPlanning(item)
  }
  const updateVideoItemSettings = (id: string, settings: VideoCompressionSettings) => {
    const item = videoItems.value.find(candidate => candidate.id === id)
    if (!item) return
    item.settings = cloneVideoSettings(settings)
    queueVideoPlanning(item)
  }
  const updateVideoItemSettingsDraft = (id: string, settings: VideoCompressionSettings) => {
    const item = videoItems.value.find(candidate => candidate.id === id)
    if (item) item.settings = cloneVideoSettings(settings)
  }
  const retryVideoPlanning = (id: string) => {
    const item = videoItems.value.find(candidate => candidate.id === id)
    if (item) queueVideoPlanning(item)
  }
  const bindVideoItemTask = (id: string, taskId: string) => {
    const item = videoItems.value.find(candidate => candidate.id === id)
    if (item) item.taskId = taskId
  }
  const removeVideoItem = (id: string) => {
    videoItems.value = videoItems.value.filter(item => item.id !== id)
  }
  const clearVideoDrafts = () => {
    videoItems.value = []
    videoLastRejections.value = []
  }

  // 磁吸打组逻辑
  const cloneSettings = (settings: CompressionOptions): CompressionOptions => ({ ...settings, ...('fastPack' in settings ? { fastPack: false } : {}) })

  const getEffectiveSettings = (settings?: CompressionOptions): CompressionOptions => {
    return settings || globalSettings.value
  }

  const getEffectiveOutputPath = (outputPath?: string): string => {
    return outputPath ?? globalOutputPath.value
  }

  const addFile = (file: FileObject) => {
    const alreadySelected = selectedFiles.value.some(existing => existing.path === file.path) ||
      groups.value.some(group => group.files.some(existing => existing.path === file.path))
    if (alreadySelected) return false
    selectedFiles.value.push({
      ...file,
      expanded: false
    })
    return true
  }

  const updateFileSettings = (path: string, settings: CompressionOptions) => {
    const file = selectedFiles.value.find(item => item.path === path)
    if (file) {
      file.outputPath ??= globalOutputPath.value
      file.settings = cloneSettings(settings)
      if (settings.format === 'lz' || settings.format === 'tar.lz' || settings.format === 'tar.lzma') file.draftPasswordRequired = false
    }
  }

  const useGlobalFileSettings = (path: string) => {
    const file = selectedFiles.value.find(item => item.path === path)
    if (file) { file.settings = undefined; file.outputPath = undefined }
  }

  const updateFileOutputPath = (path: string, outputPath: string) => {
    const file = selectedFiles.value.find(item => item.path === path)
    if (file) {
      file.settings ??= cloneSettings(globalSettings.value)
      file.outputPath = outputPath
    }
  }

  const updateGroupSettings = (groupId: string, settings: CompressionOptions) => {
    const group = groups.value.find(item => item.id === groupId)
    if (group) {
      group.outputPath ??= globalOutputPath.value
      group.settings = cloneSettings(settings)
      if (settings.format === 'lz' || settings.format === 'tar.lz' || settings.format === 'tar.lzma') group.draftPasswordRequired = false
    }
  }

  const useGlobalGroupSettings = (groupId: string) => {
    const group = groups.value.find(item => item.id === groupId)
    if (group) { group.settings = undefined; group.outputPath = undefined }
  }

  const updateGroupOutputPath = (groupId: string, outputPath: string) => {
    const group = groups.value.find(item => item.id === groupId)
    if (group) {
      group.settings ??= cloneSettings(globalSettings.value)
      group.outputPath = outputPath
    }
  }

  const applyBatchConfiguration = (ids: string[], patch: BatchConfigPatch) => {
    const selected = new Set(ids)
    if (patch.wimMethod === undefined && patch.zipMethod === undefined && patch.zipPasswordEncoding === undefined && patch.format === undefined && patch.level === undefined && patch.rarRecoveryPercent === undefined && patch.rarRecoveryVolumes === undefined && patch.outputDirectory === undefined) return 0
    if (patch.zipPasswordEncoding === 'auto') throw new Error('创建新包请选择明确的密码编码，默认使用 UTF-8。')
    let count = 0
    const drafts = [...selectedFiles.value.map(file => ({ id: file.path, draft: file })),
      ...groups.value.map(group => ({ id: group.id, draft: group }))]
    if (patch.format === 'wim' && drafts.some(({id,draft}) => selected.has(id) && !draft.taskId && (getEffectiveSettings(draft.settings).password || draft.draftPasswordRequired))) throw new Error('WIM 不支持原生密码，所选配置未修改。请保持密码格式，或先在单条配置中明确移除密码。')
    const wimOnly = patch.wimMethod !== undefined && Object.entries(patch).every(([key,v]) => key === 'wimMethod' || v === undefined)
    if (wimOnly && !drafts.some(({id,draft}) => selected.has(id) && !draft.taskId && getEffectiveSettings(draft.settings).format === 'wim')) throw new Error('所选任务中没有 WIM 任务，请选择 WIM 格式或保持各自方法。')
    const methodOnly = patch.zipMethod !== undefined && Object.entries(patch).every(([field, value]) => field === 'zipMethod' || value === undefined)
    const formatOptionsOnly = Object.entries(patch).every(([field,value]) => ['wimMethod','zipMethod','zipPasswordEncoding'].includes(field) || value === undefined)
    const zipOptionsOnly = Object.entries(patch).every(([field,value]) => ['zipMethod','zipPasswordEncoding'].includes(field) || value === undefined)
    if (methodOnly && !drafts.some(({ id, draft }) => selected.has(id) && !draft.taskId && getEffectiveSettings(draft.settings).format === 'zip')) throw new Error('所选任务中没有 ZIP 任务，请选择 ZIP 格式或保持各自方法。')
    if (zipOptionsOnly && !drafts.some(({id,draft}) => selected.has(id) && !draft.taskId && ((patch.zipMethod !== undefined && getEffectiveSettings(draft.settings).format === 'zip') || (patch.zipPasswordEncoding !== undefined && ['zip','zipx'].includes(getEffectiveSettings(draft.settings).format))))) throw new Error('所选任务中没有适用的 ZIP/ZIPX 设置。')
    if (patch.rarRecoveryPercent && drafts.some(({ id, draft }) => selected.has(id) && !draft.taskId && (patch.format || getEffectiveSettings(draft.settings).format) !== 'rar')) throw new Error('恢复保护只适用于 RAR，请先将所选任务设为 RAR，或保持各自恢复设置。')
    if (patch.rarRecoveryVolumes && drafts.some(({ id, draft }) => selected.has(id) && !draft.taskId && (patch.format || getEffectiveSettings(draft.settings).format) !== 'rar')) throw new Error('恢复卷只适用于 RAR 分包，请先调整目标格式。')
    if ((patch.format === 'zip' || patch.format === 'zipx') && drafts.some(({ id, draft }) => selected.has(id) && !draft.taskId && getEffectiveSettings(draft.settings).encryptFilenames)) {
      throw new Error('所选任务中有已开启文件名加密的 7Z 任务。请保持 7Z 格式，或先在该条配置中关闭文件名加密。')
    }
    if (['lzma', 'lzma.aes'].includes(patch.format || '') && drafts.some(({id,draft}) => {
      if (!selected.has(id) || draft.taskId) return false
      const files = 'files' in draft ? draft.files : [draft]
      return files.length !== 1 || files[0].isDirectory
    })) throw new Error('LZMA / LZMA.AES 每条任务只能压缩一个普通文件；目录或多个文件请选择 TAR.LZMA / TAR.LZMA.AES。所选配置未修改。')
    for (const { id, draft } of drafts) {
      if (!selected.has(id) || draft.taskId || (methodOnly && getEffectiveSettings(draft.settings).format !== 'zip')) continue
      if (wimOnly && getEffectiveSettings(draft.settings).format !== 'wim') continue
      if (zipOptionsOnly && !((patch.zipMethod !== undefined && getEffectiveSettings(draft.settings).format === 'zip') || (patch.zipPasswordEncoding !== undefined && ['zip','zipx'].includes(getEffectiveSettings(draft.settings).format)))) continue
      const currentFormat = getEffectiveSettings(draft.settings).format
      if (formatOptionsOnly && !((patch.wimMethod !== undefined && currentFormat === 'wim') || (patch.zipMethod !== undefined && currentFormat === 'zip') || (patch.zipPasswordEncoding !== undefined && ['zip','zipx'].includes(currentFormat)))) continue
      const settings = cloneSettings(getEffectiveSettings(draft.settings))
      if (patch.format !== undefined) settings.format = patch.format
      if (patch.format === 'tar.lz' || patch.format === 'lzma' || patch.format === 'tar.lzma') {
        draft.draftPasswordRequired = false
        settings.password = ''
        settings.encryptFilenames = false
        settings.createSolidArchive = false
        settings.rarRecoveryPercent = 0
        settings.rarRecoveryVolumes = 0
      }
      if (patch.format === 'wim' || patch.format === 'zpaq' || patch.format === 'lzma.aes' || patch.format === 'tar.lzma.aes') {
        settings.encryptFilenames = false
        settings.createSolidArchive = false
        settings.rarRecoveryPercent = 0
        settings.rarRecoveryVolumes = 0
      }
      if (patch.wimMethod !== undefined && settings.format === 'wim') settings.wimMethod = patch.wimMethod
      if (patch.zipMethod !== undefined && settings.format === 'zip') settings.zipMethod = patch.zipMethod
      if (patch.zipPasswordEncoding !== undefined && ['zip','zipx'].includes(settings.format)) settings.zipPasswordEncoding = patch.zipPasswordEncoding
      if (patch.level !== undefined) settings.level = patch.level
      if (patch.rarRecoveryPercent !== undefined) settings.rarRecoveryPercent = patch.rarRecoveryPercent
      if (patch.rarRecoveryVolumes !== undefined) settings.rarRecoveryVolumes = patch.rarRecoveryVolumes
      draft.outputPath = patch.outputDirectory ?? getEffectiveOutputPath(draft.outputPath)
      draft.settings = settings
      if (patch.format !== undefined || patch.level !== undefined || patch.zipMethod !== undefined || patch.wimMethod !== undefined) clearAnalysis(id)
      count++
    }
    return count
  }

  const setAnalysisState = (jobId: string, state: CompressionAnalysisState) => {
    compressionAnalysis.value[jobId] = state
  }

  const clearAnalysis = (jobId: string) => {
    delete compressionAnalysis.value[jobId]
  }

  const recordActualSize = (jobId: string, actualSize: number) => {
    const state = compressionAnalysis.value[jobId]
    if (!state?.result || actualSize < 0) return
    const denominator = Math.max(actualSize, 1)
    state.actualSize = actualSize
    state.predictionErrorPercent = Math.round(
      Math.abs(state.result.estimatedSize - actualSize) / denominator * 100
    )
  }

  const bindJobTask = (
    jobId: string,
    taskId: string,
    settings: CompressionOptions,
    outputPath: string,
  ) => {
    const group = groups.value.find(item => item.id === jobId)
    if (group) {
      group.taskId = taskId
      group.settings = cloneSettings(settings)
      group.outputPath = outputPath
      return
    }

    const file = selectedFiles.value.find(item => item.path === jobId)
    if (file) {
      file.taskId = taskId
      file.settings = cloneSettings(settings)
      file.outputPath = outputPath
    }
  }

  const createGroup = (paths: string[]) => {
    const id = Date.now().toString()
    const colors = ['#3b82f6', '#8b5cf6', '#ec4899', '#10b981', '#f59e0b']
    const themeColor = colors[groups.value.length % colors.length]
    
    // 找到对应的 FileObject
    const targetFiles = selectedFiles.value.filter(f => paths.includes(f.path))
    targetFiles.forEach(file => clearAnalysis(file.path))
    
    groups.value.push({
      id,
      name: `新建压缩组 ${groups.value.length + 1}`,
      files: [...targetFiles],
      themeColor,
      expanded: true
    })
    
    // 从未分组列表中移除
    selectedFiles.value = selectedFiles.value.filter(f => !paths.includes(f.path))
    return id
  }

  const prepareQuickPacks = () => {
    selectedFiles.value = []
    groups.value = []
    compressionAnalysis.value = {}
  }

  const addQuickPack = (files: FileObject[], name: string, outputPath: string) => {
    groups.value.push({
      id: `quick-pack-${Date.now()}-${groups.value.length}`,
      name,
      files: files.map(file => ({ ...file, expanded: false })),
      themeColor: '#3b82f6',
      expanded: true,
      outputPath,
      settings: {
        ...cloneSettings(globalSettings.value),
        format: 'zip',
        filename: name,
      },
    })
    globalSettings.value.format = 'zip'
    autoStartRequested.value = true
  }

  const replaceWithQuickPack = (files: FileObject[], name: string, outputPath: string) => {
    prepareQuickPacks()
    addQuickPack(files, name, outputPath)
  }

  const dissolveGroup = (groupId: string) => {
    const index = groups.value.findIndex(g => g.id === groupId)
    if (index !== -1) {
      const group = groups.value[index]
      selectedFiles.value.push(...group.files)
      groups.value.splice(index, 1)
      clearAnalysis(groupId)
    }
  }

  const removeFileFromGroup = (groupId: string, filePath: string) => {
    const group = groups.value.find(g => g.id === groupId)
    if (!group) return
    group.files = group.files.filter(f => f.path !== filePath)
    clearAnalysis(groupId)
    // 如果组内没有文件了，自动解散
    if (group.files.length === 0) {
      groups.value = groups.value.filter(g => g.id !== groupId)
    }
  }

  const addTemplateDraft = (
    files: FileObject[],
    name: string,
    settings: CompressionOptions,
  ) => {
    const occupied = new Set([
      ...selectedFiles.value.map(file => file.path.replace(/\//g, '\\').toLowerCase()),
      ...groups.value.flatMap(group => group.files.map(file => file.path.replace(/\//g, '\\').toLowerCase())),
    ])
    const unique = new Set<string>()
    const accepted = files.filter(file => {
      const key = file.path.replace(/\//g, '\\').toLowerCase()
      if (!file.path || occupied.has(key) || unique.has(key)) return false
      unique.add(key)
      return true
    })
    if (accepted.length === 0) return null

    const id = `template-draft-${Date.now()}-${groups.value.length}`
    groups.value.push({
      id,
      name,
      files: accepted.map(file => ({ ...file, expanded: false })),
      themeColor: '#6366f1',
      expanded: true,
      settings: cloneSettings(settings),
    })
    // A template draft must never inherit a pending Explorer quick-action auto start.
    autoStartRequested.value = false
    return { id, addedCount: accepted.length, skippedCount: files.length - accepted.length }
  }

  const addHistoryDraft = (files: FileObject[], name: string, settings: CompressionOptions, outputPath: string, retryOfTaskId: string) => {
    const existing = groups.value.find(group => group.retryOfTaskId === retryOfTaskId)
    if (existing) return existing.id
    if (!files.length) throw new Error('请至少选择一个来源')
    const id = `history-${globalThis.crypto.randomUUID()}`
    groups.value.push({ id, retryOfTaskId, name, files: files.map(file => ({ ...file })),
      themeColor: '#6366f1', expanded: true, settings: cloneSettings(settings), outputPath })
    autoStartRequested.value = false
    return id
  }

  const removeFile = (path: string) => {
    selectedFiles.value = selectedFiles.value.filter(file => file.path !== path)
    clearAnalysis(path)
  }

  const removeJobsByTaskIds = (taskIds: string[]) => {
    const removed = new Set(taskIds)
    groups.value.filter(group => group.taskId && removed.has(group.taskId)).forEach(group => clearAnalysis(group.id))
    selectedFiles.value.filter(file => file.taskId && removed.has(file.taskId)).forEach(file => clearAnalysis(file.path))
    groups.value = groups.value.filter(group => !group.taskId || !removed.has(group.taskId))
    selectedFiles.value = selectedFiles.value.filter(file => !file.taskId || !removed.has(file.taskId))
  }

  const requestAutoStart = () => {
    autoStartRequested.value = true
  }

  const consumeAutoStart = () => {
    const requested = autoStartRequested.value
    autoStartRequested.value = false
    return requested
  }

  return {
    selectedFiles,
    groups,
    globalSettings,
    initializeArchiveDefaults,
    restoreArchiveDefaults,
    globalOutputPath,
    autoStartRequested,
    imageItems,
    imageGlobalSettings,
    imageLastRejections,
    videoItems,
    videoGlobalSettings,
    videoOutputDirectory,
    videoLastRejections,
    acceptedImageCount,
    estimatedSize,
    compressionAnalysis,
    totalOriginalSize,
    addImageCandidates,
    completeImageInspection,
    failImageInspection,
    getEffectiveImageSettings,
    enableImageItemOverride,
    disableImageItemOverride,
    updateImageItemSettings,
    bindImageItemTask,
    setImageResultPreview,
    removeImageItem,
    clearImageDrafts,
    addVideoCandidates,
    completeVideoPlanning,
    failVideoPlanning,
    getEffectiveVideoSettings,
    updateVideoGlobalSettings,
    enableVideoItemOverride,
    updateVideoItemSettingsDraft,
    disableVideoItemOverride,
    updateVideoItemSettings,
    retryVideoPlanning,
    bindVideoItemTask,
    removeVideoItem,
    clearVideoDrafts,
    cloneSettings,
    applyBatchConfiguration,
    getEffectiveSettings,
    getEffectiveOutputPath,
    addFile,
    updateFileSettings,
    useGlobalFileSettings,
    updateFileOutputPath,
    updateGroupSettings,
    useGlobalGroupSettings,
    updateGroupOutputPath,
    setAnalysisState,
    clearAnalysis,
    recordActualSize,
    bindJobTask,
    createGroup,
    prepareQuickPacks,
    addQuickPack,
    addTemplateDraft,
    addHistoryDraft,
    replaceWithQuickPack,
    dissolveGroup,
    removeFileFromGroup,
    removeFile,
    removeJobsByTaskIds,
    requestAutoStart,
    consumeAutoStart
  }
})
