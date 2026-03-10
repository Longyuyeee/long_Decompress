<template>
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
    <div class="glass-card max-w-2xl w-full max-h-[90vh] overflow-y-auto">
      <div class="flex items-center justify-between mb-6 sticky top-0 bg-white/5 dark:bg-black/5 backdrop-blur-sm py-4">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
          {{ password ? '编辑密码' : '添加新密码' }}
        </h3>
        <button
          @click="$emit('close')"
          class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
          :disabled="isLoading"
        >
          <i class="pi pi-times"></i>
        </button>
      </div>

      <!-- 错误和成功消息 -->
      <div v-if="error" class="mb-4 p-4 rounded-lg bg-red-500/10 border border-red-500/20">
        <div class="flex items-center">
          <i class="pi pi-exclamation-triangle text-red-500 mr-2"></i>
          <span class="text-red-500">{{ error }}</span>
        </div>
      </div>

      <form @submit.prevent="handleSubmit" class="space-y-6">
        <!-- 基本信息 -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              名称 <span class="text-red-500">*</span>
            </label>
            <input
              type="text"
              v-model="formData.name"
              class="w-full glass-input"
              placeholder="为密码起个名称"
              required
              :disabled="isLoading"
            />
          </div>

          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              用户名
            </label>
            <input
              type="text"
              v-model="formData.username"
              class="w-full glass-input"
              placeholder="用户名或邮箱"
              :disabled="isLoading"
            />
          </div>
        </div>

        <!-- 密码 -->
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            密码 <span class="text-red-500">*</span>
          </label>
          <div class="relative">
            <input
              :type="showPassword ? 'text' : 'password'"
              v-model="formData.password"
              class="w-full glass-input pr-10"
              placeholder="输入密码"
              required
              @input="handleStrengthCheck"
              :disabled="isLoading"
            />
            <button
              type="button"
              @click="showPassword = !showPassword"
              class="absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-500"
              :disabled="isLoading"
            >
              <i :class="showPassword ? 'pi pi-eye-slash' : 'pi pi-eye'"></i>
            </button>
          </div>

          <!-- 密码强度指示 -->
          <div v-if="formData.password" class="mt-3">
            <div class="flex items-center justify-between text-sm mb-1">
              <span>密码强度</span>
              <span :class="strengthTextColor">
                {{ strengthLabel }} ({{ passwordStrength }}/100)
              </span>
            </div>
            <div class="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
              <div
                class="h-full rounded-full transition-all duration-300"
                :class="strengthColor"
                :style="{ width: passwordStrength + '%' }"
              ></div>
            </div>
          </div>
        </div>

        <!-- URL和分类 -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              URL
            </label>
            <input
              type="url"
              v-model="formData.url"
              class="w-full glass-input"
              placeholder="https://example.com"
              :disabled="isLoading"
            />
          </div>

          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              分类 <span class="text-red-500">*</span>
            </label>
            <select
              v-model="formData.category"
              class="w-full glass-input"
              required
              :disabled="isLoading"
            >
              <option v-for="category in categories" :key="category.value" :value="category.value">
                {{ category.label }}
              </option>
            </select>
          </div>
        </div>

        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            备注
          </label>
          <textarea
            v-model="formData.notes"
            class="w-full glass-input"
            rows="3"
            placeholder="添加备注信息"
            :disabled="isLoading"
          ></textarea>
        </div>

        <!-- 表单操作 -->
        <div class="pt-6 border-t border-gray-200 dark:border-gray-700">
          <div class="flex justify-end space-x-3">
            <button
              type="button"
              @click="$emit('close')"
              class="glass-button px-6 py-2"
              :disabled="isLoading"
            >
              取消
            </button>
            <button
              type="submit"
              class="glass-button-primary px-6 py-2"
              :disabled="isLoading || !formData.name || !formData.password"
            >
              <i v-if="isLoading" class="pi pi-spin pi-spinner mr-2"></i>
              {{ password ? '更新' : '保存' }}
            </button>
          </div>
        </div>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { usePasswordStore, CustomFieldType, PasswordCategory } from '@/stores'
import type { PasswordEntry } from '@/stores'

interface Props {
  password?: PasswordEntry | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'save', data: any): void
  (e: 'close'): void
}>()

const passwordStore = usePasswordStore()
const isLoading = ref(false)
const error = ref<string | null>(null)
const showPassword = ref(false)
const passwordStrength = ref(0)

const formData = ref({
  name: '',
  username: '',
  password: '',
  url: '',
  notes: '',
  tags: [] as string[],
  category: PasswordCategory.Personal,
  expires_at: '',
  favorite: false,
  custom_fields: [] as any[]
})

const categories = [
  { value: PasswordCategory.Personal, label: '个人' },
  { value: PasswordCategory.Work, label: '工作' },
  { value: PasswordCategory.Other, label: '其他' }
]

const strengthLabel = computed(() => {
  if (passwordStrength.value < 25) return '弱'
  if (passwordStrength.value < 75) return '中'
  return '强'
})

const strengthColor = computed(() => {
  if (passwordStrength.value < 25) return 'bg-red-500'
  if (passwordStrength.value < 75) return 'bg-yellow-500'
  return 'bg-green-500'
})

const strengthTextColor = computed(() => {
  if (passwordStrength.value < 25) return 'text-red-500'
  if (passwordStrength.value < 75) return 'text-yellow-500'
  return 'text-green-500'
})

const handleStrengthCheck = () => {
  passwordStrength.value = Math.min(100, formData.value.password.length * 10)
}

const handleSubmit = async () => {
  isLoading.value = true
  try {
    emit('save', { ...formData.value })
  } catch (err) {
    error.value = '保存失败'
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  if (props.password) {
    formData.value = {
      name: props.password.name,
      username: props.password.username || '',
      password: props.password.password,
      url: props.password.url || '',
      notes: props.password.notes || '',
      tags: [...props.password.tags],
      category: props.password.category,
      expires_at: props.password.expires_at ? props.password.expires_at.split('T')[0] : '',
      favorite: props.password.favorite,
      custom_fields: props.password.custom_fields.map(f => ({ ...f }))
    }
    handleStrengthCheck()
  }
})

watch(() => formData.value.password, handleStrengthCheck)
</script>

<style scoped>
.glass-input {
  @apply px-3 py-2 bg-white/10 dark:bg-black/10 border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary transition-all;
}
.glass-button {
  @apply px-4 py-2 bg-white/10 border border-gray-300 rounded-lg hover:bg-white/20 transition-all;
}
.glass-button-primary {
  @apply px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary/90 transition-all;
}
.glass-card {
  @apply bg-white/5 backdrop-blur-md border border-white/10 rounded-xl p-6;
}
</style>
