<template>
  <div class="task-list-example space-y-6">
    <!-- 标题 -->
    <div class="text-center">
      <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-2">任务列表组件示例</h1>
      <p class="text-lg text-gray-600 dark:text-gray-400">显示压缩解压任务列表和状�?/p>
    </div>

    <!-- 任务列表 -->
    <div class="space-y-4">
      <div class="flex items-center justify-between">
        <h2 class="text-2xl font-semibold text-gray-900 dark:text-white">任务列表</h2>
        <div class="flex items-center space-x-2">
          <span class="text-sm text-gray-600 dark:text-gray-400">
            �?{{ tasks.length }} 个任务，{{ activeTasksCount }} 个进行中
          </span>
          <button
            @click="refreshTasks"
            class="glass-button-secondary px-3 py-1.5 text-sm"
            :disabled="isRefreshing"
          >
            <i class="pi pi-refresh mr-1" :class="{ 'animate-spin': isRefreshing }"></i>
            刷新
          </button>
        </div>
      </div>

      <!-- 任务列表容器 -->
      <div class="space-y-3">
        <div
          v-for="task in tasks"
          :key="task.id"
          class="glass-card p-4 hover:scale-[1.02] transition-transform duration-200"
        >
          <div class="flex items-start justify-between">
            <!-- 任务信息 -->
            <div class="flex-1 min-w-0">
              <div class="flex items-center mb-2">
                <div class="w-10 h-10 rounded-lg flex items-center justify-center mr-3"
                     :class="getTaskVariantClass(task)">
                  <i :class="getTaskIcon(task)" class="text-lg"></i>
                </div>
                <div class="min-w-0">
                  <div class="flex items-center">
                    <h3 class="font-semibold text-gray-900 dark:text-white truncate">
                      {{ task.name }}
                    </h3>
                    <span class="ml-2 text-xs px-2 py-0.5 rounded-full"
                          :class="getStatusClass(task)">
                      {{ getStatusText(task) }}
                    </span>
                  </div>
                  <p class="text-sm text-gray-600 dark:text-gray-400 truncate">
                    {{ task.description }}
                  </p>
                </div>
              </div>

              <!-- 进度�?-->
              <div class="mt-3">
                <div class="flex justify-between text-sm mb-1">
                  <span class="text-gray-700 dark:text-gray-300">进度</span>
                  <span class="font-medium text-gray-900 dark:text-white">
                    {{ task.progress }}%
                  </span>
                </div>
                <ProgressBar
                  :value="task.progress"
                  :variant="getProgressVariant(task)"
                  :size="task.status === 'processing' ? 'lg' : 'md'"
                  :striped="task.status === 'processing'"
                  :animated="task.status === 'processing'"
                  :indeterminate="task.status === 'queued'"
                />
              </div>

              <!-- 任务详情 -->
              <div class="mt-3 grid grid-cols-2 gap-3 text-sm">
                <div class="flex items-center">
                  <i class="pi pi-file text-gray-400 mr-2"></i>
                  <span class="text-gray-600 dark:text-gray-400 truncate">{{ task.fileName }}</span>
                </div>
                <div class="flex items-center">
                  <i class="pi pi-calendar text-gray-400 mr-2"></i>
                  <span class="text-gray-600 dark:text-gray-400">{{ formatTime(task.createdAt) }}</span>
                </div>
                <div class="flex items-center">
                  <i class="pi pi-database text-gray-400 mr-2"></i>
                  <span class="text-gray-600 dark:text-gray-400">{{ formatFileSize(task.fileSize) }}</span>
                </div>
                <div class="flex items-center">
                  <i class="pi pi-clock text-gray-400 mr-2"></i>
                  <span class="text-gray-600 dark:text-gray-400">{{ task.estimatedTime }}</span>
                </div>
              </div>
            </div>

            <!-- 操作按钮 -->
            <div class="ml-4 flex flex-col space-y-2">
              <button
                v-if="task.status === 'processing'"
                @click="pauseTask(task.id)"
                class="glass-button-warning px-3 py-1.5 text-sm"
                :title="`暂停 ${task.name}`"
              >
                <i class="pi pi-pause mr-1"></i>
                暂停
              </button>
              <button
                v-else-if="task.status === 'paused'"
                @click="resumeTask(task.id)"
                class="glass-button-success px-3 py-1.5 text-sm"
                :title="`继续 ${task.name}`"
              >
                <i class="pi pi-play mr-1"></i>
                继续
              </button>
              <button
                v-if="task.status === 'queued' || task.status === 'paused'"
                @click="cancelTask(task.id)"
                class="glass-button-danger px-3 py-1.5 text-sm"
                :title="`取消 ${task.name}`"
              >
                <i class="pi pi-times mr-1"></i>
                取消
              </button>
              <button
                v-if="task.status === 'completed' || task.status === 'failed'"
                @click="removeTask(task.id)"
                class="glass-button px-3 py-1.5 text-sm"
                :title="`移除 ${task.name}`"
              >
                <i class="pi pi-trash mr-1"></i>
                移除
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 空状�?-->
      <div v-if="tasks.length === 0" class="text-center py-12 rounded-lg border-2 border-dashed border-gray-300 dark:border-gray-600">
        <i class="pi pi-inbox text-gray-400 text-4xl mb-4"></i>
        <p class="text-gray-500 dark:text-gray-400 mb-2">暂无任务</p>
        <p class="text-sm text-gray-400 dark:text-gray-500">开始解压文件后，任务将显示在这�?/p>
      </div>
    </div>

    <!-- 统计面板 -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <div class="glass-card p-4 text-center">
        <div class="w-12 h-12 rounded-full bg-primary/10 flex items-center justify-center mx-auto mb-3">
          <i class="pi pi-play text-primary text-xl"></i>
        </div>
        <p class="text-2xl font-bold text-gray-900 dark:text-white">{{ activeTasksCount }}</p>
        <p class="text-sm text-gray-600 dark:text-gray-400">进行�?/p>
      </div>

      <div class="glass-card p-4 text-center">
        <div class="w-12 h-12 rounded-full bg-success/10 flex items-center justify-center mx-auto mb-3">
          <i class="pi pi-check text-success text-xl"></i>
        </div>
        <p class="text-2xl font-bold text-gray-900 dark:text-white">{{ completedTasksCount }}</p>
        <p class="text-sm text-gray-600 dark:text-gray-400">已完�?/p>
      </div>

      <div class="glass-card p-4 text-center">
        <div class="w-12 h-12 rounded-full bg-warning/10 flex items-center justify-center mx-auto mb-3">
          <i class="pi pi-clock text-warning text-xl"></i>
        </div>
        <p class="text-2xl font-bold text-gray-900 dark:text-white">{{ queuedTasksCount }}</p>
        <p class="text-sm text-gray-600 dark:text-gray-400">等待�?/p>
      </div>

      <div class="glass-card p-4 text-center">
        <div class="w-12 h-12 rounded-full bg-danger/10 flex items-center justify-center mx-auto mb-3">
          <i class="pi pi-times text-danger text-xl"></i>
        </div>
        <p class="text-2xl font-bold text-gray-900 dark:text-white">{{ failedTasksCount }}</p>
        <p class="text-sm text-gray-600 dark:text-gray-400">失败</p>
      </div>
    </div>

    <!-- 批量操作 -->
    <div class="glass-card p-6">
      <h3 class="font-medium text-gray-900 dark:text-white mb-4">批量操作</h3>
      <div class="flex flex-wrap gap-3">
        <button
          @click="pauseAllTasks"
          :disabled="activeTasksCount === 0"
          class="glass-button-warning px-4 py-2"
          :class="{ 'opacity-50 cursor-not-allowed': activeTasksCount === 0 }"
        >
          <i class="pi pi-pause mr-2"></i>
          暂停所�?({{ activeTasksCount }})
        </button>
        <button
          @click="resumeAllTasks"
          :disabled="pausedTasksCount === 0"
          class="glass-button-success px-4 py-2"
          :class="{ 'opacity-50 cursor-not-allowed': pausedTasksCount === 0 }"
        >
          <i class="pi pi-play mr-2"></i>
          继续所�?({{ pausedTasksCount }})
        </button>
        <button
          @click="cancelAllTasks"
          :disabled="cancelableTasksCount === 0"
          class="glass-button-danger px-4 py-2"
          :class="{ 'opacity-50 cursor-not-allowed': cancelableTasksCount === 0 }"
        >
          <i class="pi pi-times mr-2"></i>
          取消所�?({{ cancelableTasksCount }})
        </button>
        <button
          @click="clearCompletedTasks"
          :disabled="completedTasksCount === 0"
          class="glass-button px-4 py-2"
          :class="{ 'opacity-50 cursor-not-allowed': completedTasksCount === 0 }"
        >
          <i class="pi pi-trash mr-2"></i>
          清理已完�?({{ completedTasksCount }})
        </button>
      </div>
    </div>

    <!-- 使用说明 -->
    <div class="glass-card p-6">
      <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-3">使用说明</h3>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div>
          <h4 class="font-medium text-gray-800 dark:text-gray-200 mb-2">任务状�?/h4>
          <ul class="space-y-2 text-gray-600 dark:text-gray-400">
            <li class="flex items-center">
              <span class="w-3 h-3 rounded-full bg-primary mr-2"></span>
              <span>进行中：任务正在执行</span>
            </li>
            <li class="flex items-center">
              <span class="w-3 h-3 rounded-full bg-success mr-2"></span>
              <span>已完成：任务成功完成</span>
            </li>
            <li class="flex items-center">
              <span class="w-3 h-3 rounded-full bg-warning mr-2"></span>
              <span>等待中：任务在队列中等待</span>
            </li>
            <li class="flex items-center">
              <span class="w-3 h-3 rounded-full bg-danger mr-2"></span>
              <span>失败：任务执行失�?/span>
            </li>
            <li class="flex items-center">
              <span class="w-3 h-3 rounded-full bg-gray-400 mr-2"></span>
              <span>已暂停：任务被暂�?/span>
            </li>
          </ul>
        </div>
        <div>
          <h4 class="font-medium text-gray-800 dark:text-gray-200 mb-2">操作说明</h4>
          <ul class="space-y-2 text-gray-600 dark:text-gray-400">
            <li class="flex items-start">
              <i class="pi pi-play text-success mr-2 mt-0.5"></i>
              <span>继续：恢复已暂停的任�?/span>
            </li>
            <li class="flex items-start">
              <i class="pi pi-pause text-warning mr-2 mt-0.5"></i>
              <span>暂停：暂时停止任务执�?/span>
            </li>
            <li class="flex items-start">
              <i class="pi pi-times text-danger mr-2 mt-0.5"></i>
              <span>取消：取消未完成的任�?/span>
            </li>
            <li class="flex items-start">
              <i class="pi pi-trash text-gray-500 mr-2 mt-0.5"></i>
              <span>移除：删除已完成或失败的任务</span>
            </li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import ProgressBar from './ProgressBar.vue'

interface Task {
  id: string
  name: string
  description: string
  fileName: string
  fileSize: number
  status: 'queued' | 'processing' | 'paused' | 'completed' | 'failed'
  progress: number
  createdAt: Date
  estimatedTime: string
  type: 'compress' | 'decompress'
}

// 状�?
const tasks = ref<Task[]>([
  {
    id: '1',
    name: '解压文档文件',
    description: '解压包含项目文档的压缩包',
    fileName: 'project-docs.zip',
    fileSize: 1024 * 1024 * 45, // 45MB
    status: 'processing',
    progress: 75,
    createdAt: new Date(Date.now() - 30 * 60 * 1000), // 30分钟�?
    estimatedTime: '剩余 5分钟',
    type: 'decompress'
  },
  {
    id: '2',
    name: '压缩图片�?,
    description: '压缩图片文件夹以节省空间',
    fileName: 'photos/',
    fileSize: 1024 * 1024 * 120, // 120MB
    status: 'queued',
    progress: 0,
    createdAt: new Date(Date.now() - 15 * 60 * 1000), // 15分钟�?
    estimatedTime: '预计 8分钟',
    type: 'compress'
  },
  {
    id: '3',
    name: '解压备份文件',
    description: '解压系统备份文件',
    fileName: 'backup-2024.rar',
    fileSize: 1024 * 1024 * 210, // 210MB
    status: 'completed',
    progress: 100,
    createdAt: new Date(Date.now() - 2 * 60 * 60 * 1000), // 2小时�?
    estimatedTime: '已完�?,
    type: 'decompress'
  },
  {
    id: '4',
    name: '压缩日志文件',
    description: '压缩应用程序日志文件',
    fileName: 'app-logs.tar.gz',
    fileSize: 1024 * 1024 * 85, // 85MB
    status: 'failed',
    progress: 45,
    createdAt: new Date(Date.now() - 45 * 60 * 1000), // 45分钟�?
    estimatedTime: '失败',
    type: 'compress'
  },
  {
    id: '5',
    name: '解压媒体文件',
    description: '解压视频和音频文�?,
    fileName: 'media-files.7z',
    fileSize: 1024 * 1024 * 350, // 350MB
    status: 'paused',
    progress: 30,
    createdAt: new Date(Date.now() - 20 * 60 * 1000), // 20分钟�?
    estimatedTime: '已暂�?,
    type: 'decompress'
  }
])

const isRefreshing = ref(false)

// 计算属�?
const activeTasksCount = computed(() => {
  return tasks.value.filter(task => task.status === 'processing').length
})

const completedTasksCount = computed(() => {
  return tasks.value.filter(task => task.status === 'completed').length
})

const queuedTasksCount = computed(() => {
  return tasks.value.filter(task => task.status === 'queued').length
})

const failedTasksCount = computed(() => {
  return tasks.value.filter(task => task.status === 'failed').length
})

const pausedTasksCount = computed(() => {
  return tasks.value.filter(task => task.status === 'paused').length
})

const cancelableTasksCount = computed(() => {
  return tasks.value.filter(task =>
    task.status === 'queued' ||
    task.status === 'paused' ||
    task.status === 'processing'
  ).length
})

// 方法
const getTaskVariantClass = (task: Task): string => {
  switch (task.type) {
    case 'compress':
      return 'bg-accent/10 text-accent'
    case 'decompress':
      return 'bg-primary/10 text-primary'
    default:
      return 'bg-gray-100 dark:bg-gray-800 text-gray-500'
  }
}

const getTaskIcon = (task: Task): string => {
  switch (task.type) {
    case 'compress':
      return 'pi pi-compress'
    case 'decompress':
      return 'pi pi-expand'
    default:
      return 'pi pi-file'
  }
}

const getStatusClass = (task: Task): string => {
  switch (task.status) {
    case 'processing':
      return 'bg-primary/10 text-primary'
    case 'completed':
      return 'bg-success/10 text-success'
    case 'queued':
      return 'bg-warning/10 text-warning'
    case 'failed':
      return 'bg-danger/10 text-danger'
    case 'paused':
      return 'bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400'
    default:
      return 'bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400'
  }
}

const getStatusText = (task: Task): string => {
  switch (task.status) {
    case 'processing':
      return '进行�?
    case 'completed':
      return '已完�?
    case 'queued':
      return '等待�?
    case 'failed':
      return '失败'
    case 'paused':
      return '已暂�?
    default:
      return '未知'
  }
}

const getProgressVariant = (task: Task) => {
  switch (task.status) {
    case 'processing':
      return 'primary'
    case 'completed':
      return 'success'
    case 'failed':
      return 'danger'
    case 'paused':
      return 'secondary'
    default:
      return 'info'
  }
}

const formatTime = (date: Date): string => {
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / (1000 * 60))

  if (diffMins < 1) return '刚刚'
  if (diffMins < 60) return `${diffMins}分钟前`

  const diffHours = Math.floor(diffMins / 60)
  if (diffHours < 24) return `${diffHours}小时前`

  const diffDays = Math.floor(diffHours / 24)
  return `${diffDays}天前`
}

const formatFileSize = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const refreshTasks = () => {
  isRefreshing.value = true
  // 模拟API调用
  setTimeout(() => {
    // 更新进度
    tasks.value = tasks.value.map(task => {
      if (task.status === 'processing' && task.progress < 100) {
        return {
          ...task,
          progress: Math.min(100, task.progress + Math.random() * 10)
        }
      }
      return task
    })
    isRefreshing.value = false
  }, 1000)
}

const pauseTask = (taskId: string) => {
  const task = tasks.value.find(t => t.id === taskId)
  if (task && task.status === 'processing') {
    task.status = 'paused'
    task.estimatedTime = '已暂�?
  }
}

const resumeTask = (taskId: string) => {
  const task = tasks.value.find(t => t.id === taskId)
  if (task && task.status === 'paused') {
    task.status = 'processing'
    task.estimatedTime = '剩余 5分钟'
  }
}

const cancelTask = (taskId: string) => {
  const task = tasks.value.find(t => t.id === taskId)
  if (task && (task.status === 'queued' || task.status === 'paused' || task.status === 'processing')) {
    task.status = 'failed'
    task.progress = 0
    task.estimatedTime = '已取�?
  }
}

const removeTask = (taskId: string) => {
  tasks.value = tasks.value.filter(t => t.id !== taskId)
}

const pauseAllTasks = () => {
  tasks.value = tasks.value.map(task => {
    if (task.status === 'processing') {
      return { ...task, status: 'paused', estimatedTime: '已暂�? }
    }
    return task
  })
}

const resumeAllTasks = () => {
  tasks.value = tasks.value.map(task => {
    if (task.status === 'paused') {
      return { ...task, status: 'processing', estimatedTime: '剩余 5分钟' }
    }
    return task
  })
}

const cancelAllTasks = () => {
  tasks.value = tasks.value.map(task => {
    if (task.status === 'queued' || task.status === 'paused' || task.status === 'processing') {
      return { ...task, status: 'failed', progress: 0, estimatedTime: '已取�? }
    }
    return task
  })
}

const clearCompletedTasks = () => {
  tasks.value = tasks.value.filter(task => task.status !== 'completed')
}

// 模拟任务进度更新
setInterval(() => {
  tasks.value = tasks.value.map(task => {
    if (task.status === 'processing' && task.progress < 100) {
      const newProgress = Math.min(100, task.progress + Math.random() * 2)
      if (newProgress >= 100) {
        return {
          ...task,
          progress: 100,
          status: 'completed',
          estimatedTime: '已完�?
        }
      }
      return {
        ...task,
        progress: newProgress
      }
    }
    return task
  })
}, 3000)
</script>

<style scoped>
.task-list-example {
  max-width: 1200px;
  margin: 0 auto;
  padding: 2rem;
}
</style>
