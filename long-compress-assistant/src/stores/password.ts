import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'

// --- 类型定义 ---

export enum PasswordCategory {
  Personal = 'Personal',
  Work = 'Work',
  Finance = 'Finance',
  Social = 'Social',
  Shopping = 'Shopping',
  Entertainment = 'Entertainment',
  Education = 'Education',
  Travel = 'Travel',
  Health = 'Health',
  Other = 'Other'
}

export enum PasswordStrength {
  VeryWeak = 'VeryWeak',
  Weak = 'Weak',
  Medium = 'Medium',
  Strong = 'Strong',
  VeryStrong = 'VeryStrong'
}

export enum CustomFieldType {
  Text = 'Text',
  Password = 'Password',
  Email = 'Email',
  Url = 'Url',
  Phone = 'Phone',
  Date = 'Date',
  Number = 'Number',
  MultilineText = 'MultilineText'
}

export interface CustomField {
  name: string
  value: string
  field_type: CustomFieldType
  sensitive: boolean
}

export interface PasswordEntry {
  id: string
  name: string
  username?: string | null
  password: string
  url?: string | null
  notes?: string | null
  tags: string[]
  category: PasswordCategory
  strength: PasswordStrength
  created_at: string
  updated_at: string
  last_used?: string | null
  expires_at?: string | null
  favorite: boolean
  use_count: number
  custom_fields: CustomField[]
  }
export interface PasswordGroup {
  id: string
  name: string
  description?: string | null
  category: PasswordCategory
  entry_ids: string[]
  created_at: string
  updated_at: string
}

export interface AddPasswordRequest {
  name: string
  username?: string
  password: string
  url?: string
  notes?: string
  tags: string[]
  category: PasswordCategory
  expires_at?: string | Date
  custom_fields: CustomField[]
}

export interface UpdatePasswordRequest extends Partial<AddPasswordRequest> {
  id: string
}

export interface PasswordStrengthAssessment {
  score: number
  entropyBits: number
  crackTimeDisplay: string
  issues: Array<{ description: string }>
  recommendations: string[]
}

// --- Store 定义 ---

export const usePasswordStore = defineStore('password', () => {
  // 状态
  const entries = ref<PasswordEntry[]>([])
  const groups = ref<PasswordGroup[]>([])
  const isUnlocked = ref(false) // 默认未解锁
  const isLoading = ref(false)
  const searchQuery = ref('')
  const currentCategory = ref<PasswordCategory | 'All'>('All')
  const errorMessage = ref('')
  const successMessage = ref('')
  const selectedPasswords = ref<string[]>([])

  // 计算属性
  const isAllSelected = computed(() => {
    return filteredEntries.value.length > 0 && selectedPasswords.value.length === filteredEntries.value.length
  })

  const filteredEntries = computed(() => {
    let result = entries.value

    if (currentCategory.value !== 'All') {
      result = result.filter(e => e.category === currentCategory.value)
    }

    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase()
      result = result.filter(e => 
        e.name.toLowerCase().includes(q) || 
        (e.username && e.username.toLowerCase().includes(q)) ||
        (e.url && e.url.toLowerCase().includes(q)) ||
        e.tags.some(t => t.toLowerCase().includes(q))
      )
    }

    // 按调用次数从高到低排序，其次按最后使用时间
    return result.sort((a, b) => {
      if ((b.use_count || 0) !== (a.use_count || 0)) {
        return (b.use_count || 0) - (a.use_count || 0)
      }
      const timeA = a.last_used ? new Date(a.last_used).getTime() : 0
      const timeB = b.last_used ? new Date(b.last_used).getTime() : 0
      return timeB - timeA
    })
  })

  const favoriteEntries = computed(() => entries.value.filter(e => e.favorite))

  // --- 方法 ---

  // 初始化检查
  const checkUnlockStatus = async () => {
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
    }
  }

  const autoInitialize = async () => {
    const defaultMaster = 'long-decompress-default-key'
    try {
      // 先尝试解锁
      const success = await invoke<boolean>('unlock_encrypted_password_service', { masterPassword: defaultMaster })
      if (success) {
        isUnlocked.value = true
        await fetchAllData()
      } else {
        // 解锁失败可能是还没初始化，尝试初始化
        await invoke('init_encrypted_password_service', { masterPassword: defaultMaster })
        isUnlocked.value = true
        await fetchAllData()
      }
    } catch (e) {
      console.error('自动初始化密码服务失败:', e)
      isUnlocked.value = false
      errorMessage.value = "密码保险箱初始化失败，请检查后端状态"
    }
  }

  // 解锁
  const unlock = async (password: string) => {
    isLoading.value = true
    errorMessage.value = ''
    try {
      const success = await invoke<boolean>('unlock_encrypted_password_service', { masterPassword: password })
      if (success) {
        isUnlocked.value = true
        await fetchAllData()
      } else {
        errorMessage.value = '主密码错误'
      }
      return success
    } catch (e) {
      errorMessage.value = `解锁失败: ${e}`
      return false
    } finally {
      isLoading.value = false
    }
  }

  // 锁定
  const lock = async () => {
    try {
      await invoke('lock_encrypted_password_service')
      isUnlocked.value = false
      entries.value = []
      groups.value = []
    } catch (e) {
      console.error('锁定失败:', e)
    }
  }

  // 获取数据
  const fetchAllData = async () => {
    if (!isUnlocked.value) return
    isLoading.value = true
    try {
      const [allEntries, allGroups] = await Promise.all([
        invoke<PasswordEntry[]>('list_encrypted_passwords'),
        invoke<PasswordGroup[]>('list_password_groups')
      ])
      entries.value = allEntries
      groups.value = allGroups
    } catch (e) {
      console.error('获取密码本数据失败', e)
      errorMessage.value = `获取数据失败: ${e}`
    } finally {
      isLoading.value = false
    }
  }

  // 添加
  const addEntry = async (entryRequest: any) => {
    try {
      const newEntry = await invoke<PasswordEntry>('add_encrypted_password', { entry: entryRequest })
      entries.value.push(newEntry)
      return newEntry
    } catch (e) {
      if (typeof e === 'string' && e.includes('服务未初始化')) {
        // 尝试重新初始化并重试一次
        await checkUnlockStatus()
        if (isUnlocked.value) {
          const newEntry = await invoke<PasswordEntry>('add_encrypted_password', { entry: entryRequest })
          entries.value.push(newEntry)
          return newEntry
        }
      }
      console.error('添加失败:', e)
      throw e
    }
  }

  // 删除
  const deleteEntry = async (id: string) => {
    try {
      await invoke('delete_encrypted_password', { id })
      entries.value = entries.value.filter(e => e.id !== id)
    } catch (e) {
      console.error('删除失败:', e)
      throw e
    }
  }

  // 更新
  const updateEntry = async (id: string, entryRequest: any) => {
    try {
      const updated = await invoke<PasswordEntry>('update_encrypted_password', { id, entry: entryRequest })
      const index = entries.value.findIndex(e => e.id === id)
      if (index !== -1) {
        entries.value[index] = updated
      }
      return updated
    } catch (e) {
      console.error('更新失败:', e)
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
    
    // 2. 匹配名称或标签中包含文件名的条目
    entries.value.forEach(e => {
      if (e.name.toLowerCase().includes(stem) || e.tags.some(t => t.toLowerCase().includes(stem))) {
        candidates.add(e.password)
      }
    })
    
    // 3. 兜底：添加最近使用的 5 个密码
    const recent = [...entries.value]
      .sort((a, b) => {
        const timeA = a.last_used ? new Date(a.last_used).getTime() : 0
        const timeB = b.last_used ? new Date(b.last_used).getTime() : 0
        return timeB - timeA
      })
      .slice(0, 5)
    
    recent.forEach(e => candidates.add(e.password))
    
    return Array.from(candidates)
  }

  // 兼容性计算属性 (适配旧版驼峰命名)
  const filteredPasswords = computed(() => filteredEntries.value)
  const availableTags = computed(() => {
    const tags = new Set<string>()
    entries.value.forEach(e => e.tags.forEach(t => tags.add(t)))
    return Array.from(tags)
  })

  // 统计信息兼容
  const statistics = computed(() => {
    const stats = {
      total: entries.value.length,
      favorite: favoriteEntries.value.length,
      byCategory: {} as Record<string, number>,
      byStrength: {
        veryWeak: 0,
        weak: 0,
        medium: 0,
        strong: 0,
        veryStrong: 0
      }
    }

    entries.value.forEach(e => {
      // 类别统计
      stats.byCategory[e.category] = (stats.byCategory[e.category] || 0) + 1
      
      // 强度统计
      switch (e.strength) {
        case PasswordStrength.VeryWeak: stats.byStrength.veryWeak++; break
        case PasswordStrength.Weak: stats.byStrength.weak++; break
        case PasswordStrength.Medium: stats.byStrength.medium++; break
        case PasswordStrength.Strong: stats.byStrength.strong++; break
        case PasswordStrength.VeryStrong: stats.byStrength.veryStrong++; break
      }
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
  const setSearchFilters = (filters: any) => { /* 实现逻辑 */ }
  const clearSearchFilters = () => { /* 实现逻辑 */ }
  const setSort = (field: string, desc: boolean) => { /* 实现逻辑 */ }

  // 操作桩
  const toggleFavorite = async (id: string) => { /* 实现逻辑 */ }
  const archivePassword = async (id: string) => { /* 实现逻辑 */ }
  const deleteSelectedPasswords = async () => { /* 实现逻辑 */ }
  const usePassword = async (id: string) => { return '' }
  const assessPasswordStrength = async (password: string) => {
    return { score: 50, entropyBits: 0, crackTimeDisplay: '未知', issues: [], recommendations: [] }
  }
  const hideAllPasswords = () => { /* 实现逻辑 */ }

  return {
    // 状态
    entries,
    groups,
    isUnlocked,
    isLoading,
    searchQuery,
    currentCategory,
    errorMessage,
    error: errorMessage, 
    successMessage,
    selectedPasswords,
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
    unlock,
    lock,
    fetchAllData,
    loadPasswords,
    addEntry,
    addPassword,
    deleteEntry,
    deletePassword,
    updateEntry,
    updatePassword,
    remoteSearch,
    findCandidatePasswords,
    
    // 别名/桩方法
    setSearchFilters,
    clearSearchFilters,
    setSort,
    toggleFavorite,
    archivePassword,
    deleteSelectedPasswords,
    usePassword,
    assessPasswordStrength,
    hideAllPasswords,
    
    // 辅助工具方法
    togglePasswordVisibility: (id: string) => { /* 简单实现 */ },
    showPassword: (id: string) => false,
    formatTime: (date?: string | null) => date ? new Date(date).toLocaleString() : '从不',
    getStrengthColor: (s: PasswordStrength | number) => 'bg-green-500',
    getStrengthTextColor: (s: PasswordStrength | number) => 'text-green-500',
    getStrengthLabel: (s: PasswordStrength | number) => '中等'
  }
})
