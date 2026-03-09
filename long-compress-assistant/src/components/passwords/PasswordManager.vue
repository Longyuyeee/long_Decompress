<template>
  <div class="password-manager">
    <!-- 头部：标题和操作按钮 -->
    <div class="mb-6">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold text-gray-900 dark:text-white">密码本管�?/h1>
          <p class="text-gray-600 dark:text-gray-400 mt-1">管理您的压缩文件密码，提高解压效�?/p>
        </div>
        <div class="flex space-x-3">
          <button
            @click="showAddPasswordModal = true"
            class="glass-button-primary px-4 py-2"
          >
            <i class="pi pi-plus mr-2"></i>
            添加密码
          </button>
          <button
            @click="importPasswords"
            class="glass-button px-4 py-2"
          >
            <i class="pi pi-upload mr-2"></i>
            导入
          </button>
          <button
            @click="exportPasswords"
            class="glass-button px-4 py-2"
          >
            <i class="pi pi-download mr-2"></i>
            导出
          </button>
        </div>
      </div>
    </div>

    <!-- 搜索和过�?-->
    <div class="mb-6 glass-card p-4">
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <!-- 搜索�?-->
        <div class="relative">
          <i class="pi pi-search absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400"></i>
          <input
            type="text"
            v-model="searchQuery"
            class="w-full glass-input pl-10"
            placeholder="搜索密码、标签或描述..."
            @input="handleSearch"
          />
        </div>

        <!-- 标签过滤 -->
        <div>
          <select
            v-model="selectedTag"
            class="w-full glass-input"
            @change="handleTagFilter"
          >
            <option value="">所有标�?/option>
            <option v-for="tag in availableTags" :key="tag" :value="tag">
              {{ tag }}
            </option>
          </select>
        </div>

        <!-- 排序选项 -->
        <div>
          <select
            v-model="sortBy"
            class="w-full glass-input"
            @change="handleSort"
          >
            <option value="createdAt">添加时间</option>
            <option value="lastUsed">最后使�?/option>
            <option value="strength">密码强度</option>
            <option value="name">名称</option>
          </select>
        </div>
      </div>
    </div>

    <!-- 密码列表 -->
    <div class="mb-6">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">
          密码列表 ({{ filteredPasswords.length }})
        </h2>
        <div class="flex items-center space-x-2">
          <button
            @click="toggleSelectAll"
            class="text-sm text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200"
          >
            {{ isAllSelected ? '取消全�? : '全�? }}
          </button>
          <button
            v-if="selectedPasswords.length > 0"
            @click="deleteSelected"
            class="text-sm text-red-600 hover:text-red-700"
          >
            <i class="pi pi-trash mr-1"></i>
            删除选中 ({{ selectedPasswords.length }})
          </button>
        </div>
      </div>

      <!-- 密码表格 -->
      <div class="glass-card overflow-hidden">
        <div class="overflow-x-auto">
          <table class="w-full">
            <thead>
              <tr class="border-b border-gray-200 dark:border-gray-700">
                <th class="py-3 px-4 text-left">
                  <input
                    type="checkbox"
                    v-model="isAllSelected"
                    class="rounded border-gray-300 text-primary focus:ring-primary"
                  />
                </th>
                <th class="py-3 px-4 text-left text-sm font-medium text-gray-700 dark:text-gray-300">
                  名称/描述
                </th>
                <th class="py-3 px-4 text-left text-sm font-medium text-gray-700 dark:text-gray-300">
                  密码
                </th>
                <th class="py-3 px-4 text-left text-sm font-medium text-gray-700 dark:text-gray-300">
                  标签
                </th>
                <th class="py-3 px-4 text-left text-sm font-medium text-gray-700 dark:text-gray-300">
                  强度
                </th>
                <th class="py-3 px-4 text-left text-sm font-medium text-gray-700 dark:text-gray-300">
                  最后使�?
                </th>
                <th class="py-3 px-4 text-left text-sm font-medium text-gray-700 dark:text-gray-300">
                  操作
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="password in paginatedPasswords"
                :key="password.id"
                class="border-b border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800/50"
              >
                <td class="py-3 px-4">
                  <input
                    type="checkbox"
                    :value="password.id"
                    v-model="selectedPasswords"
                    class="rounded border-gray-300 text-primary focus:ring-primary"
                  />
                </td>
                <td class="py-3 px-4">
                  <div>
                    <p class="font-medium text-gray-900 dark:text-white">
                      {{ password.name || '未命名密�? }}
                    </p>
                    <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
                      {{ password.description || '无描�? }}
                    </p>
                  </div>
                </td>
                <td class="py-3 px-4">
                  <div class="flex items-center">
                    <span class="font-mono">
                      {{ showPasswordMap[password.id] ? password.password : '•••••••�? }}
                    </span>
                    <button
                      @click="togglePasswordVisibility(password.id)"
                      class="ml-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
                    >
                      <i :class="showPasswordMap[password.id] ? 'pi pi-eye-slash' : 'pi pi-eye'"></i>
                    </button>
                    <button
                      @click="copyPassword(password.password)"
                      class="ml-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
                      :title="`复制密码`"
                    >
                      <i class="pi pi-copy"></i>
                    </button>
                  </div>
                </td>
                <td class="py-3 px-4">
                  <div class="flex flex-wrap gap-1">
                    <span
                      v-for="tag in password.tags"
                      :key="tag"
                      class="px-2 py-1 text-xs rounded-full bg-primary/10 text-primary"
                    >
                      {{ tag }}
                    </span>
                    <span v-if="password.tags.length === 0" class="text-gray-400 text-sm">
                      无标�?
                    </span>
                  </div>
                </td>
                <td class="py-3 px-4">
                  <div class="flex items-center">
                    <div class="w-24 h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                      <div
                        class="h-full rounded-full"
                        :class="strengthColorClasses[password.strength]"
                        :style="{ width: password.strength + '%' }"
                      ></div>
                    </div>
                    <span class="ml-2 text-sm" :class="strengthTextClasses[password.strength]">
                      {{ strengthLabels[password.strength] }}
                    </span>
                  </div>
                </td>
                <td class="py-3 px-4">
                  <div class="text-sm text-gray-600 dark:text-gray-400">
                    {{ formatTime(password.lastUsed) }}
                  </div>
                  <div class="text-xs text-gray-500 dark:text-gray-500">
                    使用次数: {{ password.usageCount }}
                  </div>
                </td>
                <td class="py-3 px-4">
                  <div class="flex items-center space-x-2">
                    <button
                      @click="editPassword(password)"
                      class="text-blue-600 hover:text-blue-700"
                      :title="`编辑`"
                    >
                      <i class="pi pi-pencil"></i>
                    </button>
                    <button
                      @click="usePassword(password)"
                      class="text-green-600 hover:text-green-700"
                      :title="`使用此密码`"
                    >
                      <i class="pi pi-play"></i>
                    </button>
                    <button
                      @click="deletePassword(password.id)"
                      class="text-red-600 hover:text-red-700"
                      :title="`删除`"
                    >
                      <i class="pi pi-trash"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- 空状�?-->
        <div v-if="filteredPasswords.length === 0" class="py-12 text-center">
          <i class="pi pi-lock text-4xl text-gray-300 dark:text-gray-600 mb-4"></i>
          <p class="text-gray-500 dark:text-gray-400">暂无密码记录</p>
          <p class="text-sm text-gray-400 dark:text-gray-500 mt-2">
            点击"添加密码"按钮开始管理您的密�?
          </p>
        </div>
      </div>
    </div>

    <!-- 分页 -->
    <div v-if="filteredPasswords.length > 0" class="flex items-center justify-between">
      <div class="text-sm text-gray-600 dark:text-gray-400">
        显示 {{ startIndex + 1 }}-{{ endIndex }} 条，�?{{ filteredPasswords.length }} �?
      </div>
      <div class="flex items-center space-x-2">
        <button
          @click="prevPage"
          :disabled="currentPage === 1"
          class="glass-button px-3 py-1"
          :class="{ 'opacity-50 cursor-not-allowed': currentPage === 1 }"
        >
          <i class="pi pi-chevron-left"></i>
        </button>
        <span class="text-sm text-gray-700 dark:text-gray-300">
          �?{{ currentPage }} �?/ �?{{ totalPages }} �?
        </span>
        <button
          @click="nextPage"
          :disabled="currentPage === totalPages"
          class="glass-button px-3 py-1"
          :class="{ 'opacity-50 cursor-not-allowed': currentPage === totalPages }"
        >
          <i class="pi pi-chevron-right"></i>
        </button>
      </div>
    </div>

    <!-- 统计信息 -->
    <div class="mt-8 grid grid-cols-1 md:grid-cols-4 gap-4">
      <div class="glass-card p-4">
        <div class="flex items-center">
          <i class="pi pi-lock text-2xl text-primary mr-3"></i>
          <div>
            <p class="text-sm text-gray-600 dark:text-gray-400">总密码数</p>
            <p class="text-2xl font-bold text-gray-900 dark:text-white">
              {{ passwords.length }}
            </p>
          </div>
        </div>
      </div>
      <div class="glass-card p-4">
        <div class="flex items-center">
          <i class="pi pi-star text-2xl text-yellow-500 mr-3"></i>
          <div>
            <p class="text-sm text-gray-600 dark:text-gray-400">强密�?/p>
            <p class="text-2xl font-bold text-gray-900 dark:text-white">
              {{ strongPasswordsCount }}
            </p>
          </div>
        </div>
      </div>
      <div class="glass-card p-4">
        <div class="flex items-center">
          <i class="pi pi-history text-2xl text-blue-500 mr-3"></i>
          <div>
            <p class="text-sm text-gray-600 dark:text-gray-400">今日使用</p>
            <p class="text-2xl font-bold text-gray-900 dark:text-white">
              {{ todayUsageCount }}
            </p>
          </div>
        </div>
      </div>
      <div class="glass-card p-4">
        <div class="flex items-center">
          <i class="pi pi-tags text-2xl text-green-500 mr-3"></i>
          <div>
            <p class="text-sm text-gray-600 dark:text-gray-400">标签数量</p>
            <p class="text-2xl font-bold text-gray-900 dark:text-white">
              {{ availableTags.length }}
            </p>
          </div>
        </div>
      </div>
    </div>

    <!-- 添加/编辑密码模态框 -->
    <div v-if="showAddPasswordModal" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div class="glass-card max-w-md w-full mx-4">
        <div class="flex items-center justify-between mb-6">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
            {{ editingPassword ? '编辑密码' : '添加新密�? }}
          </h3>
          <button
            @click="closeModal"
            class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
          >
            <i class="pi pi-times"></i>
          </button>
        </div>

        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              名称
            </label>
            <input
              type="text"
              v-model="newPassword.name"
              class="w-full glass-input"
              placeholder="为密码起个名�?
            />
          </div>

          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              描述
            </label>
            <textarea
              v-model="newPassword.description"
              class="w-full glass-input"
              rows="2"
              placeholder="描述这个密码的用�?
            ></textarea>
          </div>

          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              密码
            </label>
            <div class="relative">
              <input
                :type="showNewPassword ? 'text' : 'password'"
                v-model="newPassword.password"
                class="w-full glass-input pr-10"
                placeholder="输入密码"
                @input="updatePasswordStrength"
              />
              <button
                @click="showNewPassword = !showNewPassword"
                class="absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-500"
              >
                <i :class="showNewPassword ? 'pi pi-eye-slash' : 'pi pi-eye'"></i>
              </button>
            </div>
            <div v-if="newPassword.password" class="mt-2">
              <div class="flex items-center justify-between text-sm mb-1">
                <span>密码强度</span>
                <span :class="strengthTextClasses[passwordStrength]">
                  {{ strengthLabels[passwordStrength] }}
                </span>
              </div>
              <div class="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                <div
                  class="h-full rounded-full transition-all"
                  :class="strengthColorClasses[passwordStrength]"
                  :style="{ width: passwordStrength + '%' }"
                ></div>
              </div>
            </div>
          </div>

          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              标签
            </label>
            <div class="flex flex-wrap gap-2 mb-2">
              <span
                v-for="tag in newPassword.tags"
                :key="tag"
                class="px-3 py-1 text-sm rounded-full bg-primary/10 text-primary flex items-center"
              >
                {{ tag }}
                <button
                  @click="removeTag(tag)"
                  class="ml-1 text-primary/70 hover:text-primary"
                >
                  <i class="pi pi-times text-xs"></i>
                </button>
              </span>
            </div>
            <div class="flex space-x-2">
              <input
                type="text"
                v-model="tagInput"
                class="flex-1 glass-input"
                placeholder="输入标签，按回车添加"
                @keyup.enter="addTag"
              />
              <button
                @click="addTag"
                class="glass-button px-4"
              >
                添加
              </button>
            </div>
          </div>

          <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
            <div class="flex justify-end space-x-3">
              <button
                @click="closeModal"
                class="glass-button px-4 py-2"
              >
                取消
              </button>
              <button
                @click="savePassword"
                class="glass-button-primary px-4 py-2"
                :disabled="!newPassword.password"
              >
                {{ editingPassword ? '更新' : '保存' }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'

// 密码接口定义
interface PasswordItem {
  id: string
  name: string
  description: string
  password: string
  tags: string[]
  strength: number // 0-100
  lastUsed: Date
  usageCount: number
  createdAt: Date
  updatedAt: Date
}

// 状�?
const passwords = ref<PasswordItem[]>([])
const selectedPasswords = ref<string[]>([])
const searchQuery = ref('')
const selectedTag = ref('')
const sortBy = ref<'createdAt' | 'lastUsed' | 'strength' | 'name'>('createdAt')
const showPasswordMap = ref<Record<string, boolean>>({})
const showAddPasswordModal = ref(false)
const editingPassword = ref<PasswordItem | null>(null)
const newPassword = ref({
  name: '',
  description: '',
  password: '',
  tags: [] as string[]
})
const tagInput = ref('')
const showNewPassword = ref(false)
const passwordStrength = ref(0)
const currentPage = ref(1)
const pageSize = 10

// 密码强度相关
const strengthColorClasses: Record<number, string> = {
  0: 'bg-red-500',
  25: 'bg-orange-500',
  50: 'bg-yellow-500',
  75: 'bg-green-500',
  100: 'bg-green-600'
}

const strengthTextClasses: Record<number, string> = {
  0: 'text-red-500',
  25: 'text-orange-500',
  50: 'text-yellow-500',
  75: 'text-green-500',
  100: 'text-green-600'
}

const strengthLabels: Record<number, string> = {
  0: '非常�?,
  25: '�?,
  50: '中等',
  75: '�?,
  100: '非常�?
}

// 计算属�?
const filteredPasswords = computed(() => {
  let filtered = [...passwords.value]

  // 搜索过滤
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    filtered = filtered.filter(p =>
      p.name.toLowerCase().includes(query) ||
      p.description.toLowerCase().includes(query) ||
      p.tags.some(tag => tag.toLowerCase().includes(query))
    )
  }

  // 标签过滤
  if (selectedTag.value) {
    filtered = filtered.filter(p => p.tags.includes(selectedTag.value))
  }

  // 排序
  filtered.sort((a, b) => {
    switch (sortBy.value) {
      case 'createdAt':
        return b.createdAt.getTime() - a.createdAt.getTime()
      case 'lastUsed':
        return b.lastUsed.getTime() - a.lastUsed.getTime()
      case 'strength':
        return b.strength - a.strength
      case 'name':
        return a.name.localeCompare(b.name)
      default:
        return 0
    }
  })

  return filtered
})

const availableTags = computed(() => {
  const tags = new Set<string>()
  passwords.value.forEach(p => {
    p.tags.forEach(tag => tags.add(tag))
  })
  return Array.from(tags).sort()
})

const isAllSelected = computed({
  get: () => filteredPasswords.value.length > 0 &&
    selectedPasswords.value.length === filteredPasswords.value.length,
  set: (value) => {
    if (value) {
      selectedPasswords.value = filteredPasswords.value.map(p => p.id)
    } else {
      selectedPasswords.value = []
    }
  }
})

const strongPasswordsCount = computed(() => {
  return passwords.value.filter(p => p.strength >= 75).length
})

const todayUsageCount = computed(() => {
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  return passwords.value.filter(p => p.lastUsed >= today).length
})

// 分页相关
const totalPages = computed(() => {
  return Math.ceil(filteredPasswords.value.length / pageSize)
})

const paginatedPasswords = computed(() => {
  const start = (currentPage.value - 1) * pageSize
  const end = start + pageSize
  return filteredPasswords.value.slice(start, end)
})

const startIndex = computed(() => {
  return (currentPage.value - 1) * pageSize
})

const endIndex = computed(() => {
  return Math.min(startIndex.value + pageSize, filteredPasswords.value.length)
})

// 方法
const togglePasswordVisibility = (passwordId: string) => {
  showPasswordMap.value[passwordId] = !showPasswordMap.value[passwordId]
}

const copyPassword = async (password: string) => {
  try {
    await navigator.clipboard.writeText(password)
    alert('密码已复制到剪贴�?)
  } catch (err) {
    console.error('复制失败:', err)
    alert('复制失败，请手动复制')
  }
}

const toggleSelectAll = () => {
  isAllSelected.value = !isAllSelected.value
}

const deleteSelected = () => {
  if (confirm(`确定要删除选中�?${selectedPasswords.value.length} 个密码吗？`)) {
    passwords.value = passwords.value.filter(p => !selectedPasswords.value.includes(p.id))
    selectedPasswords.value = []
    saveToStorage()
  }
}

const editPassword = (password: PasswordItem) => {
  editingPassword.value = password
  newPassword.value = {
    name: password.name,
    description: password.description,
    password: password.password,
    tags: [...password.tags]
  }
  updatePasswordStrength()
  showAddPasswordModal.value = true
}

const usePassword = (password: PasswordItem) => {
  // 更新使用次数和时�?
  password.usageCount++
  password.lastUsed = new Date()
  saveToStorage()

  // 这里可以触发使用密码的事�?
  emit('password-selected', password.password)
}

const deletePassword = (passwordId: string) => {
  if (confirm('确定要删除这个密码吗�?)) {
    passwords.value = passwords.value.filter(p => p.id !== passwordId)
    saveToStorage()
  }
}

const addTag = () => {
  if (tagInput.value.trim() && !newPassword.value.tags.includes(tagInput.value.trim())) {
    newPassword.value.tags.push(tagInput.value.trim())
    tagInput.value = ''
  }
}

const removeTag = (tag: string) => {
  newPassword.value.tags = newPassword.value.tags.filter(t => t !== tag)
}

const updatePasswordStrength = () => {
  const password = newPassword.value.password
  let strength = 0

  // 简单的密码强度计算
  if (password.length >= 8) strength += 25
  if (/[a-z]/.test(password) && /[A-Z]/.test(password)) strength += 25
  if (/\d/.test(password)) strength += 25
  if (/[^a-zA-Z0-9]/.test(password)) strength += 25

  passwordStrength.value = strength
}

const savePassword = () => {
  if (!newPassword.value.password) return

  const now = new Date()

  if (editingPassword.value) {
    // 更新现有密码
    const index = passwords.value.findIndex(p => p.id === editingPassword.value!.id)
    if (index !== -1) {
      passwords.value[index] = {
        ...passwords.value[index],
        ...newPassword.value,
        strength: passwordStrength.value,
        updatedAt: now
      }
    }
  } else {
    // 添加新密�?
    const newItem: PasswordItem = {
      id: generateId(),
      ...newPassword.value,
      strength: passwordStrength.value,
      lastUsed: now,
      usageCount: 0,
      createdAt: now,
      updatedAt: now
    }
    passwords.value.unshift(newItem)
  }

  saveToStorage()
  closeModal()
}

const closeModal = () => {
  showAddPasswordModal.value = false
  editingPassword.value = null
  newPassword.value = {
    name: '',
    description: '',
    password: '',
    tags: []
  }
  tagInput.value = ''
  showNewPassword.value = false
  passwordStrength.value = 0
}

const importPasswords = () => {
  // TODO: 实现导入功能
  alert('导入功能开发中...')
}

const exportPasswords = () => {
  // TODO: 实现导出功能
  alert('导出功能开发中...')
}

const handleSearch = () => {
  currentPage.value = 1
}

const handleTagFilter = () => {
  currentPage.value = 1
}

const handleSort = () => {
  currentPage.value = 1
}

const prevPage = () => {
  if (currentPage.value > 1) {
    currentPage.value--
  }
}

const nextPage = () => {
  if (currentPage.value < totalPages.value) {
    currentPage.value++
  }
}

const formatTime = (date: Date): string => {
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const diffDays = Math.floor(diff / (1000 * 60 * 60 * 24))

  if (diffDays === 0) {
    return '今天'
  } else if (diffDays === 1) {
    return '昨天'
  } else if (diffDays < 7) {
    return `${diffDays}天前`
  } else if (diffDays < 30) {
    return `${Math.floor(diffDays / 7)}周前`
  } else {
    return date.toLocaleDateString()
  }
}

const generateId = (): string => {
  return Date.now().toString(36) + Math.random().toString(36).substr(2)
}

// 存储相关
const saveToStorage = () => {
  try {
    const data = passwords.value.map(p => ({
      ...p,
      lastUsed: p.lastUsed.toISOString(),
      createdAt: p.createdAt.toISOString(),
      updatedAt: p.updatedAt.toISOString()
    }))
    localStorage.setItem('passwords', JSON.stringify(data))
  } catch (err) {
    console.error('保存密码失败:', err)
  }
}

const loadFromStorage = () => {
  try {
    const saved = localStorage.getItem('passwords')
    if (saved) {
      const data = JSON.parse(saved)
      passwords.value = data.map((item: any) => ({
        ...item,
        lastUsed: new Date(item.lastUsed),
        createdAt: new Date(item.createdAt),
        updatedAt: new Date(item.updatedAt)
      }))
    }
  } catch (err) {
    console.error('加载密码失败:', err)
  }
}

// 事件
const emit = defineEmits<{
  (e: 'password-selected', password: string): void
}>()

// 初始�?
onMounted(() => {
  loadFromStorage()

  // 如果没有数据，添加一些示例数�?
  if (passwords.value.length === 0) {
    passwords.value = [
      {
        id: generateId(),
        name: '个人文档备份',
        description: '用于个人重要文档的压缩包',
        password: 'MyDoc@2024',
        tags: ['个人', '文档', '备份'],
        strength: 75,
        lastUsed: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000), // 2天前
        usageCount: 3,
        createdAt: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000), // 30天前
        updatedAt: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000)
      },
      {
        id: generateId(),
        name: '工作项目',
        description: '公司项目文件压缩密码',
        password: 'Work#Project123',
        tags: ['工作', '项目'],
        strength: 100,
        lastUsed: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000), // 7天前
        usageCount: 5,
        createdAt: new Date(Date.now() - 60 * 24 * 60 * 60 * 1000), // 60天前
        updatedAt: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000)
      },
      {
        id: generateId(),
        name: '照片�?,
        description: '家庭照片压缩�?,
        password: 'family2024',
        tags: ['家庭', '照片'],
        strength: 50,
        lastUsed: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000), // 1天前
        usageCount: 2,
        createdAt: new Date(Date.now() - 15 * 24 * 60 * 60 * 1000), // 15天前
        updatedAt: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000)
      }
    ]
    saveToStorage()
  }
})
</script>

<style scoped>
.password-manager {
  @apply space-y-6;
}

.glass-input {
  @apply px-3 py-2 bg-white/10 dark:bg-black/10 border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all;
}

.glass-button {
  @apply px-4 py-2 bg-white/10 dark:bg-black/10 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-white/20 dark:hover:bg-black/20 transition-all focus:outline-none focus:ring-2 focus:ring-primary;
}

.glass-button-primary {
  @apply px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary/90 transition-all focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2;
}

.glass-card {
  @apply bg-white/5 dark:bg-black/5 backdrop-blur-sm border border-gray-300/20 dark:border-gray-600/20 rounded-xl p-6;
}

table {
  @apply min-w-full divide-y divide-gray-200 dark:divide-gray-700;
}

th {
  @apply text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider;
}

td {
  @apply whitespace-nowrap;
}
</style>
