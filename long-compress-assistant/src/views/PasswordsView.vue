<template>
  <div class="h-full flex flex-col">
    <!-- 顶部状态栏 -->
    <div class="mb-6 flex justify-between items-center">
      <div>
        <h1 class="text-2xl font-bold text-gray-900 dark:text-white">密码本管理</h1>
        <p class="text-gray-500 text-sm">安全存储并自动填充您的压缩包密码</p>
      </div>
      <div class="flex gap-3">
        <button 
          v-if="isUnlocked"
          @click="showAddModal = true"
          class="glass-button-primary px-4 py-2 flex items-center"
        >
          <i class="pi pi-plus mr-2"></i> 添加密码
        </button>
        <button 
          @click="isUnlocked ? passwordStore.lock() : (showUnlockModal = true)"
          :class="isUnlocked ? 'glass-button' : 'glass-button-primary'"
          class="px-4 py-2 flex items-center"
        >
          <i :class="isUnlocked ? 'pi pi-lock' : 'pi pi-unlock'" class="mr-2"></i>
          {{ isUnlocked ? '锁定中' : '解锁密码库' }}
        </button>
      </div>
    </div>

    <!-- 未解锁状态 -->
    <div v-if="!isUnlocked" class="flex-1 flex flex-col items-center justify-center p-12">
      <div class="w-24 h-24 bg-primary/10 rounded-full flex items-center justify-center mb-6">
        <i class="pi pi-shield text-4xl text-primary"></i>
      </div>
      <h2 class="text-xl font-semibold mb-2">密码库已锁定</h2>
      <p class="text-gray-500 mb-8 text-center max-w-md">
        您的密码本已使用 AES-256 加密存储在本地。请输入主密码解锁以访问您的密码库。
      </p>
      <button @click="showUnlockModal = true" class="glass-button-primary px-8 py-3 text-lg font-medium">
        立即解锁
      </button>
    </div>

    <!-- 已解锁状态 -->
    <div v-else class="flex-1 flex gap-6 overflow-hidden">
      <!-- 左侧分类 -->
      <div class="w-64 flex flex-col gap-2">
        <div class="glass-card p-2">
          <button 
            v-for="cat in categories" 
            :key="cat.value"
            @click="passwordStore.currentCategory = cat.value"
            :class="passwordStore.currentCategory === cat.value ? 'bg-primary text-white shadow-lg' : 'hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-600 dark:text-gray-400'"
            class="w-full text-left px-4 py-2.5 rounded-xl transition-all flex items-center mb-1"
          >
            <i :class="cat.icon" class="mr-3"></i>
            <span class="font-medium">{{ cat.label }}</span>
            <span class="ml-auto text-xs opacity-60">{{ getCategoryCount(cat.value) }}</span>
          </button>
        </div>

        <div class="glass-card p-4 mt-2">
          <h3 class="text-xs font-bold text-gray-400 uppercase tracking-wider mb-3 px-2">统计</h3>
          <div class="space-y-3 px-2">
            <div class="flex justify-between items-center text-sm">
              <span class="text-gray-500">总数</span>
              <span class="font-mono">{{ entries.length }}</span>
            </div>
            <div class="flex justify-between items-center text-sm">
              <span class="text-gray-500">安全性优</span>
              <span class="text-green-500 font-mono">{{ strongCount }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 右侧列表 -->
      <div class="flex-1 flex flex-col overflow-hidden">
        <!-- 搜索框 -->
        <div class="glass-card p-3 mb-4 flex items-center gap-3">
          <i class="pi pi-search text-gray-400 ml-2"></i>
          <input 
            v-model="passwordStore.searchQuery"
            type="text" 
            placeholder="搜索名称、用户名、标签..."
            class="bg-transparent border-none outline-none flex-1 text-gray-700 dark:text-gray-200"
          />
          <div class="h-6 w-px bg-gray-200 dark:bg-gray-700 mx-2"></div>
          <button class="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg text-gray-500">
            <i class="pi pi-filter"></i>
          </button>
        </div>

        <!-- 密码卡片容器 -->
        <div class="flex-1 overflow-y-auto pr-2 space-y-4">
          <div v-if="filteredEntries.length === 0" class="h-full flex flex-col items-center justify-center opacity-40 py-20">
            <i class="pi pi-inbox text-5xl mb-4"></i>
            <p>没有找到匹配的密码</p>
          </div>
          
          <div 
            v-for="entry in filteredEntries" 
            :key="entry.id"
            class="glass-card p-4 hover:shadow-xl transition-all group relative border-l-4"
            :style="{ borderColor: getStrengthColor(entry.strength) }"
          >
            <div class="flex items-start gap-4">
              <div class="w-12 h-12 rounded-2xl bg-gray-100 dark:bg-gray-800 flex items-center justify-center text-2xl">
                {{ getCategoryIcon(entry.category) }}
              </div>
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 mb-1">
                  <h3 class="font-bold text-lg text-gray-900 dark:text-white truncate">{{ entry.name }}</h3>
                  <button @click="toggleFavorite(entry)" class="text-gray-400 hover:text-yellow-500 transition-colors">
                    <i :class="entry.favorite ? 'pi pi-star-fill text-yellow-500' : 'pi pi-star'"></i>
                  </button>
                </div>
                <div class="flex items-center gap-4 text-sm text-gray-500">
                  <span class="flex items-center"><i class="pi pi-user mr-1.5 text-xs"></i>{{ entry.username || '无用户名' }}</span>
                  <span class="flex items-center font-mono tracking-widest bg-gray-100 dark:bg-gray-800 px-2 py-0.5 rounded">
                    ••••••••
                  </span>
                </div>
              </div>
              
              <div class="flex gap-2">
                <button @click="copyPassword(entry.password)" class="p-2.5 rounded-xl hover:bg-primary/10 hover:text-primary transition-all" title="复制密码">
                  <i class="pi pi-copy"></i>
                </button>
                <button @click="editEntry(entry)" class="p-2.5 rounded-xl hover:bg-gray-100 dark:hover:bg-gray-800 transition-all">
                  <i class="pi pi-pencil"></i>
                </button>
                <button @click="confirmDelete(entry)" class="p-2.5 rounded-xl hover:bg-red-50 dark:hover:bg-red-900/20 hover:text-red-500 transition-all">
                  <i class="pi pi-trash"></i>
                </button>
              </div>
            </div>
            
            <div class="mt-3 flex flex-wrap gap-2">
              <span v-for="tag in entry.tags" :key="tag" class="text-[10px] px-2 py-0.5 bg-gray-100 dark:bg-gray-800 rounded-full text-gray-500">
                #{{ tag }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 解锁模态框 -->
    <div v-if="showUnlockModal" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40 backdrop-blur-sm">
      <div class="glass-card p-8 w-full max-w-md animate-in fade-in zoom-in duration-300">
        <div class="text-center mb-6">
          <div class="inline-flex items-center justify-center w-16 h-16 bg-primary/10 rounded-full mb-4">
            <i class="pi pi-unlock text-2xl text-primary"></i>
          </div>
          <h3 class="text-xl font-bold">验证身份</h3>
          <p class="text-gray-500 text-sm">请输入主密码以解锁加密存储</p>
        </div>
        
        <div class="space-y-4">
          <div class="space-y-1">
            <label class="text-sm font-medium text-gray-600 dark:text-gray-400">主密码</label>
            <input 
              v-model="unlockPassword"
              type="password" 
              class="w-full px-4 py-3 bg-gray-50 dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 outline-none focus:ring-2 focus:ring-primary/50 transition-all"
              placeholder="••••••••"
              @keyup.enter="handleUnlock"
            />
          </div>
          <p v-if="unlockError" class="text-red-500 text-xs flex items-center">
            <i class="pi pi-exclamation-circle mr-1"></i> {{ unlockError }}
          </p>
          <div class="flex gap-3 pt-2">
            <button @click="showUnlockModal = false" class="flex-1 glass-button py-3">取消</button>
            <button 
              @click="handleUnlock" 
              :disabled="isUnlocking"
              class="flex-1 glass-button-primary py-3 font-bold"
            >
              <i v-if="isUnlocking" class="pi pi-spin pi-spinner mr-2"></i>
              解锁
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 添加/编辑模态框 (TSK-201 核心部分) -->
    <div v-if="showAddModal" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40 backdrop-blur-sm">
      <div class="glass-card p-8 w-full max-w-2xl max-h-[90vh] overflow-y-auto">
        <h3 class="text-xl font-bold mb-6">{{ isEditing ? '编辑密码条目' : '添加新密码' }}</h3>
        
        <div class="grid grid-cols-2 gap-6">
          <div class="space-y-4">
            <div class="space-y-1">
              <label class="text-sm font-medium">显示名称 *</label>
              <input v-model="form.name" type="text" class="form-input" placeholder="例如：我的ZIP压缩包密码" />
            </div>
            <div class="space-y-1">
              <label class="text-sm font-medium">用户名</label>
              <input v-model="form.username" type="text" class="form-input" placeholder="可选" />
            </div>
            <div class="space-y-1">
              <label class="text-sm font-medium">密码 *</label>
              <div class="relative">
                <input v-model="form.password" :type="showFormPass ? 'text' : 'password'" class="form-input pr-10" />
                <button @click="showFormPass = !showFormPass" class="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400">
                  <i :class="showFormPass ? 'pi pi-eye-slash' : 'pi pi-eye'"></i>
                </button>
              </div>
            </div>
          </div>
          
          <div class="space-y-4">
            <div class="space-y-1">
              <label class="text-sm font-medium">分类</label>
              <select v-model="form.category" class="form-input">
                <option v-for="cat in categories.slice(1)" :key="cat.value" :value="cat.value">
                  {{ cat.label }}
                </option>
              </select>
            </div>
            <div class="space-y-1">
              <label class="text-sm font-medium">标签 (逗号分隔)</label>
              <input v-model="tagInput" type="text" class="form-input" placeholder="例如：work, sensitive" />
            </div>
            <div class="space-y-1">
              <label class="text-sm font-medium">备注</label>
              <textarea v-model="form.notes" class="form-input h-20 resize-none"></textarea>
            </div>
          </div>
        </div>

        <div class="flex justify-end gap-3 mt-8">
          <button @click="closeAddModal" class="px-6 py-2 glass-button">取消</button>
          <button @click="handleSave" class="px-6 py-2 glass-button-primary font-bold">保存密码</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { usePasswordStore, PasswordCategory, PasswordStrength } from '@/stores/password'
import { useUIStore } from '@/stores/ui'
import { storeToRefs } from 'pinia'

const passwordStore = usePasswordStore()
const uiStore = useUIStore()
const { entries, filteredEntries, isUnlocked } = storeToRefs(passwordStore)

// UI 状态
const showUnlockModal = ref(false)
const showAddModal = ref(false)
const isUnlocking = ref(false)
const unlockPassword = ref('')
const unlockError = ref('')
const isEditing = ref(false)
const showFormPass = ref(false)
const tagInput = ref('')

const categories = [
  { label: '全部密码', value: 'All', icon: 'pi pi-objects-column' },
  { label: '个人', value: PasswordCategory.Personal, icon: 'pi pi-user' },
  { label: '工作', value: PasswordCategory.Work, icon: 'pi pi-briefcase' },
  { label: '金融', value: PasswordCategory.Finance, icon: 'pi pi-wallet' },
  { label: '社交', value: PasswordCategory.Social, icon: 'pi pi-comments' },
  { label: '购物', value: PasswordCategory.Shopping, icon: 'pi pi-shopping-cart' },
  { label: '其他', value: PasswordCategory.Other, icon: 'pi pi-folder' },
]

const form = reactive({
  id: '',
  name: '',
  username: '',
  password: '',
  url: '',
  notes: '',
  category: PasswordCategory.Personal,
  favorite: false
})

onMounted(() => {
  passwordStore.checkUnlockStatus()
})

const getCategoryCount = (catValue: string) => {
  if (catValue === 'All') return entries.value.length
  return entries.value.filter(e => e.category === catValue).length
}

const getCategoryIcon = (cat: PasswordCategory) => {
  const found = categories.find(c => c.value === cat)
  return found ? found.icon.replace('pi pi-', '') : '📁'
}

const getStrengthColor = (strength: PasswordStrength) => {
  switch (strength) {
    case PasswordStrength.VeryStrong: return '#10b981'
    case PasswordStrength.Strong: return '#34d399'
    case PasswordStrength.Medium: return '#fbbf24'
    case PasswordStrength.Weak: return '#f87171'
    default: return '#ef4444'
  }
}

const strongCount = computed(() => 
  entries.value.filter(e => e.strength === PasswordStrength.Strong || e.strength === PasswordStrength.VeryStrong).length
)

const handleUnlock = async () => {
  if (!unlockPassword.value) return
  isUnlocking.value = true
  unlockError.value = ''
  
  const success = await passwordStore.unlock(unlockPassword.value)
  if (success) {
    showUnlockModal.value = false
    unlockPassword.value = ''
    uiStore.showSuccess('密码库已解锁')
  } else {
    unlockError.value = '解锁失败，请检查密码是否正确'
  }
  isUnlocking.value = false
}

const copyPassword = async (text: string) => {
  await navigator.clipboard.writeText(text)
  uiStore.showSuccess('密码已复制到剪贴板')
}

const toggleFavorite = async (entry: any) => {
  await passwordStore.updateEntry(entry.id, { ...entry, favorite: !entry.favorite })
}

const editEntry = (entry: any) => {
  isEditing.value = true
  Object.assign(form, entry)
  tagInput.value = entry.tags.join(', ')
  showAddModal.value = true
}

const confirmDelete = async (entry: any) => {
  if (window.confirm(`确定要删除“${entry.name}”吗？此操作不可撤销。`)) {
    try {
      await passwordStore.deleteEntry(entry.id)
      uiStore.showSuccess('密码已删除')
    } catch (e) {
      uiStore.showError('删除失败')
    }
  }
}

const closeAddModal = () => {
  showAddModal.value = false
  isEditing.value = false
  Object.assign(form, { id: '', name: '', username: '', password: '', url: '', notes: '', category: PasswordCategory.Personal })
  tagInput.value = ''
}

const handleSave = async () => {
  if (!form.name || !form.password) {
    uiStore.showWarning('请填写名称和密码')
    return
  }

  const entryData = {
    ...form,
    tags: tagInput.value.split(',').map(s => s.trim()).filter(Boolean),
    custom_fields: []
  }

  try {
    if (isEditing.value) {
      await passwordStore.updateEntry(form.id, entryData)
      uiStore.showSuccess('密码已更新')
    } else {
      await passwordStore.addEntry(entryData)
      uiStore.showSuccess('新密码已保存')
    }
    closeAddModal()
  } catch (e) {
    uiStore.showError('保存失败')
  }
}
</script>

<style scoped>
.form-input {
  @apply w-full px-4 py-2 bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 outline-none focus:ring-2 focus:ring-primary/50 transition-all;
}
</style>
