import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { FileItem } from '../types'

export interface FileHistory {
  id: string
  fileId: string
  action: 'decompress' | 'compress' | 'preview'
  status: 'success' | 'error'
  timestamp: Date
  outputPath?: string
  error?: string
}

export interface FavoriteFile {
  id: string
  fileId: string
  name: string
  path: string
  addedAt: Date
  tags: string[]
}

export const useFileStore = defineStore('file', () => {
  // 状�?
  const files = ref<FileItem[]>([])
  const selectedFiles = ref<string[]>([]) // 存储选中的文件ID
  const fileHistory = ref<FileHistory[]>([])
  const favoriteFiles = ref<FavoriteFile[]>([])
  const currentDirectory = ref('')
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // 计算属�?
  const selectedFileItems = computed(() => {
    return files.value.filter(file => selectedFiles.value.includes(file.id))
  })

  const totalSelectedSize = computed(() => {
    return selectedFileItems.value.reduce((total, file) => total + file.size, 0)
  })

  const recentHistory = computed(() => {
    return [...fileHistory.value]
      .sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime())
      .slice(0, 10)
  })

  const favoritesByTag = computed(() => {
    const result: Record<string, FavoriteFile[]> = {}
    favoriteFiles.value.forEach(favorite => {
      favorite.tags.forEach(tag => {
        if (!result[tag]) {
          result[tag] = []
        }
        result[tag].push(favorite)
      })
    })
    return result
  })

  // 方法 - 文件操作
  const addFile = (file: Omit<FileItem, 'id' | 'status' | 'progress'>) => {
    const fileItem: FileItem = {
      ...file,
      id: generateId(),
      status: 'pending',
      progress: 0
    }

    files.value.push(fileItem)
    return fileItem.id
  }

  const removeFile = (fileId: string) => {
    files.value = files.value.filter(file => file.id !== fileId)
    selectedFiles.value = selectedFiles.value.filter(id => id !== fileId)
  }

  const clearFiles = () => {
    files.value = []
    selectedFiles.value = []
  }

  const updateFileStatus = (fileId: string, status: FileItem['status'], progress?: number) => {
    const file = files.value.find(f => f.id === fileId)
    if (file) {
      file.status = status
      if (progress !== undefined) {
        file.progress = progress
      }
    }
  }

  // 方法 - 选择操作
  const selectFile = (fileId: string) => {
    if (!selectedFiles.value.includes(fileId)) {
      selectedFiles.value.push(fileId)
    }
  }

  const deselectFile = (fileId: string) => {
    selectedFiles.value = selectedFiles.value.filter(id => id !== fileId)
  }

  const toggleFileSelection = (fileId: string) => {
    if (selectedFiles.value.includes(fileId)) {
      deselectFile(fileId)
    } else {
      selectFile(fileId)
    }
  }

  const selectAllFiles = () => {
    selectedFiles.value = files.value.map(file => file.id)
  }

  const deselectAllFiles = () => {
    selectedFiles.value = []
  }

  // 方法 - 历史记录
  const addToHistory = (
    fileId: string,
    action: FileHistory['action'],
    status: FileHistory['status'],
    outputPath?: string,
    error?: string
  ) => {
    const history: FileHistory = {
      id: generateId(),
      fileId,
      action,
      status,
      timestamp: new Date(),
      outputPath,
      error
    }

    fileHistory.value.unshift(history)

    // 保持历史记录不超�?00�?
    if (fileHistory.value.length > 100) {
      fileHistory.value = fileHistory.value.slice(0, 100)
    }

    saveHistoryToStorage()
  }

  const clearHistory = () => {
    fileHistory.value = []
    localStorage.removeItem('file-history')
  }

  // 方法 - 收藏�?
  const addToFavorites = (fileId: string, name: string, path: string, tags: string[] = []) => {
    const favorite: FavoriteFile = {
      id: generateId(),
      fileId,
      name,
      path,
      addedAt: new Date(),
      tags
    }

    favoriteFiles.value.push(favorite)
    saveFavoritesToStorage()
  }

  const removeFromFavorites = (favoriteId: string) => {
    favoriteFiles.value = favoriteFiles.value.filter(fav => fav.id !== favoriteId)
    saveFavoritesToStorage()
  }

  const updateFavoriteTags = (favoriteId: string, tags: string[]) => {
    const favorite = favoriteFiles.value.find(fav => fav.id === favoriteId)
    if (favorite) {
      favorite.tags = tags
      saveFavoritesToStorage()
    }
  }

  // 方法 - 目录操作
  const setCurrentDirectory = (path: string) => {
    currentDirectory.value = path
    localStorage.setItem('current-directory', path)
  }

  // 存储相关方法
  const saveHistoryToStorage = () => {
    try {
      const historyData = fileHistory.value.map(history => ({
        ...history,
        timestamp: history.timestamp.toISOString()
      }))
      localStorage.setItem('file-history', JSON.stringify(historyData))
    } catch (err) {
      console.error('Failed to save history to storage:', err)
    }
  }

  const loadHistoryFromStorage = () => {
    try {
      const savedHistory = localStorage.getItem('file-history')
      if (savedHistory) {
        const historyData = JSON.parse(savedHistory)
        fileHistory.value = historyData.map((item: any) => ({
          ...item,
          timestamp: new Date(item.timestamp)
        }))
      }
    } catch (err) {
      console.error('Failed to load history from storage:', err)
    }
  }

  const saveFavoritesToStorage = () => {
    try {
      const favoritesData = favoriteFiles.value.map(fav => ({
        ...fav,
        addedAt: fav.addedAt.toISOString()
      }))
      localStorage.setItem('favorite-files', JSON.stringify(favoritesData))
    } catch (err) {
      console.error('Failed to save favorites to storage:', err)
    }
  }

  const loadFavoritesFromStorage = () => {
    try {
      const savedFavorites = localStorage.getItem('favorite-files')
      if (savedFavorites) {
        const favoritesData = JSON.parse(savedFavorites)
        favoriteFiles.value = favoritesData.map((item: any) => ({
          ...item,
          addedAt: new Date(item.addedAt)
        }))
      }
    } catch (err) {
      console.error('Failed to load favorites from storage:', err)
    }
  }

  // 辅助函数
  const generateId = () => {
    return Date.now().toString(36) + Math.random().toString(36).substr(2)
  }

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B'

    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))

    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  const getFileExtension = (filename: string): string => {
    return filename.slice((filename.lastIndexOf('.') - 1 >>> 0) + 2)
  }

  // 初始�?
  const initialize = () => {
    loadHistoryFromStorage()
    loadFavoritesFromStorage()

    const savedDirectory = localStorage.getItem('current-directory')
    if (savedDirectory) {
      currentDirectory.value = savedDirectory
    }
  }

  // 执行初始�?
  initialize()

  return {
    // 状�?
    files,
    selectedFiles,
    fileHistory,
    favoriteFiles,
    currentDirectory,
    isLoading,
    error,

    // 计算属�?
    selectedFileItems,
    totalSelectedSize,
    recentHistory,
    favoritesByTag,

    // 文件操作方法
    addFile,
    removeFile,
    clearFiles,
    updateFileStatus,

    // 选择操作方法
    selectFile,
    deselectFile,
    toggleFileSelection,
    selectAllFiles,
    deselectAllFiles,

    // 历史记录方法
    addToHistory,
    clearHistory,

    // 收藏夹方�?
    addToFavorites,
    removeFromFavorites,
    updateFavoriteTags,

    // 目录操作方法
    setCurrentDirectory,

    // 工具方法
    formatFileSize,
    getFileExtension,

    // 存储方法
    saveHistoryToStorage,
    loadHistoryFromStorage,
    saveFavoritesToStorage,
    loadFavoritesFromStorage
  }
})
