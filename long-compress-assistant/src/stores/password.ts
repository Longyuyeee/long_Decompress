import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { translations } from '@/i18n'

const lang = () => localStorage.getItem('app-language') || 'zh-CN'
const t = (key: string) => translations[lang()]?.[key] || translations['zh-CN']?.[key] || key

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
  usage_history: Record<string, number>
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
        (e.username && e.username.toLowerCase().includes(q)) ||
        (e.url && e.url.toLowerCase().includes(q)) ||
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
      // 获取或创建每安装实例的随机主密钥（不再使用硬编码默认密码）
      const masterKey = await invoke<string>('get_or_create_master_key')
      // 先尝试解锁
      const success = await invoke<boolean>('unlock_encrypted_password_service', { masterPassword: masterKey })
      if (success) {
        isUnlocked.value = true
        await fetchAllData()
      } else {
        // 解锁失败可能是还没初始化，尝试初始化
        await invoke('init_encrypted_password_service', { masterPassword: masterKey })
        isUnlocked.value = true
        await fetchAllData()
      }
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
      const [allEntries, allGroups] = await Promise.all([
        invoke<PasswordEntry[]>('list_encrypted_passwords'),
        invoke<PasswordGroup[]>('list_password_groups')
      ])
      entries.value = allEntries
      groups.value = allGroups
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
  const updateEntry = async (id: string, entryRequest: any) => {
    try {
      // console.log('Invoke: update_encrypted_password', id)
      // 关键修复：合并原始数据，确保 strength, usage_history 等字段不丢失
      const originalEntry = entries.value.find(e => e.id === id)
      if (!originalEntry) throw new Error('找不到原始条目')
      
      const payload = { ...originalEntry, ...entryRequest, id }
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
  const assessPasswordStrength = async (password: string) => {
    const issues: string[] = []
    const recommendations: string[] = []

    if (!password) {
      return { score: 0, entropyBits: 0, crackTimeDisplay: 'N/A', issues: ['密码为空'], recommendations: ['请输入密码'] }
    }

    // 字符集分析
    const hasLower = /[a-z]/.test(password)
    const hasUpper = /[A-Z]/.test(password)
    const hasDigit = /\d/.test(password)
    const hasSpecial = /[^a-zA-Z0-9]/.test(password)
    const charsetSize =
      (hasLower ? 26 : 0) +
      (hasUpper ? 26 : 0) +
      (hasDigit ? 10 : 0) +
      (hasSpecial ? 32 : 0)

    // 估算熵 (bits)
    const entropyBits = password.length * Math.log2(charsetSize || 1)

    // 评分 0-100
    let score = 0
    score += Math.min(password.length * 6, 40) // length contribution
    score += hasLower && hasUpper ? 15 : (hasLower || hasUpper ? 8 : 0) // case diversity
    score += hasDigit ? 10 : 0
    score += hasSpecial ? 15 : 0
    score += password.length >= 12 ? 20 : password.length >= 8 ? 10 : 0 // bonus for length

    // 检测弱模式
    const commonPatterns = [
      /^12345/, /^password/i, /^qwerty/i, /^abc/i, /^111/, /^000/,
      /(.)\1{3,}/, // 重复字符
    ]
    for (const pattern of commonPatterns) {
      if (pattern.test(password)) {
        score = Math.max(0, score - 25)
        issues.push('包含常见弱密码模式')
        recommendations.push('避免使用常见序列或重复字符')
        break
      }
    }

    if (password.length < 8) {
      issues.push('密码长度不足8位')
      recommendations.push('建议至少使用8个字符')
    }
    if (!hasSpecial) {
      issues.push('缺少特殊字符')
      recommendations.push('建议添加特殊字符如 !@#$%')
    }
    if (charsetSize <= 26) {
      issues.push('字符集单一')
      recommendations.push('混合使用大小写字母、数字和特殊字符')
    }

    // 估算破解时间
    const guessesPerSecond = 1e9 // 假设 1 billion guesses/second
    const totalCombinations = Math.pow(charsetSize || 1, password.length)
    const secondsToCrack = totalCombinations / guessesPerSecond

    let crackTimeDisplay: string
    if (secondsToCrack < 60) crackTimeDisplay = '瞬间'
    else if (secondsToCrack < 3600) crackTimeDisplay = '< 1 小时'
    else if (secondsToCrack < 86400) crackTimeDisplay = '< 1 天'
    else if (secondsToCrack < 86400 * 365) crackTimeDisplay = '< 1 年'
    else if (secondsToCrack < 86400 * 365 * 100) crackTimeDisplay = '数百年'
    else crackTimeDisplay = '数千年以上'

    score = Math.min(100, Math.max(0, Math.round(score)))

    return {
      score,
      entropyBits: Math.round(entropyBits * 10) / 10,
      crackTimeDisplay,
      issues,
      recommendations,
    }
  }
  const hideAllPasswords = () => { /* 实现逻辑 */ }

  return {
    // 状态
    entries,
    groups,
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
