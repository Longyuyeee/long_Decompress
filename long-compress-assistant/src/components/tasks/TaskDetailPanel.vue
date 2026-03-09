<template>
  <div class="task-detail-panel">
    <!-- 头部：任务标题和状�?-->
    <div class="mb-6">
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center space-x-3">
          <!-- 状态图�?-->
          <div
            class="w-10 h-10 rounded-full flex items-center justify-center flex-shrink-0"
            :class="statusClasses[task.status]"
          >
            <i :class="statusIcons[task.status]" class="text-lg"></i>
          </div>
          <div>
            <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
              {{ task.fileName || `任务 ${task.id.substring(0, 8)}` }}
            </h2>
            <div class="flex items-center space-x-3 mt-1">
              <span
                class="px-3 py-1 rounded-full text-sm font-medium"
                :class="statusBadgeClasses[task.status]"
              >
                {{ statusLabels[task.status] }}
              </span>
              <span class="text-gray-600 dark:text-gray-400 text-sm">
                {{ formatTime(task.createdAt) }}
              </span>
            </div>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="flex items-center space-x-2">
          <button
            v-if="task.status === 'error'"
            @click="handleRetry"
            class="glass-button px-4 py-2 text-green-600 hover:text-green-700"
            :disabled="isProcessing"
          >
            <i class="pi pi-replay mr-2"></i>
            重试
          </button>
          <button
            v-if="task.status === 'processing'"
            @click="handleCancel"
            class="glass-button px-4 py-2 text-red-600 hover:text-red-700"
            :disabled="isProcessing"
          >
            <i class="pi pi-times mr-2"></i>
            取消
          </button>
          <button
            v-if="task.status === 'completed'"
            @click="handleOpenOutput"
            class="glass-button px-4 py-2 text-primary hover:text-primary-dark"
            :disabled="isProcessing"
          >
            <i class="pi pi-folder-open mr-2"></i>
            打开输出目录
          </button>
          <button
            @click="handleClose"
            class="glass-button px-4 py-2 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200"
          >
            <i class="pi pi-times mr-2"></i>
            关闭
          </button>
        </div>
      </div>

      <!-- 进度条（处理中状态） -->
      <div v-if="task.status === 'processing'" class="mt-4">
        <div class="flex justify-between text-sm mb-2">
          <span class="text-gray-700 dark:text-gray-300">解压进度</span>
          <span class="font-medium">{{ task.progress }}%</span>
        </div>
        <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3">
          <div
            class="bg-primary h-3 rounded-full transition-all duration-300"
            :style="{ width: task.progress + '%' }"
          ></div>
        </div>
        <div class="flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-2">
          <span>开始时�? {{ formatTime(task.startTime) }}</span>
          <span v-if="estimatedTimeRemaining">预计剩余: {{ estimatedTimeRemaining }}</span>
        </div>
      </div>
    </div>

    <!-- 主要内容区域 -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- 左侧：基本信�?-->
      <div class="lg:col-span-2 space-y-6">
        <!-- 文件信息 -->
        <div class="glass-card">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">文件信息</h3>
          <div class="space-y-4">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  文件�?
                </label>
                <div class="flex items-center p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                  <i class="pi pi-file text-gray-500 mr-3"></i>
                  <span class="text-gray-900 dark:text-white truncate">{{ task.fileName }}</span>
                </div>
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  文件路径
                </label>
                <div class="flex items-center p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                  <i class="pi pi-folder text-gray-500 mr-3"></i>
                  <span class="text-gray-900 dark:text-white truncate" :title="task.filePath">
                    {{ getShortPath(task.filePath) }}
                  </span>
                </div>
              </div>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  输出目录
                </label>
                <div class="flex items-center p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                  <i class="pi pi-folder-open text-gray-500 mr-3"></i>
                  <span class="text-gray-900 dark:text-white truncate" :title="task.outputPath">
                    {{ getShortPath(task.outputPath) }}
                  </span>
                </div>
              </div>
              <div v-if="task.password">
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  密码保护
                </label>
                <div class="flex items-center p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                  <i class="pi pi-lock text-gray-500 mr-3"></i>
                  <span class="text-gray-900 dark:text-white">已设置密�?/span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 解压选项 -->
        <div class="glass-card">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">解压选项</h3>
          <div class="grid grid-cols-2 sm:grid-cols-3 gap-4">
            <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="flex items-center mb-2">
                <i class="pi pi-sitemap text-primary mr-2"></i>
                <span class="font-medium text-gray-900 dark:text-white">目录结构</span>
              </div>
              <span class="text-sm text-gray-600 dark:text-gray-400">
                {{ task.options.keepStructure ? '保持原结�? : '扁平�? }}
              </span>
            </div>
            <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="flex items-center mb-2">
                <i class="pi pi-copy text-primary mr-2"></i>
                <span class="font-medium text-gray-900 dark:text-white">覆盖策略</span>
              </div>
              <span class="text-sm text-gray-600 dark:text-gray-400">
                {{ getOverwriteStrategyLabel(task.options.overwrite) }}
              </span>
            </div>
            <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="flex items-center mb-2">
                <i class="pi pi-trash text-primary mr-2"></i>
                <span class="font-medium text-gray-900 dark:text-white">原文件处�?/span>
              </div>
              <span class="text-sm text-gray-600 dark:text-gray-400">
                {{ task.options.deleteAfter ? '解压后删�? : '保留原文�? }}
              </span>
            </div>
          </div>
        </div>

        <!-- 错误信息（失败状态） -->
        <div v-if="task.status === 'error' && task.error" class="glass-card border-l-4 border-red-500">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">错误信息</h3>
          <div class="p-4 rounded-lg bg-red-50 dark:bg-red-900/20">
            <div class="flex items-start">
              <i class="pi pi-exclamation-triangle text-red-500 mt-0.5 mr-3"></i>
              <div class="flex-1">
                <p class="font-medium text-red-700 dark:text-red-300 mb-2">解压失败原因</p>
                <p class="text-red-600 dark:text-red-400">{{ task.error }}</p>

                <!-- 解决方案建议 -->
                <div v-if="getErrorSolution(task.error)" class="mt-4 p-3 rounded-lg bg-yellow-50 dark:bg-yellow-900/20">
                  <div class="flex items-start">
                    <i class="pi pi-lightbulb text-yellow-500 mt-0.5 mr-2"></i>
                    <div>
                      <p class="font-medium text-yellow-700 dark:text-yellow-300 mb-1">建议解决方案</p>
                      <p class="text-yellow-600 dark:text-yellow-400 text-sm">
                        {{ getErrorSolution(task.error) }}
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 解压结果（完成状态） -->
        <div v-if="task.status === 'completed'" class="glass-card border-l-4 border-green-500">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">解压结果</h3>
          <div class="space-y-4">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div class="p-3 rounded-lg bg-green-50 dark:bg-green-900/20">
                <div class="flex items-center">
                  <i class="pi pi-check-circle text-green-500 mr-3"></i>
                  <div>
                    <p class="font-medium text-green-700 dark:text-green-300">解压成功</p>
                    <p class="text-green-600 dark:text-green-400 text-sm mt-1">
                      文件已成功解压到目标目录
                    </p>
                  </div>
                </div>
              </div>
              <div class="p-3 rounded-lg bg-blue-50 dark:bg-blue-900/20">
                <div class="flex items-center">
                  <i class="pi pi-clock text-blue-500 mr-3"></i>
                  <div>
                    <p class="font-medium text-blue-700 dark:text-blue-300">解压时长</p>
                    <p class="text-blue-600 dark:text-blue-400 text-sm mt-1">
                      {{ formatDuration(task.startTime, task.endTime) }}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 右侧：统计信息和操作 -->
      <div class="space-y-6">
        <!-- 时间�?-->
        <div class="glass-card">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">时间�?/h3>
          <div class="space-y-4">
            <div class="flex items-start">
              <div class="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center flex-shrink-0 mr-3">
                <i class="pi pi-plus text-primary text-sm"></i>
              </div>
              <div>
                <p class="font-medium text-gray-900 dark:text-white">任务创建</p>
                <p class="text-gray-600 dark:text-gray-400 text-sm">
                  {{ formatTime(task.createdAt) }}
                </p>
              </div>
            </div>

            <div v-if="task.startTime" class="flex items-start">
              <div class="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center flex-shrink-0 mr-3">
                <i class="pi pi-play text-primary text-sm"></i>
              </div>
              <div>
                <p class="font-medium text-gray-900 dark:text-white">开始解�?/p>
                <p class="text-gray-600 dark:text-gray-400 text-sm">
                  {{ formatTime(task.startTime) }}
                </p>
              </div>
            </div>

            <div v-if="task.endTime" class="flex items-start">
              <div class="w-8 h-8 rounded-full bg-green-500/10 flex items-center justify-center flex-shrink-0 mr-3">
                <i class="pi pi-check text-green-500 text-sm"></i>
              </div>
              <div>
                <p class="font-medium text-gray-900 dark:text-white">解压完成</p>
                <p class="text-gray-600 dark:text-gray-400 text-sm">
                  {{ formatTime(task.endTime) }}
                </p>
              </div>
            </div>
          </div>
        </div>

        <!-- 快速操�?-->
        <div class="glass-card">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">快速操�?/h3>
          <div class="space-y-3">
            <button
              @click="handleCopyPath"
              class="w-full glass-button text-left px-4 py-3"
              :disabled="isProcessing"
            >
              <i class="pi pi-copy mr-3"></i>
              <div>
                <p class="font-medium">复制输出路径</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">复制到剪贴板</p>
              </div>
            </button>
            <button
              @click="handleShowInExplorer"
              class="w-full glass-button text-left px-4 py-3"
              :disabled="isProcessing"
            >
              <i class="pi pi-external-link mr-3"></i>
              <div>
                <p class="font-medium">在资源管理器中显�?/p>
                <p class="text-xs text-gray-500 dark:text-gray-400">打开文件所在位�?/p>
              </div>
            </button>
            <button
              @click="handleExportLog"
              class="w-full glass-button text-left px-4 py-3"
              :disabled="isProcessing"
            >
              <i class="pi pi-download mr-3"></i>
              <div>
                <p class="font-medium">导出日志</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">保存任务日志文件</p>
              </div>
            </button>
            <button
              @click="handleDelete"
              class="w-full glass-button text-left px-4 py-3 text-red-600 hover:text-red-700"
              :disabled="isProcessing"
            >
              <i class="pi pi-trash mr-3"></i>
              <div>
                <p class="font-medium">删除任务记录</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">从历史记录中删除</p>
              </div>
            </button>
          </div>
        </div>

        <!-- 技术信�?-->
        <div class="glass-card">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">技术信�?/h3>
          <div class="space-y-3 text-sm">
            <div class="flex justify-between">
              <span class="text-gray-600 dark:text-gray-400">任务ID</span>
              <span class="font-mono text-gray-900 dark:text-white">{{ task.id.substring(0, 12) }}...</span>
            </div>
            <div class="flex justify-between">
              <span class="text-gray-600 dark:text-gray-400">文件ID</span>
              <span class="font-mono text-gray-900 dark:text-white">{{ task.fileId.substring(0, 12) }}...</span>
            </div>
            <div class="flex justify-between">
              <span class="text-gray-600 dark:text-gray-400">API版本</span>
              <span class="text-gray-900 dark:text-white">v1.0.0</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { DecompressTask } from '@/stores'

// 定义组件属�?
interface Props {
  task: DecompressTask
  isProcessing?: boolean
}

// 定义组件事件
interface Emits {
  (e: 'close'): void
  (e: 'retry', taskId: string): void
  (e: 'cancel', taskId: string): void
  (e: 'open-output', path: string): void
  (e: 'copy-path', path: string): void
  (e: 'show-in-explorer', path: string): void
  (e: 'export-log', taskId: string): void
  (e: 'delete', taskId: string): void
}

const props = withDefaults(defineProps<Props>(), {
  isProcessing: false
})

const emit = defineEmits<Emits>()

// 状态标�?
const statusLabels = {
  pending: '等待�?,
  processing: '进行�?,
  completed: '已完�?,
  error: '失败'
}

// 状态样�?
const statusClasses = {
  pending: 'bg-gray-100 dark:bg-gray-800 text-gray-500',
  processing: 'bg-primary/10 text-primary',
  completed: 'bg-green-500/10 text-green-500',
  error: 'bg-red-500/10 text-red-500'
}

const statusIcons = {
  pending: 'pi pi-clock',
  processing: 'pi pi-spin pi-spinner',
  completed: 'pi pi-check',
  error: 'pi pi-times'
}

const statusBadgeClasses = {
  pending: 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300',
  processing: 'bg-primary/10 text-primary',
  completed: 'bg-green-500/10 text-green-500',
  error: 'bg-red-500/10 text-red-500'
}

// 计算属�?
const estimatedTimeRemaining = computed(() => {
  if (props.task.status !== 'processing' || !props.task.startTime) return null

  const elapsed = Date.now() - props.task.startTime.getTime()
  const progress = props.task.progress || 1
  const totalEstimated = elapsed / (progress / 100)
  const remaining = totalEstimated - elapsed

  if (remaining < 60000) return `${Math.ceil(remaining / 1000)}秒`
  return `${Math.ceil(remaining / 60000)}分钟`
})

// 方法
const formatTime = (date?: Date): string => {
  if (!date) return '未知时间'
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  })
}

const getShortPath = (path: string): string => {
  if (!path) return '未设�?
  if (path.length <= 40) return path
  return '...' + path.slice(-37)
}

const getOverwriteStrategyLabel = (overwrite: boolean): string => {
  return overwrite ? '覆盖已存在文�? : '询问用户'
}

const formatDuration = (startTime?: Date, endTime?: Date): string => {
  if (!startTime || !endTime) return '未知时长'
  const duration = endTime.getTime() - startTime.getTime()
  const seconds = Math.floor(duration / 1000)

  if (seconds < 60) return `${seconds}秒`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}分钟`
  const hours = Math.floor(minutes / 60)
  return `${hours}小时${minutes % 60}分钟`
}

const getErrorSolution = (error: string): string => {
  const errorLower = error.toLowerCase()

  if (errorLower.includes('密码') || errorLower.includes('password')) {
    return '请检查输入的密码是否正确，或尝试使用密码本功能查找密码�?
  }

  if (errorLower.includes('损坏') || errorLower.includes('corrupt')) {
    return '文件可能已损坏，请尝试使用其他解压软件或重新下载文件�?
  }

  if (errorLower.includes('空间') || errorLower.includes('space')) {
    return '磁盘空间不足，请清理磁盘空间后重试�?
  }

  if (errorLower.includes('权限') || errorLower.includes('permission')) {
    return '权限不足，请以管理员身份运行程序或检查文件权限�?
  }

  return '请检查文件格式是否正确，或尝试使用其他解压选项�?
}

// 事件处理
const handleClose = () => {
  emit('close')
}

const handleRetry = () => {
  emit('retry', props.task.id)
}

const handleCancel = () => {
  emit('cancel', props.task.id)
}

const handleOpenOutput = () => {
  emit('open-output', props.task.outputPath)
}

const handleCopyPath = () => {
  emit('copy-path', props.task.outputPath)
}

const handleShowInExplorer = () => {
  emit('show-in-explorer', props.task.filePath)
}

const handleExportLog = () => {
  emit('export-log', props.task.id)
}

const handleDelete = () => {
  emit('delete', props.task.id)
}

// 暴露方法给父组件
defineExpose({
  getTaskInfo: () => props.task
})
</script>

<style scoped>
.task-detail-panel {
  @apply space-y-6;
}

/* 自定义滚动条 */
.task-detail-panel ::-webkit-scrollbar {
  width: 6px;
}

.task-detail-panel ::-webkit-scrollbar-track {
  @apply bg-transparent;
}

.task-detail-panel ::-webkit-scrollbar-thumb {
  @apply bg-gray-300 dark:bg-gray-700 rounded-full;
}

.task-detail-panel ::-webkit-scrollbar-thumb:hover {
  @apply bg-gray-400 dark:bg-gray-600;
}

/* 动画效果 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* 响应式调�?*/
@media (max-width: 1024px) {
  .task-detail-panel {
    @apply space-y-4;
  }
}
</style>
