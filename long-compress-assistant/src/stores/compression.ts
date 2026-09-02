import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
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
}

export interface CompressionOptions {
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
}

export interface CompressionGroup {
  id: string
  name: string
  files: FileObject[]
  themeColor: string
  expanded: boolean
  settings?: CompressionOptions
  outputPath?: string
  taskId?: string
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
  const globalOutputPath = ref('')
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
        expanded: accepted.length === 0 && videoItems.value.length === 0,
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
  const cloneSettings = (settings: CompressionOptions): CompressionOptions => ({ ...settings })

  const getEffectiveSettings = (settings?: CompressionOptions): CompressionOptions => {
    return settings || globalSettings.value
  }

  const getEffectiveOutputPath = (outputPath?: string): string => {
    return outputPath || globalOutputPath.value
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
    if (file) file.settings = cloneSettings(settings)
  }

  const updateFileOutputPath = (path: string, outputPath: string) => {
    const file = selectedFiles.value.find(item => item.path === path)
    if (file) file.outputPath = outputPath
  }

  const updateGroupSettings = (groupId: string, settings: CompressionOptions) => {
    const group = groups.value.find(item => item.id === groupId)
    if (group) group.settings = cloneSettings(settings)
  }

  const updateGroupOutputPath = (groupId: string, outputPath: string) => {
    const group = groups.value.find(item => item.id === groupId)
    if (group) group.outputPath = outputPath
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
    getEffectiveSettings,
    getEffectiveOutputPath,
    addFile,
    updateFileSettings,
    updateFileOutputPath,
    updateGroupSettings,
    updateGroupOutputPath,
    setAnalysisState,
    clearAnalysis,
    recordActualSize,
    bindJobTask,
    createGroup,
    prepareQuickPacks,
    addQuickPack,
    addTemplateDraft,
    replaceWithQuickPack,
    dissolveGroup,
    removeFileFromGroup,
    removeFile,
    removeJobsByTaskIds,
    requestAutoStart,
    consumeAutoStart
  }
})
