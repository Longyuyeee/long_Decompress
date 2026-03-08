<template>
  <div class="max-w-7xl mx-auto">
    <!-- 页面标题 -->
    <div class="mb-4 sm:mb-6 lg:mb-8">
      <h1 class="text-xl sm:text-2xl font-bold text-gray-900 dark:text-white">文件压缩</h1>
      <p class="text-gray-600 dark:text-gray-400 text-sm sm:text-base">选择文件并配置压缩选项</p>
    </div>

    <main class="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
      <!-- 左侧：文件选择和配置 -->
      <div class="lg:col-span-2 space-y-6">
        <!-- 文件选择组件 -->
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">选择要压缩的文件</h2>

          <EnhancedFileDropzone
            :multiple="true"
            :maxSize="1024 * 1024 * 1024 * 5" <!-- 5GB -->
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
                :aria-label="`清空所有文件`"
              >
                <i class="pi pi-trash mr-1"></i>
                清空所有
              </button>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
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
                >
                  <i :class="format.icon" class="text-lg block mb-2" :class="format.color"></i>
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
                />
                <span class="text-sm font-medium text-gray-700 dark:text-gray-300 w-8 text-center">
                  {{ compressionOptions.level }}
                </span>
              </div>
              <div class="flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-2">
                <span>最快</span>
                <span>平衡</span>
                <span>最小</span>
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
                  />
                  <span class="text-gray-700 dark:text-gray-300">分卷压缩</span>
                  <div class="ml-2 flex-1 max-w-xs" v-if="compressionOptions.splitArchive">
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
                  />
                  <span class="text-gray-700 dark:text-gray-300">创建固实压缩包（提高压缩率）</span>
                </label>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 右侧：操作面板 -->
      <div class="space-y-6">
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
                  class="bg-primary h-2 rounded-full transition-all duration-300"
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
                  class="bg-primary h-3 rounded-full transition-all duration-300"
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
import { invoke } from '@tauri-apps/api/core'
import { useTauriCommands } from '@/composables/useTauriCommands'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import type { FileItem } from '@/components/ui/EnhancedFileDropzone.vue'

const router = useRouter()
const tauriCommands = useTauriCommands()

// 状态
const selectedFiles = ref<FileItem[]>([])
const outputPath = ref('')
const showPassword = ref(false)
const showConfirmPassword = ref(false)
const confirmPassword = ref('')
const isProcessing = ref(false)
const currentProgress = ref(0)
const totalProgress = ref(0)
const processedFiles = ref(0)
const actualCompressionRatio = ref(0)
const errorMessage = ref('')
const successMessage = ref('')

// 压缩选项
const compressionOptions = ref({
  format: 'zip' as 'zip' | '7z' | 'tar' | 'gz' | 'bz2',
  level: 6,
  password: '',
  filename: '',
  splitArchive: false,
  splitSize: '1024',
  keepStructure: true,
  deleteAfter: false,
  createSolidArchive: false
})

// 压缩格式选项 - 匹配后端CompressionFormat枚举
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
  2: '最快',
  3: '快速',
  4: '较快',
  5: '标准',
  6: '较好',
  7: '最大',
  8: '超强',
  9: '极限'
}

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
         outputPath.value.length > 0 &&
         (!compressionOptions.value.password || compressionOptions.value.password === confirmPassword.value)
})

const estimatedCompressionRatio = computed(() => {
  // 根据压缩级别和格式估算压缩率
  const baseRatios: Record<string, number> = {
    zip: 0.7,
    '7z': 0.5,
    tar: 1.0,
    gz: 0.6,
    bz2: 0.55
  }

  const levelFactor = 1 - (compressionOptions.value.level / 10) * 0.3
  const baseRatio = baseRatios[compressionOptions.value.format] || 0.7

  return Math.round(baseRatio * levelFactor * 100)
})

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

  if (totalBytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(totalBytes) / Math.log(k))
  return parseFloat((totalBytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const formatEstimatedSize = (): string => {
  const totalBytes = selectedFiles.value.reduce((sum, file) => sum + file.size, 0)
  const estimatedBytes = totalBytes * (estimatedCompressionRatio.value / 100)

  if (estimatedBytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(estimatedBytes) / Math.log(k))
  return parseFloat((estimatedBytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const selectFormat = (format: string) => {
  compressionOptions.value.format = format as any
}

const selectOutputPath = async () => {
  try {
    // 使用Tauri对话框选择输出路径
    const result = await tauriCommands.selectDirectory()
    if (result && typeof result === 'string') {
      outputPath.value = result
    }
  } catch (error) {
    console.error('选择输出路径失败:', error)
    alert('选择输出路径失败，请重试')
  }
}

const applyPreset = (preset: any) => {
  compressionOptions.value.format = preset.format
  compressionOptions.value.level = preset.level
}

const startCompression = async () => {
  if (!canStart.value) return

  // 密码验证
  if (compressionOptions.value.password && compressionOptions.value.password !== confirmPassword.value) {
    errorMessage.value = '两次输入的密码不一致，请检查'
    setTimeout(() => { errorMessage.value = '' }, 5000)
    return
  }

  // 清除旧消息
  errorMessage.value = ''
  successMessage.value = ''

  isProcessing.value = true
  currentProgress.value = 0
  totalProgress.value = 0
  processedFiles.value = 0

  try {
    // 准备文件路径
    const filePaths = selectedFiles.value.map(file => file.path)

    // 生成输出文件名
    let outputFileName = compressionOptions.value.filename
    if (!outputFileName) {
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
      outputFileName = `压缩文件_${timestamp}`
    }

    // 获取当前选中格式的扩展名
    const selectedFormat = compressionFormats.find(f => f.value === compressionOptions.value.format)
    const extension = selectedFormat?.extension || compressionOptions.value.format
    const fullOutputPath = `${outputPath.value}/${outputFileName}.${extension}`

    // 调用Tauri压缩API
    // 注意：需要将前端选项映射到后端CompressionOptions结构
    const result = await invoke('compress_file', {
      files: filePaths,
      outputPath: fullOutputPath,
      options: {
        password: compressionOptions.value.password || null,
        compression_level: compressionOptions.value.level, // 后端字段名是compression_level
        split_size: compressionOptions.value.splitArchive && compressionOptions.value.splitSize
          ? parseInt(compressionOptions.value.splitSize) * 1024 * 1024 // 转换为字节
          : null,
        preserve_paths: compressionOptions.value.keepStructure,
        exclude_patterns: [],
        include_patterns: [],
        create_subdirectories: true,
        overwrite_existing: true
      }
    })

    console.log('压缩结果:', result)

    // 模拟进度更新
    const interval = setInterval(() => {
      currentProgress.value = Math.min(100, currentProgress.value + Math.random() * 10)
      totalProgress.value = Math.min(100, totalProgress.value + Math.random() * 5)

      if (currentProgress.value >= 100) {
        processedFiles.value++
        currentProgress.value = 0
      }

      if (totalProgress.value >= 100) {
        clearInterval(interval)
        setTimeout(() => {
          isProcessing.value = false
          successMessage.value = '压缩完成！'
          // 5秒后清除成功消息
          setTimeout(() => { successMessage.value = '' }, 5000)
          // 重置状态
          selectedFiles.value = []
          compressionOptions.value.password = ''
          confirmPassword.value = ''
        }, 500)
      }
    }, 300)

  } catch (error) {
    console.error('压缩失败:', error)
    errorMessage.value = `压缩失败: ${error}`
    isProcessing.value = false
    // 5秒后清除错误消息
    setTimeout(() => { errorMessage.value = '' }, 5000)
  }

  // 获取当前选中格式的扩展名
  const getCurrentFormatExtension = (): string => {
    const selectedFormat = compressionFormats.find(f => f.value === compressionOptions.value.format)
    return selectedFormat?.extension || compressionOptions.value.format
  }
}

// 监听压缩选项变化
watch(() => compressionOptions.value.format, () => {
  // 某些格式不支持某些选项
  if (compressionOptions.value.format === 'tar') {
    compressionOptions.value.password = ''
    compressionOptions.value.splitArchive = false
    compressionOptions.value.createSolidArchive = false
  }

  if (compressionOptions.value.format === 'gz' || compressionOptions.value.format === 'bz2') {
    compressionOptions.value.splitArchive = false
  }
})
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