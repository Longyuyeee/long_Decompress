import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface CompressionGroup {
  id: string
  name: string
  files: string[]
  themeColor: string
  expanded: boolean
  settings: {
    format: 'zip' | '7z' | 'tar'
    level: number
    password?: string
  }
}

export const useCompressionStore = defineStore('compression', () => {
  const selectedFiles = ref<string[]>([])
  const groups = ref<CompressionGroup[]>([])
  
  // 预计体积预演数据
  const estimatedSize = ref<Record<string, number>>({})

  const totalOriginalSize = computed(() => {
    // 假设我们有一个获取文件大小的逻辑
    return 0 
  })

  // 磁吸打组逻辑
  const createGroup = (files: string[]) => {
    const id = Date.now().toString()
    const colors = ['#3b82f6', '#8b5cf6', '#ec4899', '#10b981', '#f59e0b']
    const themeColor = colors[groups.value.length % colors.length]
    
    groups.value.push({
      id,
      name: `新建压缩组 ${groups.value.length + 1}`,
      files: [...files],
      themeColor,
      expanded: true,
      settings: {
        format: 'zip',
        level: 5
      }
    })
    
    // 从未分组列表中移除
    selectedFiles.value = selectedFiles.value.filter(f => !files.includes(f))
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
