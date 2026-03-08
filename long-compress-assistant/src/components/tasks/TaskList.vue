<template>
  <div class="task-list">
    <!-- 标题和操作栏 -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h2 class="text-xl font-semibold text-gray-900 dark:text-white">解压任务</h2>
        <p class="text-gray-600 dark:text-gray-400 text-sm mt-1">
          共 {{ totalTasks }} 个任务，{{ activeTasks.length }} 个进行中
        </p>
      </div>

      <div class="flex items-center space-x-2">
        <!-- 筛选按钮 -->
        <div class="relative">
          <button
            @click="toggleFilterMenu"
            class="glass-button px-4 py-2 flex items-center space-x-2"
            :class="{ 'bg-primary/10 text-primary': filter !== 'all' }"
            aria-label="筛选任务"
          >
            <i class="pi pi-filter"></i>
            <span class="hidden sm:inline">{{ filterLabels[filter] }}</span>
            <i class="pi pi-chevron-down text-xs"></i>
          </button>

          <!-- 筛选菜单 -->
          <div
            v-if="showFilterMenu"
            class="absolute right-0 mt-2 w-48 rounded-lg shadow-lg glass-effect border border-gray-200 dark:border-gray-700 py-1 z-50"
            @click.stop
          >
            <button
              v-for="option in filterOptions"
              :key="option.value"
              @click="setFilter(option.value)"
              class="w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
              :class="{ 'text-primary bg-primary/10': filter === option.value }"
            >
              {{ option.label }}
            </button>
          </div>
        </div>

        <!-- 排序按钮 -->
        <button
          @click="toggleSortOrder"
          class="glass-button px-4 py-2"
          :title="sortOrder === 'desc' ? '最新优先' : '最旧优先'"
          aria-label="切换排序顺序"
        >
          <i :class="sortOrder === 'desc' ? 'pi pi-sort-amount-down' : 'pi pi-sort-amount-up'"></i>
        </button>

        <!-- 刷新按钮 -->
        <button
          @click="refreshTasks"
          class="glass-button px-4 py-2"
          :class="{ 'animate-spin': isRefreshing }"
          :disabled="isRefreshing"
          title="刷新任务列表"
          aria-label="刷新任务列表"
        >
          <i class="pi pi-refresh"></i>
        </button>

        <!-- 清理已完成任务 -->
        <button
          @click="clearCompletedTasks"
          class="glass-button px-4 py-2 text-red-600 hover:text-red-700"
          :disabled="completedTasks.length === 0"
          title="清理已完成任务"
          aria-label="清理已完成的任务"
        >
          <i class="pi pi-trash"></i>
        </button>
      </div>
    </div>

    <!-- 任务列表 -->
    <div class="space-y-3">
      <!-- 空状态 -->
      <div
        v-if="filteredTasks.length === 0"
        class="glass-card text-center py-12"
      >
        <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-gray-100 dark:bg-gray-800 flex items-center justify-center">
          <i class="pi pi-inbox text-gray-400 text-2xl"></i>
        </div>
        <h3 class="font-medium text-gray-900 dark:text-white mb-2">暂无任务</h3>
        <p class="text-gray-600 dark:text-gray-400 text-sm">
          {{ emptyStateMessage }}
        </p>
      </div>

      <!-- 任务项 -->
      <div
        v-for="task in sortedTasks"
        :key="task.id"
        class="glass-card p-4 transition-all duration-300 hover:shadow-md"
        :class="{
          'border-l-4 border-primary': task.status === 'processing',
          'border-l-4 border-green-500': task.status === 'completed',
          'border-l-4 border-red-500': task.status === 'error',
          'border-l-4 border-gray-300 dark:border-gray-600': task.status === 'pending'
        }"
      >
        <div class="flex items-start justify-between">
          <!-- 任务信息 -->
          <div class="flex-1 min-w-0">
            <div class="flex items-center space-x-3 mb-2">
              <!-- 状态图标 -->
              <div
                class="w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0"
                :class="statusClasses[task.status]"
              >
                <i :class="statusIcons[task.status]"></i>
              </div>

              <!-- 任务标题 -->
              <div class="flex-1 min-w-0">
                <h3 class="font-medium text-gray-900 dark:text-white truncate">
                  {{ getFileName(task.fileId) }}
                </h3>
                <p class="text-gray-600 dark:text-gray-400 text-sm">
                  {{ formatTime(task.startTime) }}
                </p>
              </div>
            </div>

            <!-- 进度条 -->
            <div v-if="task.status === 'processing'" class="mt-3">
              <div class="flex justify-between text-sm mb-1">
                <span class="text-gray-700 dark:text-gray-300">解压进度</span>
                <span class="font-medium">{{ task.progress }}%</span>
              </div>
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                <div
                  class="bg-primary h-2 rounded-full transition-all duration-300"
                  :style="{ width: task.progress + '%' }"
                ></div>
              </div>
            </div>

            <!-- 错误信息 -->
            <div
              v-if="task.status === 'error' && task.error"
              class="mt-3 p-3 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800"
            >
              <div class="flex items-start">
                <i class="pi pi-exclamation-circle text-red-500 mt-0.5 mr-2"></i>
                <div class="flex-1">
                  <p class="text-sm text-red-700 dark:text-red-300 font-medium">解压失败</p>
                  <p class="text-sm text-red-600 dark:text-red-400 mt-1">{{ task.error }}</p>
                </div>
              </div>
            </div>

            <!-- 任务详情 -->
            <div class="mt-3 grid grid-cols-1 sm:grid-cols-2 gap-2 text-sm">
              <div class="flex items-center text-gray-600 dark:text-gray-400">
                <i class="pi pi-folder-open mr-2"></i>
                <span class="truncate" :title="task.outputPath">输出: {{ getShortPath(task.outputPath) }}</span>
              </div>

              <div v-if="task.password" class="flex items-center text-gray-600 dark:text-gray-400">
                <i class="pi pi-lock mr-2"></i>
                <span>密码保护</span>
              </div>

              <div class="flex items-center text-gray-600 dark:text-gray-400">
                <i class="pi pi-cog mr-2"></i>
                <span>选项: {{ formatOptions(task.options) }}</span>
              </div>

              <div v-if="task.endTime" class="flex items-center text-gray-600 dark:text-gray-400">
                <i class="pi pi-clock mr-2"></i>
                <span>完成: {{ formatDuration(task.startTime, task.endTime) }}</span>
              </div>
            </div>
          </div>

          <!-- 操作按钮 -->
          <div class="ml-4 flex flex-col space-y-2">
            <!-- 重试按钮（仅错误状态） -->
            <button
              v-if="task.status === 'error'"
              @click="retryTask(task.id)"
              class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors text-green-600"
              title="重试任务"
              aria-label="重试此任务"
            >
              <i class="pi pi-replay"></i>
            </button>

            <!-- 取消按钮（仅进行中状态） -->
            <button
              v-if="task.status === 'processing'"
              @click="cancelTask(task.id)"
              class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors text-red-600"
              title="取消任务"
              aria-label="取消此任务"
            >
              <i class="pi pi-times"></i>
            </button>

            <!-- 打开输出目录按钮（仅完成状态） -->
            <button
              v-if="task.status === 'completed'"
              @click="openOutputDirectory(task.outputPath)"
              class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors text-primary"
              title="打开输出目录"
              aria-label="打开输出目录"
            >
              <i class="pi pi-folder-open"></i>
            </button>

            <!-- 删除按钮 -->
            <button
              @click="removeTask(task.id)"
              class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors text-gray-500 hover:text-red-600"
              title="删除任务"
              aria-label="删除此任务"
            >
              <i class="pi pi-trash"></i>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 批量操作栏 -->
    <div
      v-if="selectedTasks.length > 0"
      class="fixed bottom-6 left-1/2 transform -translate-x-1/2 glass-card p-4 shadow-xl animate-slide-up"
    >
      <div class="flex items-center justify-between">
        <div class="flex items-center space-x-4">
          <span class="font-medium text-gray-900 dark:text-white">
            已选择 {{ selectedTasks.length }} 个任务
          </span>
          <button
            @click="clearSelection"
            class="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
          >
            取消选择
          </button>
        </div>
        <div class="flex items-center space-x-2">
          <button
            @click="pauseSelectedTasks"
            class="glass-button px-4 py-2 text-yellow-600"
            :disabled="!hasProcessingTasks"
          >
            <i class="pi pi-pause mr-2"></i>
            暂停
          </button>
          <button
            @click="cancelSelectedTasks"
            class="glass-button px-4 py-2 text-red-600"
          >
            <i class="pi pi-times mr-2"></i>
            取消
          </button>
          <button
            @click="deleteSelectedTasks"
            class="glass-button-primary px-4 py-2"
          >
            <i class="pi pi-trash mr-2"></i>
            删除
          </button>
        </div>
      </div>
    </div>

    <!-- 统计信息 -->
    <div class="mt-8 grid grid-cols-1 sm:grid-cols-4 gap-4">
      <div class="glass-card text-center p-4">
        <div class="text-2xl font-bold text-primary mb-1">{{ totalTasks }}</div>
        <div class="text-sm text-gray-600 dark:text-gray-400">总任务数</div>
      </div>
      <div class="glass-card text-center p-4">
        <div class="text-2xl font-bold text-yellow-500 mb-1">{{ activeTasks.length }}</div>
        <div class="text-sm text-gray-600 dark:text-gray-400">进行中</div>
      </div>
      <div class="glass-card text-center p-4">
        <div class="text-2xl font-bold text-green-500 mb-1">{{ completedTasks.length }}</div>
        <div class="text-sm text-gray-600 dark:text-gray-400">已完成</div>
      </div>
      <div class="glass-card text-center p-4">
        <div class="text-2xl font-bold text-red-500 mb-1">{{ errorTasks.length }}</div>
        <div class="text-sm text-gray-600 dark:text-gray-400">失败</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useAppStore, useFileStore, useUIStore } from '@/stores'
import type { DecompressTask } from '@/stores'

// Store
const appStore = useAppStore()
const fileStore = useFileStore()
const uiStore = useUIStore()

// 状态
const filter = ref<'all' | 'active' | 'completed' | 'error'>('all')
const sortOrder = ref<'desc' | 'asc'>('desc')
const showFilterMenu = ref(false)
const isRefreshing = ref(false)
const selectedTasks = ref<string[]>([])

// 筛选选项
const filterOptions = [
  { value: 'all' as const, label: '全部任务' },
  { value: 'active' as const, label: '进行中' },
  { value: 'completed' as const, label: '已完成' },
  { value: 'error' as const, label: '失败' }
]

const filterLabels = {
  all: '全部任务',
  active: '进行中',
  completed: '已完成',
  error: '失败'
}

// 状态样式
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

// 计算属性
const allTasks = computed(() => appStore.decompressTasks)
const activeTasks = computed(() => appStore.activeTasks)
const completedTasks = computed(() => appStore.completedTasks)
const totalTasks = computed(() => allTasks.value.length)

const errorTasks = computed(() => {
  return allTasks.value.filter(task => task.status === 'error')
})

const filteredTasks = computed(() => {
  switch (filter.value) {
    case 'active':
      return activeTasks.value
    case 'completed':
      return completedTasks.value
    case 'error':
      return errorTasks.value
    default:
      return allTasks.value
  }
})

const sortedTasks = computed(() => {
  const tasks = [...filteredTasks.value]
  return tasks.sort((a, b) => {
    const timeA = a.startTime?.getTime() || 0
    const timeB = b.startTime?.getTime() || 0
    return sortOrder.value === 'desc' ? timeB - timeA : timeA - timeB
  })
})

const emptyStateMessage = computed(() => {
  switch (filter.value) {
    case 'active':
      return '没有进行中的任务'
    case 'completed':
      return '没有已完成的任务'
    case 'error':
      return '没有失败的任务'
    default:
      return '还没有任何解压任务'
  }
})

const hasProcessingTasks = computed(() => {
  return selectedTasks.value.some(taskId => {
    const task = allTasks.value.find(t => t.id === taskId)
    return task?.status === 'processing'
  })
})

// 方法
const toggleFilterMenu = () => {
  showFilterMenu.value = !showFilterMenu.value
}

const setFilter = (value: 'all' | 'active' | 'completed' | 'error') => {
  filter.value = value
  showFilterMenu.value = false
}

const toggleSortOrder = () => {
  sortOrder.value = sortOrder.value === 'desc' ? 'asc' : 'desc'
}

const refreshTasks = async () => {
  isRefreshing.value = true
  // 这里可以添加刷新任务的逻辑，比如从服务器获取最新状态
  await new Promise(resolve => setTimeout(resolve, 1000)) // 模拟延迟
  isRefreshing.value = false
  uiStore.showSuccess('任务列表已刷新')
}

const clearCompletedTasks = () => {
  appStore.clearCompletedTasks()
  uiStore.showSuccess('已完成的任务已清理')
}

const getFileName = (fileId: string) => {
  // 这里应该根据fileId获取文件名
  // 暂时返回一个占位符
  return `文件_${fileId.substring(0, 8)}`
}

const formatTime = (date?: Date) => {
  if (!date) return '未知时间'
  return date.toLocaleString('zh-CN', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  })
}

const getShortPath = (path: string) => {
  if (path.length <= 30) return path
  return '...' + path.slice(-27)
}

const formatOptions = (options: DecompressTask['options']) => {
  const parts: string[] = []
  if (options.keepStructure) parts.push('保持结构')
  if (options.overwrite) parts.push('覆盖')
  if (options.deleteAfter) parts.push('删除原文件')
  return parts.length > 0 ? parts.join('，') : '默认'
}

const formatDuration = (startTime?: Date, endTime?: Date) => {
  if (!startTime || !endTime) return '未知时长'
  const duration = endTime.getTime() - startTime.getTime()
  const seconds = Math.floor(duration / 1000)
  if (seconds < 60) return `${seconds}秒`
  const minutes = Math.floor(seconds / 60)
  return `${minutes}分钟`
}

const retryTask = (taskId: string) => {
  // 这里应该实现重试任务的逻辑
  uiStore.showInfo('重试功能开发中')
}

const cancelTask = (taskId: string) => {
  // 这里应该实现取消任务的逻辑
  appStore.markTaskAsError(taskId, '任务已取消')
  uiStore.showWarning('任务已取消')
}

const openOutputDirectory = (path: string) => {
  // 这里应该实现打开目录的逻辑
  uiStore.showInfo(`打开目录: ${path}`)
}

const removeTask = (taskId: string) => {
  // 这里应该实现删除任务的逻辑
  const index = appStore.decompressTasks.findIndex(t => t.id === taskId)
  if (index !== -1) {
    appStore.decompressTasks.splice(index, 1)
    uiStore.showSuccess('任务已删除')
  }
}

const clearSelection = () => {
  selectedTasks.value = []
}

const pauseSelectedTasks = () => {
  // 这里应该实现暂停任务的逻辑
  uiStore.showInfo('暂停功能开发中')
}

const cancelSelectedTasks = () => {
  selectedTasks.value.forEach(taskId => {
    cancelTask(taskId)
  })
  selectedTasks.value = []
}

const deleteSelectedTasks = () => {
  selectedTasks.value.forEach(taskId => {
    removeTask(taskId)
  })
  selectedTasks.value = []
}

// 点击外部关闭筛选菜单
const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as HTMLElement
  if (!target.closest('.filter-menu')) {
    showFilterMenu.value = false
  }
}

// 生命周期
onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

// 清理
import { onUnmounted } from 'vue'
onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style scoped>
.task-list {
  max-width: 100%;
}

/* 选中状态 */
.task-item-selected {
  @apply bg-primary/5 border-primary/30;
}

/* 动画 */
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.processing-pulse {
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

/* 滚动条样式 */
.task-list::-webkit-scrollbar {
  width: 6px;
}

.task-list::-webkit-scrollbar-track {
  @apply bg-transparent;
}

.task-list::-webkit-scrollbar-thumb {
  @apply bg-gray-300 dark:bg-gray-700 rounded-full;
}

.task-list::-webkit-scrollbar-thumb:hover {
  @apply bg-gray-400 dark:bg-gray-600;
}
</style>