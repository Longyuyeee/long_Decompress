import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface FileObject {
  name: string
  path: string
  size: number
  type: string
  isDirectory: boolean
  expanded?: boolean
  settings?: CompressionOptions
  outputPath?: string
}

export interface CompressionOptions {
  format: 'zip' | '7z' | 'tar' | 'gz' | 'bz2' | 'tar.gz' | 'tar.bz2' | 'xz' | 'tar.xz' | 'rar' | 'zst' | 'tar.zst' | 'lzma'
  level: number
  password: string
  filename: string
  splitArchive: boolean
  splitSize: string
  keepStructure: boolean
  deleteAfter: boolean
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
    createSolidArchive: false
  })
  const globalOutputPath = ref('')
  
  // 预计体积预演数据
  const estimatedSize = ref<Record<string, number>>({})

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
    if (selectedFiles.value.some(existing => existing.path === file.path)) return
    selectedFiles.value.push({
      ...file,
      expanded: false
    })
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

  const createGroup = (paths: string[]) => {
    const id = Date.now().toString()
    const colors = ['#3b82f6', '#8b5cf6', '#ec4899', '#10b981', '#f59e0b']
    const themeColor = colors[groups.value.length % colors.length]
    
    // 找到对应的 FileObject
    const targetFiles = selectedFiles.value.filter(f => paths.includes(f.path))
    
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

  const dissolveGroup = (groupId: string) => {
    const index = groups.value.findIndex(g => g.id === groupId)
    if (index !== -1) {
      const group = groups.value[index]
      selectedFiles.value.push(...group.files)
      groups.value.splice(index, 1)
    }
  }

  const removeFileFromGroup = (groupId: string, filePath: string) => {
    const group = groups.value.find(g => g.id === groupId)
    if (!group) return
    group.files = group.files.filter(f => f.path !== filePath)
    // 如果组内没有文件了，自动解散
    if (group.files.length === 0) {
      groups.value = groups.value.filter(g => g.id !== groupId)
    }
  }

  return {
    selectedFiles,
    groups,
    globalSettings,
    globalOutputPath,
    estimatedSize,
    totalOriginalSize,
    cloneSettings,
    getEffectiveSettings,
    getEffectiveOutputPath,
    addFile,
    updateFileSettings,
    updateFileOutputPath,
    updateGroupSettings,
    updateGroupOutputPath,
    createGroup,
    dissolveGroup,
    removeFileFromGroup
  }
})
