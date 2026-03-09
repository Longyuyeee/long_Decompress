<template>
  <div class="compression-settings-panel">
    <!-- 压缩配置 -->
    <div class="glass-card">
      <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">压缩配置</h2>

      <div class="space-y-6">
        <!-- 压缩格式选择 -->
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            压缩格式
          </label>
          <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
            <button
              v-for="format in compressionFormats"
              :key="format.value"
              @click="selectFormat(format.value)"
              :class="[
                'p-3 rounded-lg border-2 text-center transition-all',
                compressionOptions.format === format.value
                  ? 'border-primary bg-primary/10'
                  : 'border-gray-200 dark:border-gray-700 hover:border-primary'
              ]"
              :aria-label="`选择${format.name}格式`"
            >
              <i :class="[format.icon, format.color]" class="text-lg block mb-2"></i>
              <span class="font-medium">{{ format.name }}</span>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">{{ format.description }}</p>
            </button>
          </div>
        </div>

        <!-- 压缩级别 -->
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            压缩级别
            <span class="text-gray-500 dark:text-gray-400 ml-2">
              {{ compressionLevelLabels[compressionOptions.level] }}
            </span>
          </label>
          <div class="flex items-center space-x-4">
            <input
              type="range"
              v-model="compressionOptions.level"
              min="1"
              max="9"
              step="1"
              class="flex-1 h-2 bg-gray-200 dark:bg-gray-700 rounded-lg appearance-none cursor-pointer"
              aria-label="压缩级别滑块"
            />
            <span class="text-sm font-medium text-gray-700 dark:text-gray-300 w-8 text-center">
              {{ compressionOptions.level }}
            </span>
          </div>
          <div class="flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-2">
            <span>最快</span>
            <span>平衡</span>
            <span>最优</span>
          </div>
        </div>

        <!-- 输出设置 -->
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            输出设置
          </label>
          <div class="space-y-3">
            <div class="flex space-x-2">
              <input
                type="text"
                v-model="outputPath"
                class="flex-1 glass-input"
                placeholder="选择压缩文件保存路径"
                aria-label="输出路径"
              />
              <button
                @click="selectOutputPath"
                class="glass-button px-4"
                :aria-label="`选择输出路径`"
              >
                <i class="pi pi-folder-open"></i>
              </button>
            </div>
            <div class="flex space-x-2">
              <input
                type="text"
                v-model="compressionOptions.filename"
                class="flex-1 glass-input"
                placeholder="压缩文件名（可选）"
                aria-label="压缩文件名"
              />
              <span class="flex items-center px-3 bg-gray-100 dark:bg-gray-800 rounded-lg border border-gray-300 dark:border-gray-600">
                .{{ getCurrentFormatExtension() }}
              </span>
            </div>
          </div>
        </div>

        <!-- 密码保护 -->
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            密码保护（可选）
          </label>
          <div class="space-y-3">
            <div class="relative">
              <input
                :type="showPassword ? 'text' : 'password'"
                v-model="compressionOptions.password"
                class="w-full glass-input pr-10"
                placeholder="设置压缩密码"
                aria-label="压缩密码"
              />
              <button
                @click="showPassword = !showPassword"
                class="absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-500 focus:outline-none focus:ring-2 focus:ring-primary rounded p-1"
                :aria-label="showPassword ? '隐藏密码' : '显示密码'"
              >
                <i :class="showPassword ? 'pi pi-eye-slash' : 'pi pi-eye'" aria-hidden="true"></i>
              </button>
            </div>
            <div class="relative" v-if="compressionOptions.password">
              <input
                :type="showConfirmPassword ? 'text' : 'password'"
                v-model="confirmPassword"
                class="w-full glass-input pr-10"
                placeholder="确认密码"
                aria-label="确认密码"
              />
              <button
                @click="showConfirmPassword = !showConfirmPassword"
                class="absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-500 focus:outline-none focus:ring-2 focus:ring-primary rounded p-1"
                :aria-label="showConfirmPassword ? '隐藏确认密码' : '显示确认密码'"
              >
                <i :class="showConfirmPassword ? 'pi pi-eye-slash' : 'pi pi-eye'" aria-hidden="true"></i>
              </button>
            </div>
            <div v-if="compressionOptions.password && confirmPassword && compressionOptions.password !== confirmPassword" class="text-sm text-red-500">
              <i class="pi pi-exclamation-triangle mr-1"></i>
              两次输入的密码不一致
            </div>
          </div>
        </div>

        <!-- 高级选项 -->
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            高级选项
          </label>
          <div class="space-y-3">
            <label class="flex items-center">
              <input
                type="checkbox"
                v-model="compressionOptions.splitArchive"
                class="mr-3"
                :disabled="!supportsSplitArchive"
              />
              <span class="text-gray-700 dark:text-gray-300" :class="{ 'opacity-50': !supportsSplitArchive }">
                分卷压缩
              </span>
              <div class="ml-2 flex-1 max-w-xs" v-if="compressionOptions.splitArchive && supportsSplitArchive">
                <select
                  v-model="compressionOptions.splitSize"
                  class="glass-input text-sm w-full"
                >
                  <option value="100">100 MB</option>
                  <option value="500">500 MB</option>
                  <option value="1024">1 GB</option>
                  <option value="2048">2 GB</option>
                  <option value="4096">4 GB</option>
                </select>
              </div>
            </label>
            <label class="flex items-center">
              <input
                type="checkbox"
                v-model="compressionOptions.keepStructure"
                class="mr-3"
              />
              <span class="text-gray-700 dark:text-gray-300">保持目录结构</span>
            </label>
            <label class="flex items-center">
              <input
                type="checkbox"
                v-model="compressionOptions.deleteAfter"
                class="mr-3"
              />
              <span class="text-gray-700 dark:text-gray-300">压缩后删除原文件</span>
            </label>
            <label class="flex items-center">
              <input
                type="checkbox"
                v-model="compressionOptions.createSolidArchive"
                class="mr-3"
                :disabled="!supportsSolidArchive"
              />
              <span class="text-gray-700 dark:text-gray-300" :class="{ 'opacity-50': !supportsSolidArchive }">
                创建固实压缩包（提高压缩率）
              </span>
            </label>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'

// Props
interface Props {
  modelValue?: CompressionOptions
  outputPath?: string
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: undefined,
  outputPath: ''
})

// Emits
interface Emits {
  (e: 'update:modelValue', value: CompressionOptions): void
  (e: 'update:outputPath', value: string): void
  (e: 'format-changed', format: string): void
  (e: 'options-changed', options: CompressionOptions): void
}

const emit = defineEmits<Emits>()

// 类型定义
export interface CompressionOptions {
  format: 'zip' | '7z' | 'tar' | 'gz' | 'bz2' | 'tar.gz' | 'tar.bz2' | 'xz' | 'tar.xz' | 'rar'
  level: number
  password: string
  filename: string
  splitArchive: boolean
  splitSize: string
  keepStructure: boolean
  deleteAfter: boolean
  createSolidArchive: boolean
}

// 状态
const tauriCommands = useTauriCommands()
const showPassword = ref(false)
const showConfirmPassword = ref(false)
const confirmPassword = ref('')

// 压缩选项 - 使用props或默认值
const compressionOptions = ref<CompressionOptions>(props.modelValue || {
  format: 'zip',
  level: 6,
  password: '',
  filename: '',
  splitArchive: false,
  splitSize: '1024',
  keepStructure: true,
  deleteAfter: false,
  createSolidArchive: false
})

const outputPath = ref(props.outputPath)

// 压缩格式选项
const compressionFormats = [
  { value: 'zip', name: 'ZIP', icon: 'pi pi-file', color: 'text-primary', description: '通用压缩格式', extension: 'zip' },
  { value: 'tar', name: 'TAR', icon: 'pi pi-file', color: 'text-purple-500', description: '归档格式', extension: 'tar' },
  { value: 'gz', name: 'GZIP', icon: 'pi pi-file', color: 'text-yellow-500', description: 'Gzip压缩', extension: 'gz' },
  { value: 'tar.gz', name: 'TAR.GZ', icon: 'pi pi-file', color: 'text-blue-500', description: 'Tar+Gzip', extension: 'tar.gz' },
  { value: 'bz2', name: 'BZIP2', icon: 'pi pi-file', color: 'text-red-500', description: 'Bzip2压缩', extension: 'bz2' },
  { value: 'tar.bz2', name: 'TAR.BZ2', icon: 'pi pi-file', color: 'text-orange-500', description: 'Tar+Bzip2', extension: 'tar.bz2' },
  { value: 'xz', name: 'XZ', icon: 'pi pi-file', color: 'text-indigo-500', description: 'XZ压缩', extension: 'xz' },
  { value: 'tar.xz', name: 'TAR.XZ', icon: 'pi pi-file', color: 'text-pink-500', description: 'Tar+XZ', extension: 'tar.xz' },
  { value: '7z', name: '7-Zip', icon: 'pi pi-file', color: 'text-green-500', description: '高压缩率', extension: '7z' },
  { value: 'rar', name: 'RAR', icon: 'pi pi-file', color: 'text-teal-500', description: 'RAR格式', extension: 'rar' }
]

// 压缩级别标签
const compressionLevelLabels: Record<number, string> = {
  1: '存储（最快）',
  2: '最高',
  3: '快速',
  4: '较快',
  5: '标准',
  6: '较好',
  7: '最佳',
  8: '超强',
  9: '极限'
}

// 计算属性
const supportsSplitArchive = computed(() => {
  return ['zip', '7z', 'rar'].includes(compressionOptions.value.format)
})

const supportsSolidArchive = computed(() => {
  return compressionOptions.value.format === '7z'
})

// 方法
const selectFormat = (format: string) => {
  compressionOptions.value.format = format as any
  emit('format-changed', format)
}

const selectOutputPath = async () => {
  try {
    const result = await tauriCommands.selectDirectory()
    if (result && typeof result === 'string') {
      outputPath.value = result
      emit('update:outputPath', result)
    }
  } catch (error) {
    console.error('选择输出路径失败:', error)
    alert('选择输出路径失败，请重试')
  }
}

const getCurrentFormatExtension = (): string => {
  const selectedFormat = compressionFormats.find(f => f.value === compressionOptions.value.format)
  return selectedFormat?.extension || compressionOptions.value.format
}

// 监听选项变化并发出事件
watch(compressionOptions, (newOptions) => {
  emit('update:modelValue', newOptions)
  emit('options-changed', newOptions)
}, { deep: true })

watch(outputPath, (newPath) => {
  emit('update:outputPath', newPath)
})

// 监听压缩格式变化，禁用不支持的选项
watch(() => compressionOptions.value.format, (newFormat) => {
  // 某些格式不支持某些选项
  if (newFormat === 'tar') {
    compressionOptions.value.password = ''
    compressionOptions.value.splitArchive = false
    compressionOptions.value.createSolidArchive = false
  }

  if (newFormat === 'gz' || newFormat === 'bz2') {
    compressionOptions.value.splitArchive = false
  }
})

// 暴露方法给父组件
defineExpose({
  getOptions: () => compressionOptions.value,
  getOutputPath: () => outputPath.value,
  validate: () => {
    if (compressionOptions.value.password && compressionOptions.value.password !== confirmPassword.value) {
      return { valid: false, error: '两次输入的密码不一致' }
    }
    return { valid: true }
  }
})
</script>

<style scoped>
.compression-settings-panel {
  @apply space-y-6;
}

input[type="checkbox"] {
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
</style>
