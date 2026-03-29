import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface FileObject {
  name: string
  path: string
  size: number
  type: string
  isDirectory: boolean
}

export interface CompressionOptions {
  format: 'zip' | '7z' | 'tar' | 'gz' | 'bz2' | 'tar.gz' | 'tar.bz2' | 'xz' | 'tar.xz' | 'rar'
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
  settings: CompressionOptions
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
  
  // 预计体积预演数据
  const estimatedSize = ref<Record<string, number>>({})

  const totalOriginalSize = computed(() => {
    return selectedFiles.value.reduce((acc, f) => acc + f.size, 0) + 
           groups.value.reduce((acc, g) => acc + g.files.reduce((ga, f) => ga + f.size, 0), 0)
  })

  // 磁吸打组逻辑
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
      expanded: true,
      settings: {
        format: 'zip',
        level: 6,
        password: '',
        filename: `新建压缩组 ${groups.value.length + 1}`,
        splitArchive: false,
        splitSize: '1024',
        keepStructure: true,
        deleteAfter: false,
        createSolidArchive: false
      }
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

  return {
    selectedFiles,
    groups,
    estimatedSize,
    totalOriginalSize,
    createGroup,
    dissolveGroup
  }
})
