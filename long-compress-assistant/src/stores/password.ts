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

// --- Store 定义 ---

export const usePasswordStore = defineStore('password', () => {
  // 状态
  const entries = ref<PasswordEntry[]>([])
  const groups = ref<PasswordGroup[]>([])
  const isUnlocked = ref(false)
  const isLoading = ref(false)
  const searchQuery = ref('')
  const currentCategory = ref<PasswordCategory | 'All'>('All')
  const errorMessage = ref('')

  // 计算属性
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

    return result
  })

  const favoriteEntries = computed(() => entries.value.filter(e => e.favorite))

  // --- 方法 ---

  // 初始化检查
  const checkUnlockStatus = async () => {
    try {
      isUnlocked.value = await invoke<boolean>('is_encrypted_password_service_unlocked')
      if (isUnlocked.value) {
        await fetchAllData()
      }
    } catch (e) {
      console.error('检查解锁状态失败', e)
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

  return {
    // 状态
    entries,
    groups,
    isUnlocked,
    isLoading,
    searchQuery,
    currentCategory,
    errorMessage,
    
    // 计算属性
    filteredEntries,
    favoriteEntries,
    
    // 方法
    checkUnlockStatus,
    unlock,
    lock,
    fetchAllData,
    addEntry,
    deleteEntry,
    updateEntry,
    remoteSearch,
    findCandidatePasswords
  }
})
