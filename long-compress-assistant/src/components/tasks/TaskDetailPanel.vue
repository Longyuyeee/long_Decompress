<template>
  <div class="task-detail-panel">
    <!-- 头部：任务标题和状态 -->
    <div class="mb-6">
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center space-x-3">
          <!-- 状态图标 -->
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
          <span class="text-gray-700 dark:text-gray-300">进度</span>
          <span class="font-medium">{{ task.progress }}%</span>
        </div>
        <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3">
          <div
            class="bg-primary h-3 rounded-full transition-all duration-300"
            :style="{ width: task.progress + '%' }"
          ></div>
        </div>
        <div class="flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-2">
          <span>开始时间: {{ formatTime(task.startTime) }}</span>
          <span v-if="estimatedTimeRemaining">预计剩余: {{ estimatedTimeRemaining }}</span>
        </div>
      </div>
    </div>

    <!-- 主要内容区域 -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <div class="lg:col-span-2 space-y-6">
        <!-- 文件信息 -->
        <div class="glass-card">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">文件信息</h3>
          <div class="space-y-4">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  文件名
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
                {{ task.options.keepStructure ? '保留' : '扁平' }}
              </span>
            </div>
            <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="flex items-center mb-2">
                <i class="pi pi-copy text-primary mr-2"></i>
                <span class="font-medium text-gray-900 dark:text-white">覆盖策略</span>
              </div>
              <span class="text-sm text-gray-600 dark:text-gray-400">
                {{ task.options.overwrite ? '覆盖' : '跳过' }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { DecompressTask } from '@/stores'

export interface Props {
  task: DecompressTask
  isProcessing?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  isProcessing: false
})

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'retry', taskId: string): void
  (e: 'cancel', taskId: string): void
  (e: 'open-output', path: string): void
  (e: 'copy-path', path: string): void
  (e: 'show-in-explorer', path: string): void
  (e: 'export-log', taskId: string): void
  (e: 'delete', taskId: string): void
}>()

const statusLabels = {
  pending: '等待中',
  processing: '进行中',
  completed: '已完成',
  error: '失败'
}

const statusClasses = {
  pending: 'bg-gray-100 text-gray-500',
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
  pending: 'bg-gray-100 text-gray-800',
  processing: 'bg-primary/10 text-primary',
  completed: 'bg-green-500/10 text-green-500',
  error: 'bg-red-500/10 text-red-500'
}

const estimatedTimeRemaining = computed(() => {
  if (props.task.status !== 'processing' || !props.task.startTime) return null
  return '计算中...'
})

const formatTime = (date?: Date) => date ? new Date(date).toLocaleString() : '未知'
const getShortPath = (path: string) => path ? (path.length > 40 ? '...' + path.slice(-37) : path) : '未设置'

const handleClose = () => emit('close')
const handleRetry = () => emit('retry', props.task.id)
const handleCancel = () => emit('cancel', props.task.id)
const handleOpenOutput = () => emit('open-output', props.task.outputPath)

defineExpose({
  getTaskInfo: () => props.task
})
</script>
