import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { CompressionFormatId } from '@/utils/compressionFormat'
import type { CompressionAnalysisResult } from '@/composables/useTauriCommands'

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
    estimatedSize,
    compressionAnalysis,
    totalOriginalSize,
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
    replaceWithQuickPack,
    dissolveGroup,
    removeFileFromGroup,
    removeFile,
    removeJobsByTaskIds,
    requestAutoStart,
    consumeAutoStart
  }
})
