<template>
  <div class="decompress-settings-panel">
    <!-- 输出目录设置 -->
    <div class="mb-6">
      <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        输出目录
      </label>
      <div class="flex space-x-2">
        <input
          type="text"
          v-model="settings.outputPath"
          class="flex-1 glass-input"
          placeholder="选择解压输出目录"
          @click="selectOutputPath"
          :disabled="isProcessing"
        />
        <button
          @click="selectOutputPath"
          class="glass-button px-4"
          :disabled="isProcessing"
          :aria-label="`选择输出目录`"
        >
          <i class="pi pi-folder-open"></i>
        </button>
      </div>
      <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
        留空则解压到原文件所在目录
      </p>
    </div>

    <!-- 密码设置 -->
    <div class="mb-6">
      <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        解压密码设置
      </label>
      <div class="space-y-3">
        <div class="relative">
          <input
            :type="showPassword ? 'text' : 'password'"
            v-model="settings.password"
            class="w-full glass-input pr-10"
            placeholder="输入解压密码"
            :disabled="isProcessing"
            aria-label="解压密码"
          />
          <button
            @click="showPassword = !showPassword"
            class="absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-500 focus:outline-none focus:ring-2 focus:ring-primary rounded p-1"
            :disabled="isProcessing"
            :aria-label="showPassword ? '隐藏密码' : '显示密码'"
          >
            <i :class="showPassword ? 'pi pi-eye-slash' : 'pi pi-eye'" aria-hidden="true"></i>
          </button>
        </div>

        <!-- 密码尝试设置 -->
        <div v-if="settings.password" class="pl-4 border-l-2 border-primary/30">
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            密码尝试设置
          </label>
          <div class="space-y-2">
            <label class="flex items-center">
              <input
                type="checkbox"
                v-model="settings.passwordOptions.rememberForSession"
                class="mr-3"
                :disabled="isProcessing"
              />
              <span class="text-gray-700 dark:text-gray-300 text-sm">本次会话记住密码</span>
            </label>
            <label class="flex items-center">
              <input
                type="checkbox"
                v-model="settings.passwordOptions.autoTryCommon"
                class="mr-3"
                :disabled="isProcessing"
              />
              <span class="text-gray-700 dark:text-gray-300 text-sm">自动尝试常见密码</span>
            </label>
            <div v-if="settings.passwordOptions.autoTryCommon" class="ml-6">
              <label class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
                最大尝试次数
              </label>
              <input
                type="range"
                v-model="settings.passwordOptions.maxAttempts"
                min="1"
                max="10"
                step="1"
                class="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-lg"
                :disabled="isProcessing"
              />
              <div class="flex justify-between text-xs text-gray-500 dark:text-gray-400">
                <span>1次</span>
                <span>{{ settings.passwordOptions.maxAttempts }}次</span>
                <span>10次</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 解压选项 -->
    <div class="mb-6">
      <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        解压选项
      </label>
      <div class="space-y-3">
        <label class="flex items-center">
          <input
            type="checkbox"
            v-model="settings.options.keepStructure"
            class="mr-3"
            :disabled="isProcessing"
          />
          <span class="text-gray-700 dark:text-gray-300">保持目录结构</span>
        </label>

        <!-- 文件覆盖策略 -->
        <div class="pl-4 border-l-2 border-gray-200 dark:border-gray-700">
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            文件覆盖策略
          </label>
          <div class="space-y-2">
            <label class="flex items-center">
              <input
                type="radio"
                v-model="settings.options.overwriteStrategy"
                value="ask"
                class="mr-3"
                :disabled="isProcessing"
              />
              <span class="text-gray-700 dark:text-gray-300 text-sm">每次询问</span>
            </label>
            <label class="flex items-center">
              <input
                type="radio"
                v-model="settings.options.overwriteStrategy"
                value="overwrite"
                class="mr-3"
                :disabled="isProcessing"
              />
              <span class="text-gray-700 dark:text-gray-300 text-sm">总是覆盖</span>
            </label>
            <label class="flex items-center">
              <input
                type="radio"
                v-model="settings.options.overwriteStrategy"
                value="skip"
                class="mr-3"
                :disabled="isProcessing"
              />
              <span class="text-gray-700 dark:text-gray-300 text-sm">跳过已存在文件</span>
            </label>
            <label class="flex items-center">
              <input
                type="radio"
                v-model="settings.options.overwriteStrategy"
                value="rename"
                class="mr-3"
                :disabled="isProcessing"
              />
              <span class="text-gray-700 dark:text-gray-300 text-sm">自动重命名</span>
            </label>
          </div>
        </div>

        <!-- 高级选项 -->
        <div class="pl-4 border-l-2 border-gray-200 dark:border-gray-700">
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            高级选项
          </label>
          <div class="space-y-2">
            <label class="flex items-center">
              <input
                type="checkbox"
                v-model="settings.options.deleteAfter"
                class="mr-3"
                :disabled="isProcessing"
              />
              <span class="text-gray-700 dark:text-gray-300 text-sm">解压后删除原文件</span>
            </label>
            <label class="flex items-center">
              <input
                type="checkbox"
                v-model="settings.options.preserveTimestamps"
                class="mr-3"
                :disabled="isProcessing"
              />
              <span class="text-gray-700 dark:text-gray-300 text-sm">保留文件时间戳</span>
            </label>
            <label class="flex items-center">
              <input
                type="checkbox"
                v-model="settings.options.skipCorrupted"
                class="mr-3"
                :disabled="isProcessing"
              />
              <span class="text-gray-700 dark:text-gray-300 text-sm">跳过损坏的文件</span>
            </label>
            <label class="flex items-center">
              <input
                type="checkbox"
                v-model="settings.options.extractOnlyNewer"
                class="mr-3"
                :disabled="isProcessing"
              />
              <span class="text-gray-700 dark:text-gray-300 text-sm">仅解压较新的文件</span>
            </label>
            <label class="flex items-center">
              <input
                type="checkbox"
                v-model="settings.options.createSubdirectory"
                class="mr-3"
                :disabled="isProcessing"
              />
              <span class="text-gray-700 dark:text-gray-300 text-sm">为每个压缩包创建子目录</span>
            </label>
          </div>
        </div>

        <!-- 解压过滤器 -->
        <div class="pl-4 border-l-2 border-gray-200 dark:border-gray-700">
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            文件过滤器
          </label>
          <div class="space-y-2">
            <input
              type="text"
              v-model="settings.options.fileFilter"
              class="w-full glass-input text-sm"
              placeholder="例如: *.txt, *.jpg, document*"
              :disabled="isProcessing"
            />
            <p class="text-xs text-gray-500 dark:text-gray-400">
              使用通配符过滤要解压的文件，多个模式用逗号分隔
            </p>
          </div>
        </div>
      </div>
    </div>

    <!-- 预设按钮 -->
    <div class="mb-6">
      <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        快速预设
      </label>
      <div class="grid grid-cols-1 xs:grid-cols-2 gap-1 xs:gap-2">
        <button
          @click="applyPreset('quick')"
          class="glass-button text-left px-3 py-2"
          :disabled="isProcessing"
        >
          <i class="pi pi-bolt text-primary mr-2"></i>
          <div>
            <p class="font-medium text-sm">快速解压</p>
            <p class="text-xs text-gray-500">最快速度</p>
          </div>
        </button>
        <button
          @click="applyPreset('safe')"
          class="glass-button text-left px-3 py-2"
          :disabled="isProcessing"
        >
          <i class="pi pi-shield text-green-500 mr-2"></i>
          <div>
            <p class="font-medium text-sm">安全解压</p>
            <p class="text-xs text-gray-500">验证完整性</p>
          </div>
        </button>
        <button
          @click="applyPreset('batch')"
          class="glass-button text-left px-3 py-2"
          :disabled="isProcessing"
        >
          <i class="pi pi-copy text-purple-500 mr-2"></i>
          <div>
            <p class="font-medium text-sm">批量解压</p>
            <p class="text-xs text-gray-500">多个文件</p>
          </div>
        </button>
        <button
          @click="applyPreset('deep')"
          class="glass-button text-left px-3 py-2"
          :disabled="isProcessing"
        >
          <i class="pi pi-search text-blue-500 mr-2"></i>
          <div>
            <p class="font-medium text-sm">深度扫描</p>
            <p class="text-xs text-gray-500">修复损坏</p>
          </div>
        </button>
      </div>
    </div>

    <!-- 重置按钮 -->
    <div class="flex justify-end">
      <button
        @click="resetSettings"
        class="glass-button px-4 py-2 text-sm text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200"
        :disabled="isProcessing"
      >
        <i class="pi pi-refresh mr-2"></i>
        重置为默认
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'

// 定义组件属性
interface Props {
  modelValue?: DecompressSettings
  isProcessing?: boolean
}

// 定义解压设置类型
export interface DecompressSettings {
  outputPath: string
  password: string
  options: {
    keepStructure: boolean
    overwriteStrategy: 'ask' | 'overwrite' | 'skip' | 'rename'
    deleteAfter: boolean
    preserveTimestamps: boolean
    skipCorrupted: boolean
    extractOnlyNewer: boolean
    createSubdirectory: boolean
    fileFilter: string
  }
  passwordOptions: {
    rememberForSession: boolean
    autoTryCommon: boolean
    maxAttempts: number
  }
}

// 定义组件事件
interface Emits {
  (e: 'update:modelValue', value: DecompressSettings): void
  (e: 'settings-change', value: DecompressSettings): void
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: undefined,
  isProcessing: false
})

const emit = defineEmits<Emits>()

const tauriCommands = useTauriCommands()
const showPassword = ref(false)

// 默认设置
const defaultSettings: DecompressSettings = {
  outputPath: '',
  password: '',
  options: {
    keepStructure: true,
    overwriteStrategy: 'ask',
    deleteAfter: false,
    preserveTimestamps: true,
    skipCorrupted: false,
    extractOnlyNewer: false,
    createSubdirectory: false,
    fileFilter: ''
  },
  passwordOptions: {
    rememberForSession: false,
    autoTryCommon: false,
    maxAttempts: 3
  }
}

// 本地设置状态
const settings = ref<DecompressSettings>({ ...defaultSettings })

// 监听外部传入的modelValue
watch(
  () => props.modelValue,
  (newValue) => {
    if (newValue) {
      settings.value = { ...newValue }
    }
  },
  { immediate: true }
)

// 监听设置变化，触发事件
watch(
  settings,
  (newSettings) => {
    emit('update:modelValue', newSettings)
    emit('settings-change', newSettings)
  },
  { deep: true }
)

// 计算属性：是否有加密文件需要密码
const hasEncryptedFiles = computed(() => {
  // 这里需要从父组件传入或通过其他方式获取
  return false
})

// 计算属性：是否需要密码
const requiresPassword = computed(() => {
  return hasEncryptedFiles.value && !settings.value.password
})

// 方法：选择输出路径
const selectOutputPath = async () => {
  if (props.isProcessing) return

  try {
    const result = await tauriCommands.selectDirectory()
    if (result && typeof result === 'string') {
      settings.value.outputPath = result
    }
  } catch (error) {
    console.error('选择输出路径失败:', error)
    alert('选择输出路径失败，请重试')
  }
}

// 方法：应用预设
const applyPreset = (presetType: 'quick' | 'safe' | 'batch' | 'deep') => {
  if (props.isProcessing) return

  switch (presetType) {
    case 'quick': // 快速解压
      settings.value.options.overwriteStrategy = 'overwrite'
      settings.value.options.skipCorrupted = true
      settings.value.options.extractOnlyNewer = false
      break
    case 'safe': // 安全解压
      settings.value.options.overwriteStrategy = 'ask'
      settings.value.options.skipCorrupted = false
      settings.value.options.preserveTimestamps = true
      break
    case 'batch': // 批量解压
      settings.value.options.createSubdirectory = true
      settings.value.options.keepStructure = true
      break
    case 'deep': // 深度扫描
      settings.value.options.skipCorrupted = false
      break
  }
}

// 方法：重置设置
const resetSettings = () => {
  if (props.isProcessing) return

  if (confirm('确定要重置所有解压设置为默认值吗？')) {
    settings.value = { ...defaultSettings }
  }
}

// 暴露方法给父组件
defineExpose({
  getSettings: () => settings.value,
  resetSettings,
  validateSettings: () => {
    if (requiresPassword.value) {
      return {
        valid: false,
        message: '检测到加密文件，请输入解压密码'
      }
    }
    return { valid: true }
  }
})
</script>

<style scoped>
.decompress-settings-panel {
  @apply space-y-4;
}

input[type="checkbox"],
input[type="radio"] {
  @apply rounded border-gray-300 text-primary focus:ring-primary;
}

input[type="range"] {
  @apply appearance-none;
}

input[type="range"]::-webkit-slider-thumb {
  @apply appearance-none w-4 h-4 rounded-full bg-primary cursor-pointer;
}

input[type="range"]::-moz-range-thumb {
  @apply w-4 h-4 rounded-full bg-primary cursor-pointer border-0;
}

.glass-input {
  @apply px-3 py-2 bg-white/10 dark:bg-black/10 border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all;
}

.glass-button {
  @apply px-4 py-2 bg-white/10 dark:bg-black/10 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-white/20 dark:hover:bg-black/20 transition-all focus:outline-none focus:ring-2 focus:ring-primary;
}

.glass-button:disabled {
  @apply opacity-50 cursor-not-allowed;
}
</style>
