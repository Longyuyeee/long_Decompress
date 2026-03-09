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
            :disabled="passwordStore.isLoading"
          >
            <i class="pi pi-plus mr-2"></i>
            添加密码
          </button>
          <button
            @click="importPasswords"
            class="glass-button px-4 py-2"
            :disabled="passwordStore.isLoading"
          >
            <i class="pi pi-upload mr-2"></i>
            导入
          </button>
          <button
            @click="exportPasswords"
            class="glass-button px-4 py-2"
            :disabled="passwordStore.isLoading"
          >
            <i class="pi pi-download mr-2"></i>
            导出
          </button>
        </div>
      </div>
    </div>

    <!-- 错误和成功消�?-->
    <div v-if="passwordStore.error" class="mb-4 p-4 rounded-lg bg-red-500/10 border border-red-500/20">
      <div class="flex items-center">
        <i class="pi pi-exclamation-triangle text-red-500 mr-2"></i>
        <span class="text-red-500">{{ passwordStore.error }}</span>
        <button @click="passwordStore.error = null" class="ml-auto text-red-500 hover:text-red-700">
          <i class="pi pi-times"></i>
        </button>
      </div>
    </div>

    <div v-if="passwordStore.successMessage" class="mb-4 p-4 rounded-lg bg-green-500/10 border border-green-500/20">
      <div class="flex items-center">
        <i class="pi pi-check-circle text-green-500 mr-2"></i>
        <span class="text-green-500">{{ passwordStore.successMessage }}</span>
        <button @click="passwordStore.successMessage = null" class="ml-auto text-green-500 hover:text-green-700">
          <i class="pi pi-times"></i>
        </button>
      </div>
    </div>

    <!-- 搜索和过�?-->
    <div class="mb-6 glass-card p-4">
      <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
        <!-- 搜索�?-->
        <div class="relative">
          <i class="pi pi-search absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400"></i>
          <input
            type="text"
            v-model="searchQuery"
            class="w-full glass-input pl-10"
            placeholder="搜索密码、用户名、URL..."
            @input="handleSearch"
            :disabled="passwordStore.isLoading"
          />
        </div>

        <!-- 标签过滤 -->
        <div>
          <select
            v-model="selectedTag"
            class="w-full glass-input"
            @change="handleTagFilter"
            :disabled="passwordStore.isLoading"
          >
            <option value="">所有标�?/option>
            <option v-for="tag in passwordStore.availableTags" :key="tag" :value="tag">
              {{ tag }}
            </option>
          </select>
        </div>

        <!-- 分类过滤 -->
        <div>
          <select
            v-model="selectedCategory"
            class="w-full glass-input"
            @change="handleCategoryFilter"
            :disabled="passwordStore.isLoading"
          >
            <option value="">所有分�?/option>
            <option v-for="category in categories" :key="category.value" :value="category.value">
              {{ category.label }}
            </option>
          </select>
        </div>

        <!-- 排序选项 -->
        <div>
          <select
            v-model="sortOption"
            class="w-full glass-input"
            @change="handleSort"
            :disabled="passwordStore.isLoading"
          >
            <option v-for="option in sortOptions" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </div>
      </div>

      <!-- 高级过滤 -->
      <div class="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between">
          <button
            @click="showAdvancedFilters = !showAdvancedFilters"
            class="text-sm text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200"
          >
            <i :class="showAdvancedFilters ? 'pi pi-chevron-up' : 'pi pi-chevron-down'" class="mr-1"></i>
            高级过滤
          </button>
          <button
            @click="clearFilters"
            class="text-sm text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200"
            :disabled="passwordStore.isLoading"
          >
            <i class="pi pi-filter-slash mr-1"></i>
            清除过滤
          </button>
        </div>

        <div v-if="showAdvancedFilters" class="mt-4 grid grid-cols-1 md:grid-cols-3 gap-4">
          <!-- 强度过滤 -->
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              密码强度
            </label>
            <div class="flex items-center space-x-2">
              <input
                type="range"
                v-model="strengthMin"
                min="0"
                max="100"
                step="25"
                class="flex-1"
                @change="handleStrengthFilter"
                :disabled="passwordStore.isLoading"
              />
              <span class="text-sm text-gray-600 dark:text-gray-400 w-16">
                {{ strengthLabel(strengthMin) }}
              </span>
            </div>
          </div>

          <!-- 收藏过滤 -->
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              收藏状�?
            </label>
            <select
              v-model="favoriteFilter"
              class="w-full glass-input"
              @change="handleFavoriteFilter"
              :disabled="passwordStore.isLoading"
            >
              <option value="">全部</option>
              <option value="true">已收�?/option>
              <option value="false">未收�?/option>
            </select>
          </div>

          <!-- 归档过滤 -->
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              归档状�?
            </label>
            <select
              v-model="archivedFilter"
              class="w-full glass-input"
              @change="handleArchivedFilter"
              :disabled="passwordStore.isLoading"
            >
              <option value="">全部</option>
              <option value="true">已归�?/option>
              <option value="false">未归�?/option>
            </select>
          </div>
        </div>
      </div>
    </div>

    <!-- 统计信息 -->
    <div class="mb-6 grid grid-cols-1 md:grid-cols-4 gap-4">
      <div class="glass-card p-4">
        <div class="flex items-center">
          <i class="pi pi-lock text-2xl text-primary mr-3"></i>
          <div>
            <p class="text-sm text-gray-600 dark:text-gray-400">总密码数</p>
            <p class="text-2xl font-bold text-gray-900 dark:text-white">
              {{ passwordStore.statistics.total }}
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
              {{ passwordStore.statistics.byStrength.strong + passwordStore.statistics.byStrength.veryStrong }}
            </p>
          </div>
        </div>
      </div>
      <div class="glass-card p-4">
        <div class="flex items-center">
          <i class="pi pi-heart text-2xl text-red-500 mr-3"></i>
          <div>
            <p class="text-sm text-gray-600 dark:text-gray-400">已收�?/p>
            <p class="text-2xl font-bold text-gray-900 dark:text-white">
              {{ passwordStore.statistics.favorite }}
            </p>
          </div>
        </div>
      </div>
      <div class="glass-card p-4">
        <div class="flex items-center">
          <i class="pi pi-tags text-2xl text-green-500 mr-3"></i>
          <div>
            <p class="text-sm text-gray-600 dark:text-gray-400">分类数量</p>
            <p class="text-2xl font-bold text-gray-900 dark:text-white">
              {{ Object.keys(passwordStore.statistics.byCategory).length }}
            </p>
          </div>
        </div>
      </div>
    </div>

    <!-- 密码列表 -->
    <div class="mb-6">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">
          密码列表 ({{ passwordStore.filteredPasswords.length }})
          <span v-if="passwordStore.isLoading" class="ml-2 text-sm text-gray-500">
            <i class="pi pi-spin pi-spinner mr-1"></i>加载�?..
          </span>
        </h2>
        <div class="flex items-center space-x-2">
          <button
            @click="toggleSelectAll"
            class="text-sm text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200"
            :disabled="passwordStore.isLoading"
          >
            {{ isAllSelected ? '取消全�? : '全�? }}
          </button>
          <button
            v-if="passwordStore.selectedPasswords.length > 0"
            @click="deleteSelected"
            class="text-sm text-red-600 hover:text-red-700"
            :disabled="passwordStore.isLoading"
          >
            <i class="pi pi-trash mr-1"></i>
            删除选中 ({{ passwordStore.selectedPasswords.length }})
          </button>
          <button
            @click="passwordStore.hideAllPasswords()"
            class="text-sm text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200"
            :disabled="passwordStore.isLoading"
          >
            <i class="pi pi-eye-slash mr-1"></i>
            隐藏所有密�?
          </button>
        </div>
      </div>

      <!-- 密码表格 -->
      <div class="glass-card overflow-hidden">
        <div class="overflow-x-auto">
          <table class="w-full">
            <thead>
              <tr class="border-b border-gray-200 dark:border-gray-700">
                <th class="py-3 px-4 text-left w-12">
                  <input
                    type="checkbox"
                    v-model="isAllSelected"
                    class="rounded border-gray-300 text-primary focus:ring-primary"
                    :disabled="passwordStore.isLoading"
                  />
                </th>
                <th class="py-3 px-4 text-left text-sm font-medium text-gray-700 dark:text-gray-300">
                  名称
                </th>
                <th class="py-3 px-4 text-left text-sm font-medium text-gray-700 dark:text-gray-300">
                  用户�?密码
                </th>
                <th class="py-3 px-4 text-left text-sm font-medium text-gray-700 dark:text-gray-300">
                  分类/标签
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
                v-for="password in passwordStore.paginatedPasswords"
                :key="password.id"
                class="border-b border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800/50"
              >
                <td class="py-3 px-4">
                  <input
                    type="checkbox"
                    :value="password.id"
                    v-model="passwordStore.selectedPasswords"
                    class="rounded border-gray-300 text-primary focus:ring-primary"
                    :disabled="passwordStore.isLoading"
                  />
                </td>
                <td class="py-3 px-4">
                  <div>
                    <div class="flex items-center">
                      <p class="font-medium text-gray-900 dark:text-white">
                        {{ password.name }}
                      </p>
                      <button
                        v-if="password.favorite"
                        @click="toggleFavorite(password.id)"
                        class="ml-2 text-yellow-500 hover:text-yellow-600"
                        :title="`取消收藏`"
                        :disabled="passwordStore.isLoading"
                      >
                        <i class="pi pi-star-fill"></i>
                      </button>
                      <button
                        v-else
                        @click="toggleFavorite(password.id)"
                        class="ml-2 text-gray-400 hover:text-yellow-500"
                        :title="`收藏`"
                        :disabled="passwordStore.isLoading"
                      >
                        <i class="pi pi-star"></i>
                      </button>
                    </div>
                    <p class="text-sm text-gray-500 dark:text-gray-400 mt-1 truncate max-w-xs">
                      {{ password.url || '无URL' }}
                    </p>
                    <p class="text-xs text-gray-400 dark:text-gray-500 mt-1">
                      {{ password.notes || '无备�? }}
                    </p>
                  </div>
                </td>
                <td class="py-3 px-4">
                  <div class="space-y-2">
                    <div class="flex items-center">
                      <span class="text-sm text-gray-600 dark:text-gray-400">用户:</span>
                      <span class="ml-2 text-sm font-medium">{{ password.username || '�? }}</span>
                    </div>
                    <div class="flex items-center">
                      <span class="font-mono text-sm">
                        {{ passwordStore.showPassword(password.id) ? password.password : '•••••••�? }}
                      </span>
                      <button
                        @click="passwordStore.togglePasswordVisibility(password.id)"
                        class="ml-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
                        :title="passwordStore.showPassword(password.id) ? '隐藏密码' : '显示密码'"
                        :disabled="passwordStore.isLoading"
                      >
                        <i :class="passwordStore.showPassword(password.id) ? 'pi pi-eye-slash' : 'pi pi-eye'"></i>
                      </button>
                      <button
                        @click="copyPassword(password.password)"
                        class="ml-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
                        :title="`复制密码`"
                        :disabled="passwordStore.isLoading"
                      >
                        <i class="pi pi-copy"></i>
                      </button>
                    </div>
                  </div>
                </td>
                <td class="py-3 px-4">
                  <div class="space-y-2">
                    <span class="px-2 py-1 text-xs rounded-full bg-primary/10 text-primary">
                      {{ categoryLabel(password.category) }}
                    </span>
                    <div class="flex flex-wrap gap-1 mt-2">
                      <span
                        v-for="tag in password.tags"
                        :key="tag"
                        class="px-2 py-1 text-xs rounded-full bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300"
                      >
                        {{ tag }}
                      </span>
                      <span v-if="password.tags.length === 0" class="text-gray-400 text-xs">
                        无标�?
                      </span>
                    </div>
                  </div>
                </td>
                <td class="py-3 px-4">
                  <div class="flex items-center">
                    <div class="w-24 h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                      <div
                        class="h-full rounded-full"
                        :class="passwordStore.getStrengthColor(password.strength)"
                        :style="{ width: password.strength + '%' }"
                      ></div>
                    </div>
                    <span class="ml-2 text-sm" :class="passwordStore.getStrengthTextColor(password.strength)">
                      {{ passwordStore.getStrengthLabel(password.strength) }}
                    </span>
                  </div>
                </td>
                <td class="py-3 px-4">
                  <div class="text-sm text-gray-600 dark:text-gray-400">
                    {{ passwordStore.formatTime(password.lastUsed) }}
                  </div>
                  <div class="text-xs text-gray-500 dark:text-gray-500">
                    使用次数: {{ password.usageCount }}
                  </div>
                  <div v-if="password.expiresAt" class="text-xs mt-1" :class="isExpired(password.expiresAt) ? 'text-red-500' : 'text-yellow-500'">
                    {{ isExpired(password.expiresAt) ? '已过�? : `过期: ${formatDate(password.expiresAt)}` }}
                  </div>
                </td>
                <td class="py-3 px-4">
                  <div class="flex items-center space-x-2">
                    <button
                      @click="usePassword(password)"
                      class="text-green-600 hover:text-green-700"
                      :title="`使用此密码`"
                      :disabled="passwordStore.isLoading"
                    >
                      <i class="pi pi-play"></i>
                    </button>
                    <button
                      @click="editPassword(password)"
                      class="text-blue-600 hover:text-blue-700"
                      :title="`编辑`"
                      :disabled="passwordStore.isLoading"
                    >
                      <i class="pi pi-pencil"></i>
                    </button>
                    <button
                      v-if="!password.archived"
                      @click="archivePassword(password.id)"
                      class="text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-300"
                      :title="`归档`"
                      :disabled="passwordStore.isLoading"
                    >
                      <i class="pi pi-archive"></i>
                    </button>
                    <button
                      @click="deletePassword(password.id)"
                      class="text-red-600 hover:text-red-700"
                      :title="`删除`"
                      :disabled="passwordStore.isLoading"
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
        <div v-if="passwordStore.filteredPasswords.length === 0 && !passwordStore.isLoading" class="py-12 text-center">
          <i class="pi pi-lock text-4xl text-gray-300 dark:text-gray-600 mb-4"></i>
          <p class="text-gray-500 dark:text-gray-400">暂无密码记录</p>
          <p class="text-sm text-gray-400 dark:text-gray-500 mt-2">
            点击"添加密码"按钮开始管理您的密�?
          </p>
        </div>

        <!-- 加载状�?-->
        <div v-if="passwordStore.isLoading && passwordStore.filteredPasswords.length === 0" class="py-12 text-center">
          <i class="pi pi-spin pi-spinner text-4xl text-gray-300 dark:text-gray-600 mb-4"></i>
          <p class="text-gray-500 dark:text-gray-400">加载�?..</p>
        </div>
      </div>
    </div>

    <!-- 分页 -->
    <div v-if="passwordStore.filteredPasswords.length > 0" class="flex items-center justify-between">
      <div class="text-sm text-gray-600 dark:text-gray-400">
        显示 {{ startIndex + 1 }}-{{ endIndex }} 条，�?{{ passwordStore.filteredPasswords.length }} �?
      </div>
      <div class="flex items-center space-x-2">
        <button
          @click="prevPage"
          :disabled="passwordStore.currentPage === 1 || passwordStore.isLoading"
          class="glass-button px-3 py-1"
          :class="{ 'opacity-50 cursor-not-allowed': passwordStore.currentPage === 1 || passwordStore.isLoading }"
        >
          <i class="pi pi-chevron-left"></i>
        </button>
        <span class="text-sm text-gray-700 dark:text-gray-300">
          �?{{ passwordStore.currentPage }} �?/ �?{{ passwordStore.totalPages }} �?
        </span>
        <button
          @click="nextPage"
          :disabled="passwordStore.currentPage === passwordStore.totalPages || passwordStore.isLoading"
          class="glass-button px-3 py-1"
          :class="{ 'opacity-50 cursor-not-allowed': passwordStore.currentPage === passwordStore.totalPages || passwordStore.isLoading }"
        >
          <i class="pi pi-chevron-right"></i>
        </button>
      </div>
    </div>

    <!-- 添加/编辑密码模态框 -->
    <PasswordModal
      v-if="showAddPasswordModal"
      :password="editingPassword"
      @save="handleSavePassword"
      @close="closeModal"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { usePasswordStore, type PasswordEntry, type PasswordCategory, type AddPasswordRequest, type UpdatePasswordRequest } from '@/stores'
import PasswordModal from './PasswordModal.vue'

const passwordStore = usePasswordStore()

// 状�?
const searchQuery = ref('')
const selectedTag = ref('')
const selectedCategory = ref('')
const sortOption = ref('createdAt')
const showAdvancedFilters = ref(false)
const strengthMin = ref(0)
const favoriteFilter = ref('')
const archivedFilter = ref('')
const showAddPasswordModal = ref(false)
const editingPassword = ref<PasswordEntry | null>(null)

// 分类选项
const categories = [
  { value: PasswordCategory.Personal, label: '个人' },
  { value: PasswordCategory.Work, label: '工作' },
  { value: PasswordCategory.Finance, label: '财务' },
  { value: PasswordCategory.Social, label: '社交' },
  { value: PasswordCategory.Shopping, label: '购物' },
  { value: PasswordCategory.Entertainment, label: '娱乐' },
  { value: PasswordCategory.Education, label: '教育' },
  { value: PasswordCategory.Travel, label: '旅行' },
  { value: PasswordCategory.Health, label: '健康' },
  { value: PasswordCategory.Other, label: '其他' }
]

// 排序选项
const sortOptions = [
  { value: 'name', label: '名称' },
  { value: 'category', label: '分类' },
  { value: 'strength', label: '强度' },
  { value: 'lastUsed', label: '最后使�? },
  { value: 'createdAt', label: '添加时间' },
  { value: 'updatedAt', label: '更新时间' }
]

// 计算属�?
const isAllSelected = computed({
  get: () => passwordStore.isAllSelected,
  set: (value) => {
    if (value) {
      passwordStore.selectedPasswords = passwordStore.filteredPasswords.map(p => p.id)
    } else {
      passwordStore.selectedPasswords = []
    }
  }
})

const startIndex = computed(() => {
  return (passwordStore.currentPage - 1) * passwordStore.pageSize
})

const endIndex = computed(() => {
  return Math.min(startIndex.value + passwordStore.pageSize, passwordStore.filteredPasswords.length)
})

// 方法
const handleSearch = () => {
  passwordStore.setSearchFilters({
    query: searchQuery.value || undefined
  })
}

const handleTagFilter = () => {
  passwordStore.setSearchFilters({
    tags: selectedTag.value ? [selectedTag.value] : undefined
  })
}

const handleCategoryFilter = () => {
  passwordStore.setSearchFilters({
    category: selectedCategory.value as PasswordCategory || undefined
  })
}

const handleSort = () => {
  passwordStore.setSort(sortOption.value as any, true)
}

const handleStrengthFilter = () => {
  passwordStore.setSearchFilters({
    strengthMin: strengthMin.value > 0 ? strengthMin.value : undefined
  })
}

const handleFavoriteFilter = () => {
  passwordStore.setSearchFilters({
    favorite: favoriteFilter.value === '' ? undefined : favoriteFilter.value === 'true'
  })
}

const handleArchivedFilter = () => {
  passwordStore.setSearchFilters({
    archived: archivedFilter.value === '' ? undefined : archivedFilter.value === 'true'
  })
}

const clearFilters = () => {
  searchQuery.value = ''
  selectedTag.value = ''
  selectedCategory.value = ''
  strengthMin.value = 0
  favoriteFilter.value = ''
  archivedFilter.value = ''
  passwordStore.clearSearchFilters()
}

const toggleSelectAll = () => {
  isAllSelected.value = !isAllSelected.value
}

const deleteSelected = async () => {
  if (passwordStore.selectedPasswords.length === 0) return

  if (confirm(`确定要删除选中�?${passwordStore.selectedPasswords.length} 个密码吗？`)) {
    try {
      await passwordStore.deleteSelectedPasswords()
    } catch (err) {
      console.error('删除失败:', err)
    }
  }
}

const editPassword = (password: PasswordEntry) => {
  editingPassword.value = password
  showAddPasswordModal.value = true
}

const usePassword = async (password: PasswordEntry) => {
  try {
    const passwordText = await passwordStore.usePassword(password.id)
    if (passwordText) {
      emit('password-selected', passwordText)
    }
  } catch (err) {
    console.error('使用密码失败:', err)
  }
}

const deletePassword = async (id: string) => {
  if (confirm('确定要删除这个密码吗�?)) {
    try {
      await passwordStore.deletePassword(id)
    } catch (err) {
      console.error('删除失败:', err)
    }
  }
}

const archivePassword = async (id: string) => {
  if (confirm('确定要归档这个密码吗？归档后密码将不再显示在默认列表中�?)) {
    try {
      await passwordStore.archivePassword(id)
    } catch (err) {
      console.error('归档失败:', err)
    }
  }
}

const toggleFavorite = async (id: string) => {
  try {
    await passwordStore.toggleFavorite(id)
  } catch (err) {
    console.error('切换收藏状态失�?', err)
  }
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

const importPasswords = () => {
  // TODO: 实现导入功能
  alert('导入功能开发中...')
}

const exportPasswords = () => {
  // TODO: 实现导出功能
  alert('导出功能开发中...')
}

const handleSavePassword = async (passwordData: any) => {
  try {
    if (editingPassword.value) {
      // 更新密码
      const request: UpdatePasswordRequest = {
        id: editingPassword.value.id,
        name: passwordData.name,
        username: passwordData.username,
        password: passwordData.password,
        url: passwordData.url,
        notes: passwordData.notes,
        tags: passwordData.tags,
        category: passwordData.category,
        expiresAt: passwordData.expiresAt,
        favorite: passwordData.favorite,
        customFields: passwordData.customFields
      }
      await passwordStore.updatePassword(request)
    } else {
      // 添加新密�?
      const request: AddPasswordRequest = {
        name: passwordData.name,
        username: passwordData.username,
        password: passwordData.password,
        url: passwordData.url,
        notes: passwordData.notes,
        tags: passwordData.tags,
        category: passwordData.category,
        expiresAt: passwordData.expiresAt,
        customFields: passwordData.customFields || []
      }
      await passwordStore.addPassword(request)
    }
    closeModal()
  } catch (err) {
    console.error('保存密码失败:', err)
  }
}

const closeModal = () => {
  showAddPasswordModal.value = false
  editingPassword.value = null
}

const prevPage = () => {
  passwordStore.prevPage()
}

const nextPage = () => {
  passwordStore.nextPage()
}

const categoryLabel = (category: PasswordCategory): string => {
  const found = categories.find(c => c.value === category)
  return found ? found.label : '其他'
}

const strengthLabel = (strength: number): string => {
  if (strength < 25) return '非常�?
  if (strength < 50) return '�?
  if (strength < 75) return '中等'
  if (strength < 90) return '�?
  return '非常�?
}

const isExpired = (date: Date): boolean => {
  return new Date() > date
}

const formatDate = (date: Date): string => {
  return date.toLocaleDateString()
}

// 事件
const emit = defineEmits<{
  (e: 'password-selected', password: string): void
}>()

// 初始�?
onMounted(() => {
  passwordStore.loadPasswords()
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

input[type="range"] {
  @apply appearance-none h-2 bg-gray-200 dark:bg-gray-700 rounded-lg;
}

input[type="range"]::-webkit-slider-thumb {
  @apply appearance-none w-4 h-4 rounded-full bg-primary cursor-pointer;
}

input[type="range"]::-moz-range-thumb {
  @apply w-4 h-4 rounded-full bg-primary cursor-pointer border-0;
}
</style>
