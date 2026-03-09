<template>
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
    <div class="glass-card max-w-2xl w-full max-h-[90vh] overflow-y-auto">
      <div class="flex items-center justify-between mb-6 sticky top-0 bg-white/5 dark:bg-black/5 backdrop-blur-sm py-4">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
          {{ password ? '编辑密码' : '添加新密�? }}
        </h3>
        <button
          @click="$emit('close')"
          class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
          :disabled="isLoading"
        >
          <i class="pi pi-times"></i>
        </button>
      </div>

      <!-- 错误和成功消�?-->
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
              placeholder="为密码起个名�?
              required
              :disabled="isLoading"
            />
          </div>

          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              用户�?
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
              @input="assessPasswordStrength"
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

          <!-- 密码强度指示�?-->
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

            <!-- 密码评估详情 -->
            <div v-if="passwordAssessment" class="mt-3 space-y-2">
              <div class="grid grid-cols-2 gap-2 text-xs">
                <div class="text-gray-600 dark:text-gray-400">
                  <i class="pi pi-shield mr-1"></i> 强度等级: {{ strengthLabel }}
                </div>
                <div class="text-gray-600 dark:text-gray-400">
                  <i class="pi pi-chart-line mr-1"></i> 评分: {{ passwordStrength }}/100
                </div>
                <div class="text-gray-600 dark:text-gray-400">
                  <i class="pi pi-bolt mr-1"></i> 熵�? {{ passwordAssessment.entropyBits.toFixed(1) }} bits
                </div>
                <div class="text-gray-600 dark:text-gray-400">
                  <i class="pi pi-clock mr-1"></i> 破解时间: {{ passwordAssessment.crackTimeDisplay }}
                </div>
              </div>

              <!-- 改进建议 -->
              <div v-if="passwordAssessment.recommendations.length > 0" class="mt-2">
                <p class="text-xs text-gray-600 dark:text-gray-400 mb-1">
                  <i class="pi pi-lightbulb mr-1"></i> 改进建议:
                </p>
                <ul class="text-xs text-gray-500 dark:text-gray-500 space-y-1">
                  <li v-for="rec in passwordAssessment.recommendations" :key="rec" class="flex items-center">
                    <i class="pi pi-check-circle mr-1 text-green-500"></i> {{ rec }}
                  </li>
                </ul>
              </div>
            </div>
            <div v-else class="mt-3 grid grid-cols-2 gap-2 text-xs">
              <div class="text-gray-600 dark:text-gray-400">
                <i class="pi pi-shield mr-1"></i> 强度等级: {{ strengthLabel }}
              </div>
              <div class="text-gray-600 dark:text-gray-400">
                <i class="pi pi-chart-line mr-1"></i> 评分: {{ passwordStrength }}/100
              </div>
            </div>

            <div v-if="passwordIssues.length > 0" class="mt-3">
              <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">
                <i class="pi pi-exclamation-triangle mr-1"></i> 需要改进：
              </p>
              <ul class="text-xs text-gray-500 dark:text-gray-500 space-y-1">
                <li v-for="issue in passwordIssues" :key="issue" class="flex items-center">
                  <i class="pi pi-info-circle mr-1"></i> {{ issue }}
                </li>
              </ul>
            </div>

            <!-- 密码安全提示 -->
            <div class="mt-3 p-2 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
              <p class="text-xs text-blue-700 dark:text-blue-300">
                <i class="pi pi-lightbulb mr-1"></i>
                提示：使用至�?2位包含大小写字母、数字和特殊字符的组合密码更安全
              </p>
            </div>
          </div>
        </div>

        <!-- URL和备�?-->
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

        <!-- 标签管理 -->
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            标签
          </label>
          <div class="flex flex-wrap gap-2 mb-3">
            <span
              v-for="tag in formData.tags"
              :key="tag"
              class="px-3 py-1 text-sm rounded-full bg-primary/10 text-primary flex items-center"
            >
              {{ tag }}
              <button
                type="button"
                @click="removeTag(tag)"
                class="ml-1 text-primary/70 hover:text-primary"
                :disabled="isLoading"
              >
                <i class="pi pi-times text-xs"></i>
              </button>
            </span>
            <span v-if="formData.tags.length === 0" class="text-gray-400 text-sm">
              暂无标签
            </span>
          </div>
          <div class="flex space-x-2">
            <input
              type="text"
              v-model="tagInput"
              class="flex-1 glass-input"
              placeholder="输入标签，按回车添加"
              @keyup.enter="addTag"
              :disabled="isLoading"
            />
            <button
              type="button"
              @click="addTag"
              class="glass-button px-4"
              :disabled="isLoading"
            >
              添加
            </button>
          </div>
        </div>

        <!-- 过期时间和收�?-->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              过期时间
            </label>
            <input
              type="date"
              v-model="formData.expiresAt"
              class="w-full glass-input"
              :min="today"
              :disabled="isLoading"
            />
            <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
              留空表示永不过期
            </p>
          </div>

          <div class="flex items-center">
            <label class="flex items-center cursor-pointer">
              <input
                type="checkbox"
                v-model="formData.favorite"
                class="mr-3 rounded border-gray-300 text-primary focus:ring-primary"
                :disabled="isLoading"
              />
              <span class="text-gray-700 dark:text-gray-300">添加到收�?/span>
            </label>
          </div>
        </div>

        <!-- 自定义字�?-->
        <div>
          <div class="flex items-center justify-between mb-3">
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
              自定义字�?
            </label>
            <button
              type="button"
              @click="addCustomField"
              class="text-sm text-primary hover:text-primary/80"
              :disabled="isLoading"
            >
              <i class="pi pi-plus mr-1"></i>添加字段
            </button>
          </div>

          <div v-if="formData.customFields.length === 0" class="text-center py-4 text-gray-400">
            暂无自定义字�?
          </div>

          <div v-else class="space-y-3">
            <div
              v-for="(field, index) in formData.customFields"
              :key="index"
              class="glass-card p-4"
            >
              <div class="flex items-center justify-between mb-3">
                <input
                  type="text"
                  v-model="field.name"
                  class="flex-1 glass-input mr-3"
                  placeholder="字段名称"
                  :disabled="isLoading"
                />
                <select
                  v-model="field.fieldType"
                  class="glass-input w-32"
                  :disabled="isLoading"
                >
                  <option v-for="type in customFieldTypes" :key="type.value" :value="type.value">
                    {{ type.label }}
                  </option>
                </select>
              </div>

              <div class="relative">
                <input
                  :type="getCustomFieldInputType(field)"
                  v-model="field.value"
                  class="w-full glass-input pr-10"
                  :placeholder="getCustomFieldPlaceholder(field)"
                  :disabled="isLoading"
                />
                <button
                  v-if="field.fieldType === 'Password'"
                  type="button"
                  @click="toggleCustomFieldVisibility(index)"
                  class="absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-500"
                  :disabled="isLoading"
                >
                  <i :class="customFieldVisibility[index] ? 'pi pi-eye-slash' : 'pi pi-eye'"></i>
                </button>
              </div>

              <div class="flex items-center justify-between mt-3">
                <label class="flex items-center">
                  <input
                    type="checkbox"
                    v-model="field.sensitive"
                    class="mr-2 rounded border-gray-300 text-primary focus:ring-primary"
                    :disabled="isLoading"
                  />
                  <span class="text-sm text-gray-600 dark:text-gray-400">敏感信息</span>
                </label>
                <button
                  type="button"
                  @click="removeCustomField(index)"
                  class="text-red-500 hover:text-red-700 text-sm"
                  :disabled="isLoading"
                >
                  <i class="pi pi-trash mr-1"></i>删除
                </button>
              </div>
            </div>
          </div>
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
              :disabled="isLoading || !formData.name || !formData.password || !formData.category"
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
import { usePasswordStore } from '@/stores'
import type { PasswordEntry, PasswordCategory, CustomField, CustomFieldType, PasswordStrengthAssessment } from '@/stores'

interface Props {
  password?: PasswordEntry | null
}

interface Emits {
  (e: 'save', data: any): void
  (e: 'close'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// Store
const passwordStore = usePasswordStore()

// 状�?
const isLoading = ref(false)
const error = ref<string | null>(null)
const showPassword = ref(false)
const tagInput = ref('')
const passwordStrength = ref(0)
const passwordIssues = ref<string[]>([])
const customFieldVisibility = ref<Record<number, boolean>>({})
const passwordAssessment = ref<PasswordStrengthAssessment | null>(null)

// 表单数据
const formData = ref({
  name: '',
  username: '',
  password: '',
  url: '',
  notes: '',
  tags: [] as string[],
  category: PasswordCategory.Personal,
  expiresAt: '',
  favorite: false,
  customFields: [] as Array<{
    name: string
    value: string
    fieldType: CustomFieldType
    sensitive: boolean
  }>
})

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

// 自定义字段类型选项
const customFieldTypes = [
  { value: CustomFieldType.Text, label: '文本' },
  { value: CustomFieldType.Password, label: '密码' },
  { value: CustomFieldType.Email, label: '邮箱' },
  { value: CustomFieldType.Url, label: '网址' },
  { value: CustomFieldType.Phone, label: '电话' },
  { value: CustomFieldType.Date, label: '日期' },
  { value: CustomFieldType.Number, label: '数字' },
  { value: CustomFieldType.MultilineText, label: '多行文本' }
]

// 计算属�?
const today = computed(() => {
  return new Date().toISOString().split('T')[0]
})

const strengthLabel = computed(() => {
  if (passwordStrength.value < 25) return '非常�?
  if (passwordStrength.value < 50) return '�?
  if (passwordStrength.value < 75) return '中等'
  if (passwordStrength.value < 90) return '�?
  return '非常�?
})

const strengthColor = computed(() => {
  if (passwordStrength.value < 25) return 'bg-red-500'
  if (passwordStrength.value < 50) return 'bg-orange-500'
  if (passwordStrength.value < 75) return 'bg-yellow-500'
  if (passwordStrength.value < 90) return 'bg-green-500'
  return 'bg-green-600'
})

const strengthTextColor = computed(() => {
  if (passwordStrength.value < 25) return 'text-red-500'
  if (passwordStrength.value < 50) return 'text-orange-500'
  if (passwordStrength.value < 75) return 'text-yellow-500'
  if (passwordStrength.value < 90) return 'text-green-500'
  return 'text-green-600'
})

// 方法
const assessPasswordStrength = async () => {
  if (!formData.value.password) {
    passwordStrength.value = 0
    passwordIssues.value = []
    passwordAssessment.value = null
    return
  }

  try {
    // 使用store中的方法调用后端密码强度评估服务
    const assessment = await passwordStore.assessPasswordStrength(formData.value.password)

    // 保存完整的评估结�?
    passwordAssessment.value = assessment

    // 更新UI状�?
    passwordStrength.value = assessment.score
    passwordIssues.value = assessment.issues.map(issue => issue.description)

  } catch (error) {
    console.error('密码强度评估失败:', error)
    // 如果后端调用失败，回退到简单的本地评估
    fallbackPasswordStrengthAssessment()
    passwordAssessment.value = null
  }
}

// 回退的本地密码强度评�?
const fallbackPasswordStrengthAssessment = () => {
  let strength = 0
  const issues: string[] = []

  // 长度检�?
  if (formData.value.password.length >= 8) strength += 25
  else issues.push('密码长度至少8�?)

  // 大小写检�?
  if (/[a-z]/.test(formData.value.password) && /[A-Z]/.test(formData.value.password)) strength += 25
  else issues.push('包含大小写字�?)

  // 数字检�?
  if (/\d/.test(formData.value.password)) strength += 25
  else issues.push('包含数字')

  // 特殊字符检�?
  if (/[^a-zA-Z0-9]/.test(formData.value.password)) strength += 25
  else issues.push('包含特殊字符')

  passwordStrength.value = strength
  passwordIssues.value = issues
}

const addTag = () => {
  const tag = tagInput.value.trim()
  if (tag && !formData.value.tags.includes(tag)) {
    formData.value.tags.push(tag)
    tagInput.value = ''
  }
}

const removeTag = (tag: string) => {
  formData.value.tags = formData.value.tags.filter(t => t !== tag)
}

const addCustomField = () => {
  formData.value.customFields.push({
    name: '',
    value: '',
    fieldType: CustomFieldType.Text,
    sensitive: false
  })
}

const removeCustomField = (index: number) => {
  formData.value.customFields.splice(index, 1)
  // 清理可见性状�?
  const newVisibility = { ...customFieldVisibility.value }
  delete newVisibility[index]
  customFieldVisibility.value = newVisibility
}

const toggleCustomFieldVisibility = (index: number) => {
  customFieldVisibility.value = {
    ...customFieldVisibility.value,
    [index]: !customFieldVisibility.value[index]
  }
}

const getCustomFieldInputType = (field: any) => {
  if (field.fieldType === CustomFieldType.Password) {
    return customFieldVisibility.value[formData.value.customFields.indexOf(field)] ? 'text' : 'password'
  }
  if (field.fieldType === CustomFieldType.Email) return 'email'
  if (field.fieldType === CustomFieldType.Url) return 'url'
  if (field.fieldType === CustomFieldType.Number) return 'number'
  if (field.fieldType === CustomFieldType.Date) return 'date'
  return 'text'
}

const getCustomFieldPlaceholder = (field: any) => {
  switch (field.fieldType) {
    case CustomFieldType.Email: return 'user@example.com'
    case CustomFieldType.Url: return 'https://example.com'
    case CustomFieldType.Phone: return '+86 13800138000'
    case CustomFieldType.Date: return 'YYYY-MM-DD'
    case CustomFieldType.Number: return '123'
    case CustomFieldType.Password: return '输入密码'
    default: return '输入�?
  }
}

const handleSubmit = async () => {
  isLoading.value = true
  error.value = null

  try {
    // 准备数据
    const data = {
      name: formData.value.name,
      username: formData.value.username || undefined,
      password: formData.value.password,
      url: formData.value.url || undefined,
      notes: formData.value.notes || undefined,
      tags: formData.value.tags,
      category: formData.value.category,
      expiresAt: formData.value.expiresAt ? new Date(formData.value.expiresAt) : undefined,
      favorite: formData.value.favorite,
      customFields: formData.value.customFields.map(field => ({
        name: field.name,
        value: field.value,
        fieldType: field.fieldType,
        sensitive: field.sensitive
      }))
    }

    // 验证必填字段
    if (!data.name || !data.password || !data.category) {
      throw new Error('请填写所有必填字�?)
    }

    // 发送保存事�?
    emit('save', data)

  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    console.error('表单提交失败:', err)
  } finally {
    isLoading.value = false
  }
}

// 初始�?
onMounted(() => {
  if (props.password) {
    // 编辑模式：填充表单数�?
    formData.value = {
      name: props.password.name,
      username: props.password.username || '',
      password: props.password.password,
      url: props.password.url || '',
      notes: props.password.notes || '',
      tags: [...props.password.tags],
      category: props.password.category,
      expiresAt: props.password.expiresAt ? props.password.expiresAt.toISOString().split('T')[0] : '',
      favorite: props.password.favorite,
      customFields: props.password.customFields.map(field => ({
        name: field.name,
        value: field.value,
        fieldType: field.fieldType,
        sensitive: field.sensitive
      }))
    }

    // 评估密码强度
    assessPasswordStrength()
  }
})

// 监听密码变化
watch(() => formData.value.password, assessPasswordStrength)
</script>

<style scoped>
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

input[type="checkbox"] {
  @apply rounded border-gray-300 text-primary focus:ring-primary;
}

select {
  @apply rounded border-gray-300 text-primary focus:ring-primary;
}
</style>
