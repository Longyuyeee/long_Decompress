<template>
  <div class="max-w-7xl mx-auto">
    <!-- 页面标题 -->
    <div class="mb-4 sm:mb-6 lg:mb-8">
      <h1 class="text-xl sm:text-2xl font-bold text-gray-900 dark:text-white">文件解压</h1>
      <p class="text-gray-600 dark:text-gray-400 text-sm sm:text-base">选择文件并配置解压选项</p>
    </div>

      <main class="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
        <!-- 左侧：文件选择和配置 -->
        <div class="lg:col-span-2 space-y-6">
          <!-- 文件选择组件 -->
          <div class="glass-card">
            <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">选择文件</h2>

            <EnhancedFileDropzone
              :multiple="true"
              :maxSize="1024 * 1024 * 1024" <!-- 1GB -->
              :maxFiles="20"
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
              </div>

              <!-- 加密文件提醒 -->
              <div v-if="hasEncryptedFiles" class="mt-4 p-3 rounded-lg bg-warning/10 border border-warning/20">
                <div class="flex items-center">
                  <i class="pi pi-shield text-warning mr-2"></i>
                  <span class="text-warning text-sm">
                    检测到 {{ encryptedFilesCount }} 个加密文件，解压时需要输入密码
                  </span>
                </div>
              </div>
            </div>
          </div>

          <!-- 解压配置 -->
          <div class="glass-card">
            <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">解压配置</h2>

            <div class="space-y-6">
              <!-- 输出目录 -->
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  输出目录
                </label>
                <div class="flex space-x-2">
                  <input type="text" v-model="outputPath"
                         class="flex-1 glass-input"
                         placeholder="选择解压输出目录"
                         @click="selectOutputPath">
                  <button @click="selectOutputPath" class="glass-button px-4" :aria-label="`选择输出目录`">
                    <i class="pi pi-folder-open"></i>
                  </button>
                </div>
                <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  留空则解压到原文件所在目录
                </p>
              </div>

              <!-- 密码设置 -->
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  解压密码设置
                </label>
                <div class="space-y-3">
                  <div class="relative">
                    <input :type="showPassword ? 'text' : 'password'" v-model="password"
                           class="w-full glass-input pr-10"
                           placeholder="输入解压密码"
                           aria-label="解压密码">
                    <button @click="showPassword = !showPassword"
                            class="absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-500 focus:outline-none focus:ring-2 focus:ring-primary rounded p-1"
                            :aria-label="showPassword ? '隐藏密码' : '显示密码'">
                      <i :class="showPassword ? 'pi pi-eye-slash' : 'pi pi-eye'" aria-hidden="true"></i>
                    </button>
                  </div>

                  <!-- 密码尝试设置 -->
                  <div v-if="password" class="pl-4 border-l-2 border-primary/30">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                      密码尝试设置
                    </label>
                    <div class="space-y-2">
                      <label class="flex items-center">
                        <input type="checkbox" v-model="passwordOptions.rememberForSession" class="mr-3">
                        <span class="text-gray-700 dark:text-gray-300 text-sm">本次会话记住密码</span>
                      </label>
                      <label class="flex items-center">
                        <input type="checkbox" v-model="passwordOptions.autoTryCommon" class="mr-3">
                        <span class="text-gray-700 dark:text-gray-300 text-sm">自动尝试常见密码</span>
                      </label>
                      <div v-if="passwordOptions.autoTryCommon" class="ml-6">
                        <label class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
                          最大尝试次数
                        </label>
                        <input type="range" v-model="passwordOptions.maxAttempts" min="1" max="10" step="1"
                               class="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-lg">
                        <div class="flex justify-between text-xs text-gray-500 dark:text-gray-400">
                          <span>1次</span>
                          <span>{{ passwordOptions.maxAttempts }}次</span>
                          <span>10次</span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <!-- 解压选项 -->
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  解压选项
                </label>
                <div class="space-y-3">
                  <label class="flex items-center">
                    <input type="checkbox" v-model="options.keepStructure" class="mr-3">
                    <span class="text-gray-700 dark:text-gray-300">保持目录结构</span>
                  </label>

                  <!-- 文件覆盖策略 -->
                  <div class="pl-4 border-l-2 border-gray-200 dark:border-gray-700">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                      文件覆盖策略
                    </label>
                    <div class="space-y-2">
                      <label class="flex items-center">
                        <input type="radio" v-model="options.overwriteStrategy" value="ask" class="mr-3">
                        <span class="text-gray-700 dark:text-gray-300 text-sm">每次询问</span>
                      </label>
                      <label class="flex items-center">
                        <input type="radio" v-model="options.overwriteStrategy" value="overwrite" class="mr-3">
                        <span class="text-gray-700 dark:text-gray-300 text-sm">总是覆盖</span>
                      </label>
                      <label class="flex items-center">
                        <input type="radio" v-model="options.overwriteStrategy" value="skip" class="mr-3">
                        <span class="text-gray-700 dark:text-gray-300 text-sm">跳过已存在文件</span>
                      </label>
                      <label class="flex items-center">
                        <input type="radio" v-model="options.overwriteStrategy" value="rename" class="mr-3">
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
                        <input type="checkbox" v-model="options.deleteAfter" class="mr-3">
                        <span class="text-gray-700 dark:text-gray-300 text-sm">解压后删除原文件</span>
                      </label>
                      <label class="flex items-center">
                        <input type="checkbox" v-model="options.preserveTimestamps" class="mr-3">
                        <span class="text-gray-700 dark:text-gray-300 text-sm">保留文件时间戳</span>
                      </label>
                      <label class="flex items-center">
                        <input type="checkbox" v-model="options.skipCorrupted" class="mr-3">
                        <span class="text-gray-700 dark:text-gray-300 text-sm">跳过损坏的文件</span>
                      </label>
                      <label class="flex items-center">
                        <input type="checkbox" v-model="options.extractOnlyNewer" class="mr-3">
                        <span class="text-gray-700 dark:text-gray-300 text-sm">仅解压较新的文件</span>
                      </label>
                      <label class="flex items-center">
                        <input type="checkbox" v-model="options.createSubdirectory" class="mr-3">
                        <span class="text-gray-700 dark:text-gray-300 text-sm">为每个压缩包创建子目录</span>
                      </label>
                    </div>
                  </div>

                  <!-- 解压过滤器 -->
                  <div class="pl-4 border-l-2 border-gray-200 dark:border-gray-700" v-if="selectedFiles.length > 0">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                      文件过滤器
                    </label>
                    <div class="space-y-2">
                      <input type="text" v-model="options.fileFilter"
                             class="w-full glass-input text-sm"
                             placeholder="例如: *.txt, *.jpg, document*">
                      <p class="text-xs text-gray-500 dark:text-gray-400">
                        使用通配符过滤要解压的文件，多个模式用逗号分隔
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 右侧：操作面板 -->
        <div class="space-y-6">
          <!-- 开始解压按钮 -->
          <div class="glass-card">
            <button @click="startDecompress"
                    :disabled="!canStart || isProcessing"
                    class="w-full glass-button-primary py-4 text-lg font-semibold flex items-center justify-center"
                    :class="{ 'opacity-50 cursor-not-allowed': !canStart || isProcessing }">
              <i v-if="isProcessing" class="pi pi-spin pi-spinner mr-3"></i>
              <i v-else class="pi pi-play mr-3"></i>
              {{ isProcessing ? '解压中...' : '开始解压' }}
            </button>
            <p class="text-gray-500 dark:text-gray-400 text-sm mt-3 text-center">
              {{ canStart ? '点击开始解压选中的文件' : '请先选择要解压的文件' }}
            </p>
          </div>

          <!-- 进度显示 -->
          <div class="glass-card" v-if="isProcessing || progressTasks.some(task => task.status !== 'pending')">
            <h3 class="font-semibold text-gray-900 dark:text-white mb-4">解压进度</h3>

            <div class="space-y-4 max-h-60 overflow-y-auto pr-2">
              <div v-for="task in progressTasks" :key="task.id" class="space-y-2">
                <div class="flex justify-between items-center text-sm">
                  <div class="flex items-center min-w-0">
                    <span class="text-gray-700 dark:text-gray-300 truncate mr-2">{{ task.fileName }}</span>
                    <span v-if="task.status === 'completed'" class="text-xs px-2 py-0.5 rounded-full bg-green-500/10 text-green-500">
                      完成
                    </span>
                    <span v-else-if="task.status === 'failed'" class="text-xs px-2 py-0.5 rounded-full bg-red-500/10 text-red-500">
                      失败
                    </span>
                    <span v-else-if="task.status === 'processing'" class="text-xs px-2 py-0.5 rounded-full bg-primary/10 text-primary">
                      处理中
                    </span>
                  </div>
                  <span class="font-medium">{{ task.progress }}%</span>
                </div>
                <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                  <div
                    class="h-2 rounded-full transition-all duration-300"
                    :class="{
                      'bg-primary': task.status === 'processing',
                      'bg-green-500': task.status === 'completed',
                      'bg-red-500': task.status === 'failed'
                    }"
                    :style="{ width: task.progress + '%' }"
                  ></div>
                </div>
                <div v-if="task.error" class="text-xs text-red-500 bg-red-500/10 p-2 rounded">
                  <i class="pi pi-exclamation-triangle mr-1"></i>
                  {{ task.error }}
                </div>
              </div>
            </div>

            <div class="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700 space-y-3">
              <!-- 当前文件进度 -->
              <div v-if="isProcessing">
                <div class="flex justify-between text-sm mb-1">
                  <span class="text-gray-600 dark:text-gray-400">当前文件进度</span>
                  <span class="font-medium">{{ currentFileProgress }}%</span>
                </div>
                <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                  <div
                    class="bg-primary h-2 rounded-full transition-all duration-300"
                    :style="{ width: currentFileProgress + '%' }"
                  ></div>
                </div>
              </div>

              <!-- 总进度 -->
              <div>
                <div class="flex justify-between text-sm">
                  <span class="text-gray-600 dark:text-gray-400">总进度</span>
                  <span class="font-medium text-gray-900 dark:text-white">{{ totalProgress }}%</span>
                </div>
                <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3 mt-1">
                  <div
                    class="bg-primary h-3 rounded-full transition-all duration-300"
                    :style="{ width: totalProgress + '%' }"
                  ></div>
                </div>
              </div>

              <!-- 统计信息 -->
              <div class="grid grid-cols-2 gap-3 text-sm">
                <div class="p-2 rounded-lg bg-gray-50 dark:bg-gray-800">
                  <div class="flex justify-between">
                    <span class="text-gray-600 dark:text-gray-400">已处理</span>
                    <span class="font-medium">{{ processedFiles }}/{{ selectedFiles.length }}</span>
                  </div>
                </div>
                <div class="p-2 rounded-lg bg-gray-50 dark:bg-gray-800">
                  <div class="flex justify-between">
                    <span class="text-gray-600 dark:text-gray-400">剩余时间</span>
                    <span class="font-medium">{{ estimatedTimeRemaining }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 解压预设 -->
          <div class="glass-card">
            <h3 class="font-semibold text-gray-900 dark:text-white mb-4">解压预设</h3>
            <div class="space-y-3">
              <button
                v-for="preset in decompressPresets"
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
              <button class="w-full glass-button text-left px-4 py-3" @click="clearAllFiles">
                <i class="pi pi-trash mr-3"></i>
                清空所有文件
              </button>
            </div>
          </div>

          <!-- 格式支持提示 -->
          <div class="glass-card">
            <h3 class="font-semibold text-gray-900 dark:text-white mb-3">支持格式</h3>
            <div class="grid grid-cols-3 gap-2">
              <div class="text-center p-2 rounded-lg bg-primary/10">
                <i class="pi pi-file text-primary text-lg block mb-1"></i>
                <span class="text-xs text-gray-700 dark:text-gray-300">ZIP</span>
              </div>
              <div class="text-center p-2 rounded-lg bg-accent/10">
                <i class="pi pi-file text-accent text-lg block mb-1"></i>
                <span class="text-xs text-gray-700 dark:text-gray-300">RAR</span>
              </div>
              <div class="text-center p-2 rounded-lg bg-green-500/10">
                <i class="pi pi-file text-green-500 text-lg block mb-1"></i>
                <span class="text-xs text-gray-700 dark:text-gray-300">7Z</span>
              </div>
              <div class="text-center p-2 rounded-lg bg-purple-500/10">
                <i class="pi pi-file text-purple-500 text-lg block mb-1"></i>
                <span class="text-xs text-gray-700 dark:text-gray-300">TAR</span>
              </div>
              <div class="text-center p-2 rounded-lg bg-yellow-500/10">
                <i class="pi pi-file text-yellow-500 text-lg block mb-1"></i>
                <span class="text-xs text-gray-700 dark:text-gray-300">GZ</span>
              </div>
              <div class="text-center p-2 rounded-lg bg-red-500/10">
                <i class="pi pi-file text-red-500 text-lg block mb-1"></i>
                <span class="text-xs text-gray-700 dark:text-gray-300">BZ2</span>
              </div>
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
const password = ref('')
const showPassword = ref(false)
const isProcessing = ref(false)
const processedFiles = ref(0)
const currentFileProgress = ref(0)

// 解压选项
const options = ref({
  keepStructure: true,
  overwriteStrategy: 'ask' as 'ask' | 'overwrite' | 'skip' | 'rename',
  deleteAfter: false,
  preserveTimestamps: true,
  skipCorrupted: false,
  extractOnlyNewer: false,
  createSubdirectory: false,
  fileFilter: ''
})

// 密码选项
const passwordOptions = ref({
  rememberForSession: false,
  autoTryCommon: false,
  maxAttempts: 3
})

// 进度任务
const progressTasks = ref<Array<{
  id: number
  fileName: string
  progress: number
  status: 'pending' | 'processing' | 'completed' | 'failed'
  error?: string
}>>([])

// 解压预设
const decompressPresets = [
  { id: 1, name: '快速解压', icon: 'pi pi-bolt', color: 'text-primary', description: '最快速度，基本选项' },
  { id: 2, name: '安全解压', icon: 'pi pi-shield', color: 'text-green-500', description: '验证文件完整性' },
  { id: 3, name: '批量解压', icon: 'pi pi-copy', color: 'text-purple-500', description: '多个文件批量处理' },
  { id: 4, name: '深度扫描', icon: 'pi pi-search', color: 'text-blue-500', description: '扫描并修复损坏文件' }
]

// 计算属性
const canStart = computed(() => selectedFiles.value.length > 0)

const totalProgress = computed(() => {
  if (progressTasks.value.length === 0) return 0
  const completed = progressTasks.value.filter(task => task.status === 'completed').length
  return Math.round((completed / progressTasks.value.length) * 100)
})

const hasEncryptedFiles = computed(() => {
  return selectedFiles.value.some(file => file.encrypted)
})

const encryptedFilesCount = computed(() => {
  return selectedFiles.value.filter(file => file.encrypted).length
})

const estimatedTimeRemaining = computed(() => {
  if (totalProgress.value === 0) return '计算中...'
  if (totalProgress.value >= 100) return '已完成'

  const completedTasks = progressTasks.value.filter(task => task.status === 'completed').length
  const totalTasks = progressTasks.value.length

  if (completedTasks === 0) return '等待开始...'

  // 简单估算：假设每个任务平均需要5秒
  const estimatedSeconds = (totalTasks - completedTasks) * 5
  if (estimatedSeconds < 60) return `${estimatedSeconds}秒`
  return `${Math.ceil(estimatedSeconds / 60)}分钟`
})

// 方法
const handleFilesSelected = (files: FileItem[]) => {
  console.log('文件选择:', files)
  selectedFiles.value = [...selectedFiles.value, ...files]

  // 为新文件创建进度任务
  files.forEach(file => {
    progressTasks.value.push({
      id: Date.now() + Math.random(),
      fileName: file.name,
      progress: 0,
      status: 'pending'
    })
  })
}

const handleFileRemoved = (fileId: string) => {
  console.log('文件移除:', fileId)
  const removedFile = selectedFiles.value.find(file => file.id === fileId)
  if (removedFile) {
    // 移除对应的进度任务
    progressTasks.value = progressTasks.value.filter(task =>
      task.fileName !== removedFile.name || task.status === 'completed'
    )
  }
  selectedFiles.value = selectedFiles.value.filter(file => file.id !== fileId)
}

const handleFileError = (error: string) => {
  console.error('文件选择错误:', error)
  alert(`文件选择错误: ${error}`)
}

const clearAllFiles = () => {
  selectedFiles.value = []
  progressTasks.value = progressTasks.value.filter(task => task.status === 'completed')
}

const formatTotalSize = (): string => {
  const totalBytes = selectedFiles.value.reduce((sum, file) => sum + file.size, 0)

  if (totalBytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(totalBytes) / Math.log(k))
  return parseFloat((totalBytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
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
  // 根据预设应用不同的配置
  switch (preset.id) {
    case 1: // 快速解压
      options.value.overwriteStrategy = 'overwrite'
      options.value.skipCorrupted = true
      options.value.extractOnlyNewer = false
      break
    case 2: // 安全解压
      options.value.overwriteStrategy = 'ask'
      options.value.skipCorrupted = false
      options.value.preserveTimestamps = true
      break
    case 3: // 批量解压
      options.value.createSubdirectory = true
      options.value.keepStructure = true
      break
    case 4: // 深度扫描
      options.value.skipCorrupted = false
      break
  }
}

const startDecompress = async () => {
  if (!canStart.value) return

  // 如果有加密文件但未输入密码，提示用户
  if (hasEncryptedFiles.value && !password.value) {
    const confirmProceed = confirm('检测到加密文件，但未输入密码。是否继续？\n\n加密文件解压需要密码，如果继续可能会解压失败。')
    if (!confirmProceed) return
  }

  // 验证输出路径（如果提供了）
  if (outputPath.value && !outputPath.value.trim()) {
    alert('请输入有效的输出路径或留空使用默认路径')
    return
  }

  isProcessing.value = true
  processedFiles.value = 0
  currentFileProgress.value = 0

  // 更新所有任务状态为处理中
  progressTasks.value = progressTasks.value.map(task => ({
    ...task,
    status: task.status === 'pending' ? 'processing' : task.status,
    progress: task.status === 'pending' ? 0 : task.progress
  }))

  try {
    // 准备文件路径
    const filePaths = selectedFiles.value.map(file => file.path)

    // 调用Tauri解压API
    for (let i = 0; i < filePaths.length; i++) {
      const filePath = filePaths[i]
      const taskIndex = progressTasks.value.findIndex(task =>
        task.fileName === selectedFiles.value[i].name && task.status === 'processing'
      )

      if (taskIndex !== -1) {
        // 更新当前文件进度
        currentFileProgress.value = 0
        const progressInterval = setInterval(() => {
          currentFileProgress.value = Math.min(100, currentFileProgress.value + Math.random() * 20)
          progressTasks.value[taskIndex].progress = currentFileProgress.value
        }, 200)

        try {
          // 调用解压API
          const result = await invoke('extract_file', {
            filePath,
            outputDir: outputPath.value || null,
            password: password.value || null
          })

          console.log(`解压结果 ${filePath}:`, result)

          // 标记任务为完成
          clearInterval(progressInterval)
          progressTasks.value[taskIndex] = {
            ...progressTasks.value[taskIndex],
            progress: 100,
            status: 'completed'
          }
          processedFiles.value++

        } catch (error) {
          console.error(`解压失败 ${filePath}:`, error)
          clearInterval(progressInterval)
          progressTasks.value[taskIndex] = {
            ...progressTasks.value[taskIndex],
            progress: 0,
            status: 'failed',
            error: error instanceof Error ? error.message : '未知错误'
          }
        }
      }

      // 短暂延迟，避免UI卡顿
      await new Promise(resolve => setTimeout(resolve, 100))
    }

    // 检查是否有失败的任务
    const failedTasks = progressTasks.value.filter(task => task.status === 'failed')
    if (failedTasks.length > 0) {
      alert(`解压完成，但有 ${failedTasks.length} 个文件失败`)
    } else {
      alert('解压完成！')
    }

  } catch (error) {
    console.error('解压过程出错:', error)
    alert(`解压过程出错: ${error}`)
  } finally {
    isProcessing.value = false
    currentFileProgress.value = 0
  }
}

// 监听文件选择变化
watch(selectedFiles, (newFiles) => {
  // 移除已不存在的文件的进度任务
  const existingFileNames = newFiles.map(file => file.name)
  progressTasks.value = progressTasks.value.filter(task =>
    task.status === 'completed' || existingFileNames.includes(task.fileName)
  )
})
</script>

<style scoped>
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

/* 自定义滚动条 */
.glass-card ::-webkit-scrollbar {
  width: 6px;
}

.glass-card ::-webkit-scrollbar-track {
  @apply bg-transparent;
}

.glass-card ::-webkit-scrollbar-thumb {
  @apply bg-gray-300 dark:bg-gray-700 rounded-full;
}

.glass-card ::-webkit-scrollbar-thumb:hover {
  @apply bg-gray-400 dark:bg-gray-600;
}
</style>