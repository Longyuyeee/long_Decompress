import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { translations } from '@/i18n'

const lang = () => localStorage.getItem('app-language') || 'zh-CN'
const t = (key: string) => translations[lang()]?.[key] || translations['zh-CN']?.[key] || key

// --- 类型定义 ---

export enum PasswordCategory {
  General = 'General',
  Work = 'Work',
  Media = 'Media',
  Documents = 'Documents',
  Other = 'Other'
}

export interface PasswordEntry {
  id: string
  name: string
  password: string
  notes?: string | null
  tags: string[]
  category: PasswordCategory
  created_at: string
  updated_at: string
  last_used?: string | null
  favorite: boolean
  use_count: number
  usage_history: Record<string, number>
}

export interface AddPasswordRequest {
  name: string
  password: string
  notes?: string | null
  tags: string[]
  category?: PasswordCategory
  favorite?: boolean
}

export interface UpdatePasswordRequest extends Partial<AddPasswordRequest> {
  id: string
}

// --- Store 定义 ---

export const usePasswordStore = defineStore('password', () => {
  // 状态
  const entries = ref<PasswordEntry[]>([])
  const isUnlocked = ref(false) // 默认未解锁
  const isLoading = ref(false)
  const isInitialized = ref(false)
  const isSaving = ref(false)
  const searchQuery = ref('')
  const currentCategory = ref<PasswordCategory | 'All'>('All')
  const errorMessage = ref('')
  const successMessage = ref('')
  const selectedPasswords = ref<string[]>([])
  const favoritesOnly = ref(false)
  const sortField = ref<'use_count' | 'name' | 'updated_at' | 'last_used'>('use_count')
  const sortDescending = ref(true)

  // 计算属性
  const isAllSelected = computed(() => {
    return filteredEntries.value.length > 0 && selectedPasswords.value.length === filteredEntries.value.length
  })

  const filteredEntries = computed(() => {
    let result = [...entries.value]

    if (currentCategory.value !== 'All') {
      result = result.filter(e => e.category === currentCategory.value)
    }

    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase()
      result = result.filter(e => 
        e.name.toLowerCase().includes(q) || 
        (e.notes && e.notes.toLowerCase().includes(q)) ||
        e.tags.some(t => t.toLowerCase().includes(q))
      )
    }

    if (favoritesOnly.value) {
      result = result.filter(e => e.favorite)
    }

    return result.sort((a, b) => {
      let comparison = 0
      if (sortField.value === 'name') {
        comparison = a.name.localeCompare(b.name)
      } else if (sortField.value === 'updated_at') {
        comparison = new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime()
      } else if (sortField.value === 'last_used') {
        comparison = (a.last_used ? new Date(a.last_used).getTime() : 0) -
          (b.last_used ? new Date(b.last_used).getTime() : 0)
      } else {
        comparison = (a.use_count || 0) - (b.use_count || 0)
      }
      return sortDescending.value ? -comparison : comparison
    })
  })

  const favoriteEntries = computed(() => entries.value.filter(e => e.favorite))

  // --- 方法 ---

  // 初始化检查
  const checkUnlockStatus = async () => {
    if (isInitialized.value) {
      // 解压服务可能在其他页面更新文件保险箱；重新进入时必须同步后端。
      if (isUnlocked.value) await fetchAllData()
      return
    }
    isLoading.value = true
    try {
      const unlocked = await invoke<boolean>('is_encrypted_password_service_unlocked')
      if (unlocked) {
        isUnlocked.value = true
        await fetchAllData()
      } else {
        // 尝试使用默认密码自动初始化/解锁 (为了用户体验)
        await autoInitialize()
      }
    } catch (e) {
      // 如果后端报错“服务未初始化”，说明需要 call init
      await autoInitialize()
    } finally {
      isLoading.value = false
      isInitialized.value = true
    }
  }

  const autoInitialize = async () => {
    try {
      // 安装密钥及其解锁流程全部留在 Rust 后端，不向 WebView 暴露。
      await invoke<boolean>('ensure_encrypted_password_service')
      isUnlocked.value = true
      await fetchAllData()
    } catch (e) {
      console.error('自动初始化密码服务失败:', e)
      isUnlocked.value = false
      errorMessage.value = t('vault.init_failed')
    }
  }

  const retryInitialization = async () => {
    isInitialized.value = false
    isUnlocked.value = false
    errorMessage.value = ''
    await checkUnlockStatus()
  }

  // 获取数据
  const fetchAllData = async () => {
    if (!isUnlocked.value) return
    isLoading.value = true
    try {
      entries.value = await invoke<PasswordEntry[]>('list_encrypted_passwords')
    } catch (e) {
      console.error('获取密码本数据失败', e)
      errorMessage.value = t('vault.fetch_failed').replace('{0}', String(e))
    } finally {
      isLoading.value = false
    }
  }

  // 添加
  // 新增
  const addEntry = async (entryRequest: any) => {
    isSaving.value = true
    try {
      // console.log('Invoke: add_encrypted_password')
      const newEntry = await invoke<PasswordEntry>('add_encrypted_password', { entry: entryRequest })
      entries.value.unshift(newEntry)
      return newEntry
    } catch (e) {
      console.error('Add failed:', e)
      throw e
    } finally {
      isSaving.value = false
    }
  }

  // 删除
  const deleteEntry = async (id: string) => {
    try {
      // console.log('Invoke: delete_encrypted_password', id)
      await invoke('delete_encrypted_password', { id })
      entries.value = entries.value.filter(e => e.id !== id)
    } catch (e) {
      console.error('Delete failed:', e)
      throw e
    }
  }

  // 清空
  const clearAll = async () => {
    try {
      // console.log('Invoke: clear_encrypted_passwords')
      await invoke('clear_encrypted_passwords')
      entries.value = []
    } catch (e) {
      console.error('Clear failed:', e)
      throw e
    }
  }

  // 更新
  const updateEntry = async (id: string, entryRequest: Partial<AddPasswordRequest>) => {
    try {
      // console.log('Invoke: update_encrypted_password', id)
      const originalEntry = entries.value.find(e => e.id === id)
      if (!originalEntry) throw new Error('找不到原始条目')

      // 只回传归档密码领域字段。生命周期统计由 Rust 原子保留，旧网站字段不再穿过 WebView。
      const payload: AddPasswordRequest = {
        name: entryRequest.name ?? originalEntry.name,
        password: entryRequest.password ?? originalEntry.password,
        notes: entryRequest.notes ?? originalEntry.notes,
        tags: entryRequest.tags ?? originalEntry.tags,
        category: entryRequest.category,
        favorite: entryRequest.favorite ?? originalEntry.favorite,
      }
      const updated = await invoke<PasswordEntry>('update_encrypted_password', { id, entry: payload })
      
      // console.log('Update success:', updated)
      const index = entries.value.findIndex(e => e.id === id)
      if (index !== -1) {
        entries.value[index] = updated
      }
      return updated
    } catch (e) {
      console.error('Update failed:', e)
      throw e
    }
  }

  // 搜索 (后端搜索作为补充)
  const remoteSearch = async (query: string) => {
    if (!query) return await fetchAllData()
    isLoading.value = true
    try {
      entries.value = await invoke<PasswordEntry[]>('search_encrypted_passwords', { query })
    } catch (e) {
      console.error('搜索失败:', e)
    } finally {
      isLoading.value = false
    }
  }

  // 为解压寻找候选密码(TSK-103)
  const findCandidatePasswords = (fileName: string): string[] => {
    const candidates = new Set<string>()

    // 1. 提取文件名中的关键词 (排除常见后缀)
    const stem = fileName.split('.')[0].toLowerCase()

    // 2. 匹配名称或标签中包含文件名的条目 (优先级最高)
    const matched: PasswordEntry[] = []
    const unmatched: PasswordEntry[] = []

    entries.value.forEach(e => {
      if (e.name.toLowerCase().includes(stem) || e.tags.some(t => t.toLowerCase().includes(stem))) {
        matched.push(e)
      } else {
        unmatched.push(e)
      }
    })

    // 3. 按优先级排序：匹配的按使用次数排序，未匹配的按最后使用时间排序
    const sortedMatched = matched.sort((a, b) => (b.use_count || 0) - (a.use_count || 0))
    const sortedUnmatched = unmatched.sort((a, b) => {
      const timeA = a.last_used ? new Date(a.last_used).getTime() : 0
      const timeB = b.last_used ? new Date(b.last_used).getTime() : 0
      return timeB - timeA
    })

    // 4. 先添加匹配的密码，再添加所有未匹配的密码（确保尝试所有密码）
    ;[...sortedMatched, ...sortedUnmatched].forEach(e => candidates.add(e.password))

    return Array.from(candidates)
  }

  // 兼容性计算属性 (适配旧版驼峰命名)
  const filteredPasswords = computed(() => filteredEntries.value)
  const availableTags = computed(() => {
    const tags = new Set<string>()
    entries.value.forEach(e => e.tags.forEach(t => tags.add(t)))
    return Array.from(tags)
  })

  // 归档线索统计，不对解压密码做传统网站密码强度判断。
  const statistics = computed(() => {
    const stats = {
      total: entries.value.length,
      favorite: favoriteEntries.value.length,
      byCategory: {} as Record<string, number>,
    }

    entries.value.forEach(e => {
      stats.byCategory[e.category] = (stats.byCategory[e.category] || 0) + 1
    })

    return stats
  })

  // 方法别名适配
  const loadPasswords = fetchAllData
  const addPassword = addEntry
  const deletePassword = deleteEntry
  const updatePassword = updateEntry

  // 分页相关桩
  const currentPage = ref(1)
  const pageSize = ref(10)
  const totalPages = computed(() => Math.ceil(filteredEntries.value.length / pageSize.value))
  const paginatedPasswords = computed(() => {
    const start = (currentPage.value - 1) * pageSize.value
    return filteredEntries.value.slice(start, start + pageSize.value)
  })

  // 搜索和过滤桩
  const setSearchFilters = (filters: { query?: string, category?: PasswordCategory | 'All', favoritesOnly?: boolean }) => {
    if (filters.query !== undefined) searchQuery.value = filters.query
    if (filters.category !== undefined) currentCategory.value = filters.category
    if (filters.favoritesOnly !== undefined) favoritesOnly.value = filters.favoritesOnly
    currentPage.value = 1
  }
  const clearSearchFilters = () => {
    searchQuery.value = ''
    currentCategory.value = 'All'
    favoritesOnly.value = false
    currentPage.value = 1
  }
  const setSort = (field: string, desc: boolean) => {
    if (['use_count', 'name', 'updated_at', 'last_used'].includes(field)) {
      sortField.value = field as typeof sortField.value
      sortDescending.value = desc
    }
  }

  // 操作桩
  const toggleFavorite = async (id: string) => {
    const entry = entries.value.find(e => e.id === id)
    if (!entry) throw new Error(`Password entry not found: ${id}`)
    return updateEntry(id, { favorite: !entry.favorite })
  }
  const deleteSelectedPasswords = async () => {
    const ids = [...selectedPasswords.value]
    await Promise.all(ids.map(id => deleteEntry(id)))
    selectedPasswords.value = []
  }
  const usePassword = async (id: string) => {
    const entry = entries.value.find(e => e.id === id)
    if (!entry) return ''
    const updated = await invoke<PasswordEntry>('increment_encrypted_password_use_count', { id })
    const index = entries.value.findIndex(item => item.id === id)
    if (index !== -1) entries.value[index] = updated
    return entry.password
  }
  return {
    // 状态
    entries,
    isUnlocked,
    isLoading,
    isInitialized,
    isSaving,
    searchQuery,
    currentCategory,
    errorMessage,
    error: errorMessage, 
    successMessage,
    selectedPasswords,
    favoritesOnly,
    sortField,
    sortDescending,
    currentPage,
    pageSize,
    
    // 计算属性
    filteredEntries,
    favoriteEntries,
    filteredPasswords,
    availableTags,
    statistics,
    isAllSelected,
    totalPages,
    paginatedPasswords,
    
    // 方法
    checkUnlockStatus,
    retryInitialization,
    fetchAllData,
    loadPasswords,
    addEntry,
    addPassword,
    deleteEntry,
    deletePassword,
    clearAll,
    updateEntry,
    updatePassword,
    remoteSearch,
    findCandidatePasswords,
    
    // 别名/桩方法
    setSearchFilters,
    clearSearchFilters,
    setSort,
    toggleFavorite,
    deleteSelectedPasswords,
    usePassword,
    // 辅助工具方法
    formatTime: (date?: string | null) => date ? new Date(date).toLocaleString() : '从不',
  }
})
