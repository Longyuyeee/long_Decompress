<template>
  <div class="task-list">
    <!-- 标题和操作栏 -->
    <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 mb-6">
      <div>
        <h2 class="text-xl font-semibold text-gray-900 dark:text-white">执行队列</h2>
        <p class="text-gray-500 text-sm mt-1">
          共 {{ tasks.length }} 个任务，{{ activeCount }} 个正在执行
        </p>
      </div>

      <div class="flex items-center gap-2">
        <button 
          @click="taskStore.fetchTasks()" 
          class="glass-button p-2"
          title="手动刷新"
        >
          <i class="pi pi-refresh"></i>
        </button>
        <button 
          @click="taskStore.clearFinishedTasks()" 
          class="glass-button px-4 py-2 text-sm text-red-500 hover:bg-red-50"
          :disabled="!hasFinished"
        >
          <i class="pi pi-trash mr-2"></i>清理已结束
        </button>
      </div>
    </div>

    <!-- 过滤器 -->
    <div class="flex gap-2 mb-6 overflow-x-auto pb-2 scrollbar-hide">
      <button 
        v-for="f in filterOptions" 
        :key="f.value"
        @click="currentFilter = f.value"
        :class="isFilterActive(f.value) ? 'bg-primary text-white' : 'glass-button'"
        class="px-4 py-1.5 rounded-full text-xs font-medium whitespace-nowrap transition-all"
      >
        {{ f.label }}
      </button>
    </div>

    <!-- 任务列表主体 -->
    <div class="space-y-4">
      <div v-if="filteredTasks.length === 0" class="glass-card py-20 text-center opacity-40">
        <i class="pi pi-inbox text-5xl mb-4"></i>
        <p>暂无符合条件的任务</p>
      </div>

      <div 
        v-for="task in filteredTasks" 
        :key="task.id"
        class="glass-card p-4 hover:shadow-lg transition-all border-l-4"
        :class="getStatusBorder(task.status)"
      >
        <div class="flex items-start gap-4">
          <div class="w-10 h-10 rounded-xl flex items-center justify-center text-lg" :class="getStatusBg(task.status)">
            <i :class="getTaskIcon(task)"></i>
          </div>
          
          <div class="flex-1 min-w-0">
            <div class="flex justify-between items-start mb-1">
              <h3 class="font-bold text-gray-900 dark:text-white truncate max-w-md">
                <span v-if="configStore.privacyMode" class="font-mono tracking-widest text-gray-400">•••••••.{{ task.name.split('.').pop() }}</span>
                <span v-else>{{ task.name }}</span>
              </h3>
              <span class="text-[10px] font-mono opacity-40">{{ task.id.substring(0, 8) }}</span>
            </div>
            
            <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-gray-500 mb-3">
              <span class="flex items-center"><i class="pi pi-clock mr-1.5 text-[10px]"></i>{{ formatDate(task.startTime?.toISOString()) }}</span>
              <span class="flex items-center"><i class="pi pi-tag mr-1.5 text-[10px]"></i>{{ task.type === 'decompression' ? '解压' : '压缩' }}</span>
            </div>

            <!-- 进度条 -->
            <div v-if="isRunningStatus(task.status)" class="space-y-1.5">
              <div class="flex justify-between text-[10px]">
                <span class="text-primary font-bold">正在执行</span>
                <span class="font-mono">{{ task.progress }}%</span>
              </div>
              <div class="w-full bg-gray-100 dark:bg-gray-800 rounded-full h-1.5 overflow-hidden">
                <div class="bg-primary h-full transition-all duration-500" :style="{ width: task.progress + '%' }"></div>
              </div>
            </div>

            <!-- 错误提示 -->
            <div v-if="task.status === 'failed'" class="mt-2 p-2 bg-red-50 dark:bg-red-900/10 rounded-lg border border-red-100 dark:border-red-900/20">
              <p class="text-[10px] text-red-500 flex items-center">
                <i class="pi pi-exclamation-circle mr-1.5"></i> {{ task.error || '原因未知' }}
              </p>
            </div>
          </div>

          <!-- 操作 -->
          <div class="flex gap-2">
            <button 
              v-if="isRunningStatus(task.status)" 
              @click="taskStore.cancelTask(task.id)" 
              class="p-2 hover:bg-red-50 text-red-500 rounded-lg transition-all"
            >
              <i class="pi pi-stop-circle"></i>
            </button>
            <button 
              v-if="task.status === 'completed'" 
              @click="openInExplorer(task.outputPath)" 
              class="p-2 hover:bg-primary/10 text-primary rounded-lg transition-all"
            >
              <i class="pi pi-folder-open"></i>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useTaskStore, type TaskStatus } from '@/stores/task'
import { useConfigStore } from '@/stores/config'
import { storeToRefs } from 'pinia'

const taskStore = useTaskStore()
const configStore = useConfigStore()
const { tasks } = storeToRefs(taskStore)

const currentFilter = ref('all')
const filterOptions = [
  { label: '全部', value: 'all' },
  { label: '进行中', value: 'running' },
  { label: '已完成', value: 'completed' },
  { label: '失败', value: 'failed' },
]

onMounted(() => {
  taskStore.fetchTasks()
})

const isRunningStatus = (status: TaskStatus) => {
  return ['running', 'extracting', 'compressing', 'preparing', 'finalizing'].includes(status)
}

const activeCount = computed(() => tasks.value.filter(t => isRunningStatus(t.status)).length)
const hasFinished = computed(() => tasks.value.some(t => ['completed', 'failed', 'cancelled'].includes(t.status)))

const isFilterActive = (filter: string) => {
  if (filter === 'all') return currentFilter.value === 'all'
  if (filter === 'running') return currentFilter.value === 'running'
  return currentFilter.value === filter
}

const filteredTasks = computed(() => {
  if (currentFilter.value === 'all') return tasks.value
  if (currentFilter.value === 'running') return tasks.value.filter(t => isRunningStatus(t.status))
  return tasks.value.filter(t => t.status === currentFilter.value)
})

const getStatusBorder = (status: TaskStatus) => {
  if (isRunningStatus(status)) return 'border-primary'
  switch (status) {
    case 'completed': return 'border-green-500'
    case 'failed': return 'border-red-500'
    case 'cancelled': return 'border-gray-400'
    default: return 'border-transparent'
  }
}

const getStatusBg = (status: TaskStatus) => {
  if (isRunningStatus(status)) return 'bg-primary/10 text-primary'
  switch (status) {
    case 'completed': return 'bg-green-500/10 text-green-500'
    case 'failed': return 'bg-red-500/10 text-red-500'
    default: return 'bg-gray-100 dark:bg-gray-800 text-gray-400'
  }
}

const getTaskIcon = (task: any) => {
  if (isRunningStatus(task.status)) return 'pi pi-spin pi-spinner'
  return task.type === 'decompression' ? 'pi pi-file-export' : 'pi pi-file-import'
}

const formatDate = (dateStr?: string) => {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

const openInExplorer = async (path: string) => {
  // TODO: 调用 Tauri 开启资源管理器命令
  console.log('Opening in explorer:', path)
}
</script>

<style scoped>
.scrollbar-hide::-webkit-scrollbar {
  display: none;
}
.scrollbar-hide {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
</style>
