<template>
  <div class="max-w-7xl mx-auto">
    <!-- 页面标题 -->
    <div class="mb-4 sm:mb-6 lg:mb-8">
      <h1 class="text-xl sm:text-2xl font-bold text-gray-900 dark:text-white">文件压缩</h1>
      <p class="text-gray-600 dark:text-gray-400 text-sm sm:text-base">选择文件并配置压缩选项</p>
    </div>

    <main class="grid grid-cols-1 lg:grid-cols-3 gap-3 xs:gap-4 sm:gap-6">
      <!-- 左侧：文件选择和配置 -->
      <div class="lg:col-span-2 space-y-4 xs:space-y-6">
        <!-- 文件选择组件 -->
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">选择要压缩的文件</h2>

          <EnhancedFileDropzone
            :multiple="true"
            :maxSize="1024 * 1024 * 1024 * 5"
            :maxFiles="100"
            :useTauriDialog="true"
            :showPreview="false"
            @files-selected="handleFilesSelected"
            @file-removed="handleFileRemoved"
            @error="handleFileError"
            class="mb-4"
          />

          <!-- 文件统计信息 -->
          <div v-if="selectedFiles.length > 0" class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
            <div class="flex items-center justify-between mb-4">
              <h3 class="font-medium text-gray-900 dark:text-white">已选择文件 ({{ selectedFiles.length }})</h3>
              <button
                @click="clearAllFiles"
                class="text-sm text-gray-500 hover:text-red-500 transition-colors focus:outline-none focus:ring-2 focus:ring-red-500 rounded p-1"
                aria-label="清空所有文件"
              >
                <i class="pi pi-trash mr-1"></i>
                清空所有
              </button>
            </div>

            <div class="grid grid-cols-1 xs:grid-cols-2 gap-2 xs:gap-3">
              <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                <div class="flex items-center">
                  <i class="pi pi-file text-gray-500 mr-3"></i>
                  <div class="flex-1">
                    <p class="font-medium text-gray-900 dark:text-white">文件总数</p>
                    <p class="text-2xl font-bold text-primary">{{ selectedFiles.length }}</p>
                  </div>
                </div>
              </div>

              <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                <div class="flex items-center">
                  <i class="pi pi-database text-gray-500 mr-3"></i>
                  <div class="flex-1">
                    <p class="font-medium text-gray-900 dark:text-white">总大小</p>
                    <p class="text-2xl font-bold text-primary">{{ formatTotalSize() }}</p>
                  </div>
                </div>
              </div>

              <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                <div class="flex items-center">
                  <i class="pi pi-compress text-gray-500 mr-3"></i>
                  <div class="flex-1">
                    <p class="font-medium text-gray-900 dark:text-white">预估压缩后</p>
                    <p class="text-2xl font-bold text-accent">{{ formatEstimatedSize() }}</p>
                  </div>
                </div>
              </div>

              <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                <div class="flex items-center">
                  <i class="pi pi-percentage text-gray-500 mr-3"></i>
                  <div class="flex-1">
                    <p class="font-medium text-gray-900 dark:text-white">压缩率</p>
                    <p class="text-2xl font-bold text-green-500">{{ estimatedCompressionRatio }}%</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 压缩配置 -->
        <CompressionSettingsPanel
          v-model="compressionOptions"
          v-model:outputPath="outputPath"
          @format-changed="handleFormatChanged"
          @options-changed="handleOptionsChanged"
        />
      </div>

      <!-- 右侧：操作面板 -->
      <div class="space-y-4 xs:space-y-6">
        <!-- 开始压缩按钮 -->
        <div class="glass-card">
          <button
            @click="startCompression"
            :disabled="!canStart || isProcessing"
            class="w-full glass-button-primary py-4 text-lg font-semibold flex items-center justify-center"
            :class="{ 'opacity-50 cursor-not-allowed': !canStart || isProcessing }"
          >
            <i v-if="isProcessing" class="pi pi-spin pi-spinner mr-3"></i>
            <i v-else class="pi pi-compress mr-3"></i>
            {{ isProcessing ? '压缩中...' : '开始压缩' }}
          </button>
          <p class="text-gray-500 dark:text-gray-400 text-sm mt-3 text-center">
            {{ canStart ? '点击开始压缩选中的文件' : '请先选择要压缩的文件' }}
          </p>

          <!-- 错误消息 -->
          <div v-if="errorMessage" class="mt-3 p-3 rounded-lg bg-red-500/10 border border-red-500/20">
            <div class="flex items-center">
              <i class="pi pi-exclamation-triangle text-red-500 mr-2"></i>
              <span class="text-red-500 text-sm">{{ errorMessage }}</span>
            </div>
          </div>

          <!-- 成功消息 -->
          <div v-if="successMessage" class="mt-3 p-3 rounded-lg bg-green-500/10 border border-green-500/20">
            <div class="flex items-center">
              <i class="pi pi-check-circle text-green-500 mr-2"></i>
              <span class="text-green-500 text-sm">{{ successMessage }}</span>
            </div>
          </div>
        </div>

        <!-- 进度显示 -->
        <div class="glass-card" v-if="isProcessing">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">压缩进度</h3>

          <div class="space-y-4">
            <div class="space-y-2">
              <div class="flex justify-between text-sm">
                <span class="text-gray-700 dark:text-gray-300">当前文件</span>
                <span class="font-medium">{{ currentProgress }}%</span>
              </div>
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                <div
                  class="bg-primary h-2 rounded-full progress-loading"
                  :style="{ width: currentProgress + '%' }"
                ></div>
              </div>
            </div>

            <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
              <div class="flex justify-between text-sm">
                <span class="text-gray-600 dark:text-gray-400">总进度</span>
                <span class="font-medium text-gray-900 dark:text-white">{{ totalProgress }}%</span>
              </div>
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3 mt-2">
                <div
                  class="bg-primary h-3 rounded-full progress-loading"
                  :style="{ width: totalProgress + '%' }"
                ></div>
              </div>
            </div>

            <!-- 压缩信息 -->
            <div class="pt-4 border-t border-gray-200 dark:border-gray-700 space-y-2">
              <div class="flex justify-between text-sm">
                <span class="text-gray-600 dark:text-gray-400">已处理文件</span>
                <span class="font-medium">{{ processedFiles }}/{{ selectedFiles.length }}</span>
              </div>
              <div class="flex justify-between text-sm">
                <span class="text-gray-600 dark:text-gray-400">压缩率</span>
                <span class="font-medium">{{ actualCompressionRatio }}%</span>
              </div>
              <div class="flex justify-between text-sm">
                <span class="text-gray-600 dark:text-gray-400">预计剩余时间</span>
                <span class="font-medium">{{ estimatedTimeRemaining }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- 压缩预设 -->
        <div class="glass-card">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">压缩预设</h3>
          <div class="space-y-3">
            <button
              v-for="preset in compressionPresets"
              :key="preset.id"
              @click="applyPreset(preset)"
              class="w-full glass-button text-left px-4 py-3"
            >
              <i :class="preset.icon" class="mr-3" :class="preset.color"></i>
              <div class="flex-1">
                <p class="font-medium">{{ preset.name }}</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">{{ preset.description }}</p>
              </div>
            </button>
          </div>
        </div>

        <!-- 格式特性 -->
        <div class="glass-card">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-3">格式特性</h3>
          <div class="space-y-3">
            <div
              v-for="feature in formatFeatures"
              :key="feature.name"
              class="flex items-center p-2 rounded-lg"
              :class="feature.active ? 'bg-primary/10' : 'bg-gray-50 dark:bg-gray-800'"
            >
              <i :class="feature.icon" class="mr-3" :class="feature.color"></i>
              <div class="flex-1">
                <p class="text-sm font-medium text-gray-900 dark:text-white">{{ feature.name }}</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">{{ feature.description }}</p>
              </div>
              <i
                v-if="feature.active"
                class="pi pi-check text-green-500"
              ></i>
            </div>
          </div>
        </div>

        <!-- 快捷操作 -->
        <div class="glass-card">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">快捷操作</h3>
          <div class="space-y-3">
            <button class="w-full glass-button text-left px-4 py-3">
              <i class="pi pi-history mr-3"></i>
              从历史记录选择
            </button>
            <button class="w-full glass-button text-left px-4 py-3">
              <i class="pi pi-star mr-3"></i>
              使用收藏的配置
            </button>
            <button class="w-full glass-button text-left px-4 py-3">
              <i class="pi pi-cog mr-3"></i>
              高级设置
            </button>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/tauri'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useCompressionStore } from '@/stores'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import CompressionSettingsPanel from '@/components/compression/CompressionSettingsPanel.vue'
import type { FileItem } from '@/components/ui/EnhancedFileDropzone.vue'
import type { CompressionOptions } from '@/stores'

const router = useRouter()
const tauriCommands = useTauriCommands()
const compressionStore = useCompressionStore()

// 状态
const selectedFiles = ref<FileItem[]>([])
const showPassword = ref(false)
const showConfirmPassword = ref(false)
const confirmPassword = ref('')

// 使用存储中的状态
const outputPath = computed({
  get: () => compressionStore.outputPath,
  set: (value) => compressionStore.setOutputPath(value)
})

const compressionOptions = computed({
  get: () => compressionStore.compressionOptions,
  set: (value) => compressionStore.updateCompressionOptions(value)
})

const isProcessing = computed(() => compressionStore.isProcessing)
const currentProgress = computed(() => compressionStore.currentProgress)
const totalProgress = computed(() => compressionStore.totalProgress)
const processedFiles = computed(() => compressionStore.processedFiles)
const actualCompressionRatio = computed(() => compressionStore.estimatedCompressionRatio)
const errorMessage = computed(() => compressionStore.errorMessage)
const successMessage = computed(() => compressionStore.successMessage)


// 压缩预设
const compressionPresets = [
  { id: 1, name: '快速分享', icon: 'pi pi-share-alt', color: 'text-primary', format: 'zip', level: 3, description: '快速压缩，适合分享' },
  { id: 2, name: '长期存储', icon: 'pi pi-database', color: 'text-green-500', format: '7z', level: 9, description: '最高压缩率，节省空间' },
  { id: 3, name: '备份归档', icon: 'pi pi-save', color: 'text-purple-500', format: 'tar', level: 5, description: '保持目录结构，适合备份' },
  { id: 4, name: '网页优化', icon: 'pi pi-globe', color: 'text-blue-500', format: 'gz', level: 6, description: '适合网页文件压缩' }
]

// 计算属性
const canStart = computed(() => {
  return selectedFiles.value.length > 0 &&
         outputPath.value.length > 0
})

const estimatedCompressionRatio = computed(() => compressionStore.estimatedCompressionRatio)

const formatFeatures = computed(() => {
  const features = [
    { name: '密码保护', icon: 'pi pi-lock', color: 'text-warning', description: '支持AES-256加密', active: ['zip', '7z'].includes(compressionOptions.value.format) },
    { name: '分卷压缩', icon: 'pi pi-copy', color: 'text-primary', description: '支持分割大文件', active: ['zip', '7z'].includes(compressionOptions.value.format) },
    { name: '固实压缩', icon: 'pi pi-box', color: 'text-green-500', description: '提高压缩率', active: ['7z'].includes(compressionOptions.value.format) },
    { name: '恢复记录', icon: 'pi pi-shield', color: 'text-red-500', description: '数据损坏恢复', active: ['zip', '7z'].includes(compressionOptions.value.format) },
    { name: '多线程', icon: 'pi pi-bolt', color: 'text-yellow-500', description: '并行压缩加速', active: ['7z', 'gz', 'bz2'].includes(compressionOptions.value.format) }
  ]

  return features
})

const estimatedTimeRemaining = computed(() => {
  if (totalProgress.value === 0) return '计算中...'
  if (totalProgress.value >= 100) return '已完成'

  const elapsedTime = 0 // 实际应用中需要记录开始时间
  const estimatedTotalTime = elapsedTime / (totalProgress.value / 100)
  const remainingSeconds = Math.max(0, estimatedTotalTime - elapsedTime)

  if (remainingSeconds < 60) return `${Math.round(remainingSeconds)}秒`
  if (remainingSeconds < 3600) return `${Math.round(remainingSeconds / 60)}分钟`
  return `${Math.round(remainingSeconds / 3600)}小时`
})

// 方法
const handleFilesSelected = (files: FileItem[]) => {
  console.log('文件选择:', files)
  selectedFiles.value = [...selectedFiles.value, ...files]
}

const handleFileRemoved = (fileId: string) => {
  console.log('文件移除:', fileId)
  selectedFiles.value = selectedFiles.value.filter(file => file.id !== fileId)
}

const handleFileError = (error: string) => {
  console.error('文件选择错误:', error)
  alert(`文件选择错误: ${error}`)
}

const clearAllFiles = () => {
  selectedFiles.value = []
}

const formatTotalSize = (): string => {
  const totalBytes = selectedFiles.value.reduce((sum, file) => sum + file.size, 0)
  return compressionStore.formatFileSize(totalBytes)
}

const formatEstimatedSize = (): string => {
  const totalBytes = selectedFiles.value.reduce((sum, file) => sum + file.size, 0)
  const estimatedBytes = totalBytes * (estimatedCompressionRatio.value / 100)
  return compressionStore.formatFileSize(estimatedBytes)
}

const handleFormatChanged = (format: string) => {
  console.log('格式已更新', format)
}

const handleOptionsChanged = (options: CompressionOptions) => {
  console.log('选项已更新', options)
}

const applyPreset = (preset: any) => {
  compressionStore.updateCompressionOptions({
    format: preset.format,
    level: preset.level
  })
}

const startCompression = async () => {
  if (!canStart.value) return

  // 密码验证
  if (compressionOptions.value.password && compressionOptions.value.password !== confirmPassword.value) {
    compressionStore.errorMessage = '两次输入的密码不一致，请检查'
    setTimeout(() => { compressionStore.errorMessage = '' }, 5000)
    return
  }

  try {
    // 添加文件到存储
    compressionStore.addFiles(selectedFiles.value)

    // 使用存储的压缩方法
    await compressionStore.startCompression()

    // 重置本地状态
    selectedFiles.value = []
    confirmPassword.value = ''

  } catch (error) {
    console.error('压缩失败:', error)
    // 错误消息已经在存储中设置
  }
}

</script>

<style scoped>
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
