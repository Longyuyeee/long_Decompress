<script setup lang="ts">
import { computed, ref } from 'vue'
import { useTaskStore, type Task } from '@/stores/task'
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { extractErrorMessage } from '@/utils'

const taskStore = useTaskStore()
const appStore = useAppStore()
const tauriCommands = useTauriCommands()
const isExpanded = ref(false)
const retryingTaskId = ref<string | null>(null)

const activeTasks = computed(() =>
  taskStore.tasks.filter(t => !['completed', 'failed', 'cancelled'].includes(t.status))
)

const hasActiveTasks = computed(() => activeTasks.value.length > 0)

const completedCount = computed(() =>
  taskStore.tasks.filter(t => t.status === 'completed').length
)

const failedCount = computed(() =>
  taskStore.tasks.filter(t => t.status === 'failed').length
)

const totalCount = computed(() => taskStore.tasks.length)

const overallProgress = computed(() => {
  if (totalCount.value === 0) return 0
  const total = taskStore.tasks.reduce((sum, t) => sum + t.progress, 0)
  return Math.round(total / totalCount.value)
})

// 当前正在运行的任务（取第一个活跃中任务）
const runningTask = computed(() =>
  taskStore.tasks.find(t =>
    ['preparing', 'running', 'extracting', 'compressing', 'finalizing'].includes(t.status)
  )
)

const currentTaskName = computed(() => runningTask.value?.name || '')

const isVisible = computed(() => taskStore.tasks.length > 0)

const sortedTasks = computed(() =>
  [...taskStore.tasks].sort((a, b) => {
    const order = (s: string) => {
      if (['preparing', 'running', 'extracting', 'compressing', 'finalizing'].includes(s)) return 0
      if (s === 'pending') return 1
      if (s === 'failed') return 2
      if (s === 'completed') return 3
      if (s === 'cancelled') return 4
      return 5
    }
    return order(a.status) - order(b.status)
  })
)

const statusIcon = (status: string) => {
  if (status === 'completed') return 'pi pi-check-circle text-green-400'
  if (status === 'failed') return 'pi pi-exclamation-circle text-red-400'
  if (status === 'cancelled') return 'pi pi-ban text-muted'
  if (['preparing', 'running', 'extracting', 'compressing', 'finalizing'].includes(status)) return 'pi pi-spin pi-spinner text-primary'
  return 'pi pi-clock text-muted'
}

const statusLabel = (status: string) => {
  switch (status) {
    case 'pending': return appStore.t('tasks.status.pending')
    case 'preparing': return appStore.t('tasks.status.preparing')
    case 'running': case 'extracting': case 'compressing': return appStore.t('tasks.status.running')
    case 'finalizing': return appStore.t('tasks.status.finalizing')
    case 'completed': return appStore.t('tasks.status.completed')
    case 'failed': return appStore.t('tasks.status.failed')
    case 'cancelled': return appStore.t('tasks.status.cancelled')
    default: return status
  }
}

// 阶段翻译映射
const stageLabel = (stage?: string) => {
  if (!stage) return ''
  switch (stage) {
    case 'Pre-checking': return appStore.t('tasks.status.preparing')
    case 'Extracting': return appStore.t('tasks.status.running')
    case 'Finalizing': return appStore.t('tasks.status.finalizing')
    default: return stage
  }
}

const openTaskFolder = (task: Task) => {
  if (task.outputPath) {
    tauriCommands.openInExplorer(task.outputPath)
  }
}

const retryTask = async (task: Task) => {
  if (retryingTaskId.value) return
  retryingTaskId.value = task.id

  try {
    if (task.type === 'decompression') {
      taskStore.updateTaskStatus(task.id, 'pending')
      task.passwordRequired = false
      const options = {
        outputPath: task.outputPath,
        keepStructure: true,
        overwrite: false,
        deleteAfter: appStore.settings.autoDeleteSource,
        createSubdirectory: task.extractToSubfolder ?? false,
        password: task.password || undefined,
        fileFilter: task.fileFilter || null
      }
      taskStore.updateTaskStatus(task.id, 'preparing')
      await tauriCommands.decompressFile(task.sourceFiles[0], options, task.id)
    } else if (task.type === 'compression') {
      taskStore.updateTaskStatus(task.id, 'pending')
      taskStore.updateTaskStatus(task.id, 'compressing')
      await tauriCommands.compressFiles(
        task.id,
        task.sourceFiles,
        task.outputPath,
        {
          format: task.format || 'zip',
          level: 6,
          password: task.password || undefined
        }
      )
    }
  } catch (error) {
    taskStore.updateTaskStatus(task.id, 'failed')
    appStore.setError(`${appStore.t('common.error')}: ${extractErrorMessage(error)}`)
  } finally {
    retryingTaskId.value = null
  }
}

const cancelTask = async (task: Task) => {
  await taskStore.cancelTask(task.id)
}

const copyToClipboard = async (text: string) => {
  try { await navigator.clipboard.writeText(text) } catch { /* ignore */ }
}
</script>

<template>
  <transition name="progress-slide">
    <div v-if="isVisible"
         class="global-progress-bar fixed bottom-4 left-4 z-[300] select-none">

      <!-- 紧凑指示器：点击展开 -->
      <div
           @click="isExpanded = !isExpanded"
           class="flex items-center gap-4 px-5 py-3 rounded-2xl bg-card/90 backdrop-blur-2xl border-2 border-primary/30 shadow-2xl cursor-pointer hover:border-primary/60 hover:shadow-primary/20 transition-all min-w-[200px]">
        <!-- 环形进度 -->
        <div class="relative w-10 h-10 shrink-0">
          <svg class="w-10 h-10 -rotate-90" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" fill="none" class="text-input opacity-30"/>
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" fill="none" stroke-dasharray="62.83"
                    :stroke-dashoffset="62.83 - (62.83 * overallProgress) / 100"
                    class="text-primary transition-all duration-1000"/>
          </svg>
          <span class="absolute inset-0 flex items-center justify-center text-[0.625rem] font-black font-mono text-content">
            {{ overallProgress }}%
          </span>
        </div>

        <!-- 摘要信息 -->
        <div class="flex flex-col min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <i v-if="hasActiveTasks" class="pi pi-spin pi-spinner text-[0.75rem] text-primary"></i>
            <i v-else class="pi pi-check-circle text-[0.75rem] text-green-400"></i>
            <span class="text-[0.75rem] font-bold text-content whitespace-nowrap">
              {{ hasActiveTasks ? `${activeTasks.length} ${appStore.t('tasks.active')}` : appStore.t('tasks.all_done') }}
            </span>
          </div>
          <!-- 当前任务名 + 速度 -->
          <div class="flex items-center gap-2 mt-1">
            <span v-if="currentTaskName" class="text-[0.625rem] text-muted truncate max-w-[120px]">{{ currentTaskName }}</span>
            <span v-if="runningTask?.speed" class="text-[0.5625rem] font-mono text-primary/70 whitespace-nowrap">{{ runningTask!.speed }}</span>
          </div>
          <!-- 阶段 + 密码状态 -->
          <div v-if="runningTask" class="flex items-center gap-2 mt-1">
            <span v-if="runningTask.stage" class="text-[0.5625rem] text-dim uppercase tracking-tight">{{ stageLabel(runningTask.stage) }}</span>
            <span v-if="runningTask.currentFile" class="text-[0.5625rem] text-dim truncate max-w-[130px]" :title="runningTask.currentFile">
              · {{ runningTask.currentFile.split(/[\\/]/).pop() }}
            </span>
            <i v-if="runningTask.password" class="pi pi-lock text-[0.5625rem] text-amber-400" :title="appStore.t('progress.password_used')"></i>
            <i v-if="runningTask.passwordRequired" class="pi pi-exclamation-triangle text-[0.5625rem] text-rose-400" :title="appStore.t('progress.password_needed')"></i>
          </div>
        </div>

        <!-- 完成计数 -->
        <div class="flex flex-col items-end shrink-0">
          <span class="text-[0.75rem] font-mono text-primary font-black">{{ completedCount }}/{{ totalCount }}</span>
          <span v-if="failedCount > 0" class="text-[0.5625rem] font-mono text-red-400 font-bold">{{ failedCount }} {{ appStore.t('tasks.status.failed') }}</span>
        </div>

        <i :class="isExpanded ? 'pi pi-chevron-down' : 'pi pi-chevron-up'" class="text-[0.625rem] text-dim shrink-0"></i>
      </div>

      <!-- 展开的任务列表面板 -->
      <transition name="panel-slide">
        <div v-if="isExpanded"
             class="absolute bottom-full left-0 mb-2 w-[30rem] max-h-[65vh] rounded-2xl bg-card/95 backdrop-blur-2xl border-2 border-primary/30 shadow-2xl overflow-hidden flex flex-col">
          <!-- 面板头部 -->
          <div class="px-5 py-3.5 border-b border-subtle/20 shrink-0">
            <div class="flex items-center justify-between mb-2">
              <span class="text-[0.6875rem] font-black text-content uppercase tracking-widest">{{ appStore.t('tasks.monitor') }}</span>
              <div class="flex items-center gap-3">
                <span class="text-[0.625rem] font-mono text-primary font-black">{{ completedCount }}/{{ totalCount }}</span>
                <button
                  v-if="taskStore.tasks.some(t => ['completed', 'failed', 'cancelled'].includes(t.status))"
                  @click.stop="taskStore.clearFinishedTasks()"
                  class="text-[0.625rem] text-dim hover:text-red-400 transition-colors font-bold uppercase tracking-wider">
                  {{ appStore.t('tasks.clear_done') }}
                </button>
              </div>
            </div>
            <!-- 总进度条 -->
            <div class="h-2 bg-input rounded-full overflow-hidden">
              <div class="h-full bg-primary rounded-full transition-all duration-1000"
                   :style="{ width: `${overallProgress}%` }"></div>
            </div>
            <!-- 实时信息摘要栏 -->
            <div v-if="runningTask" class="flex items-center gap-4 mt-2 text-[0.5625rem] text-dim">
              <span v-if="runningTask.stage">{{ appStore.t('progress.stage') }}: {{ stageLabel(runningTask.stage) }}</span>
              <span v-if="runningTask.speed">{{ appStore.t('progress.speed') }}: {{ runningTask.speed }}</span>
              <span v-if="runningTask.currentFile" class="truncate flex-1" :title="runningTask.currentFile">
                {{ appStore.t('progress.current_file') }}: {{ runningTask.currentFile.split(/[\\/]/).pop() }}
              </span>
            </div>
          </div>

          <!-- 任务列表 -->
          <div class="flex-1 overflow-y-auto custom-scrollbar p-2 space-y-1.5">
            <div v-for="task in sortedTasks" :key="task.id"
                 class="rounded-xl border transition-all"
                 :class="[
                   task.status === 'failed' ? 'bg-red-500/5 border-red-500/20' :
                   task.status === 'completed' ? 'bg-green-500/5 border-green-500/20' :
                   task.status === 'cancelled' ? 'bg-muted/5 border-subtle/20' :
                   'bg-primary/5 border-primary/20'
                 ]">
              <!-- 任务行 -->
              <div class="flex items-center gap-2.5 px-3.5 py-3">
                <i :class="[statusIcon(task.status), 'text-[0.75rem] shrink-0']"></i>

                <div class="flex-1 min-w-0">
                  <!-- 名称 + 类型标签 -->
                  <div class="flex items-center gap-2">
                    <span class="text-[0.6875rem] font-bold text-content truncate">{{ task.name }}</span>
                    <span class="text-[0.5625rem] font-black uppercase tracking-wider px-2 py-0.5 rounded-md shrink-0"
                          :class="[
                            task.type === 'decompression' ? 'bg-blue-500/10 text-blue-400' : 'bg-purple-500/10 text-purple-400'
                          ]">
                      {{ task.type === 'decompression' ? appStore.t('tasks.type.decompress') : appStore.t('tasks.type.compress') }}
                    </span>
                    <!-- 密码/锁图标 -->
                    <i v-if="task.password" class="pi pi-lock text-[0.5625rem] text-amber-400 shrink-0" :title="appStore.t('progress.password_used')"></i>
                    <i v-if="task.passwordRequired" class="pi pi-exclamation-triangle text-[0.625rem] text-rose-400 shrink-0 animate-pulse" :title="appStore.t('progress.password_needed')"></i>
                  </div>
                  <!-- 状态 + 进度 -->
                  <div class="flex items-center gap-2 mt-1">
                    <span class="text-[0.625rem] text-dim uppercase tracking-tight">{{ statusLabel(task.status) }}</span>
                    <span v-if="['preparing', 'running', 'extracting', 'compressing', 'finalizing'].includes(task.status)"
                          class="text-[0.625rem] font-mono text-primary font-bold">{{ task.progress }}%</span>
                    <span v-if="task.speed" class="text-[0.5625rem] font-mono text-dim ml-1">{{ task.speed }}</span>
                  </div>
                  <!-- 进度条 -->
                  <div v-if="['preparing', 'running', 'extracting', 'compressing', 'finalizing'].includes(task.status)"
                       class="h-1 bg-input rounded-full mt-1.5 overflow-hidden">
                    <div class="h-full bg-primary rounded-full transition-all duration-700"
                         :style="{ width: `${Math.max(task.progress, 1)}%` }"></div>
                  </div>
                  <!-- 当前处理文件 + 阶段 -->
                  <div v-if="task.currentFile" class="text-[0.5625rem] text-dim font-mono mt-1 truncate opacity-60" :title="task.currentFile">
                    {{ task.currentFile.split(/[\\/]/).pop() }}
                  </div>
                  <!-- 输出路径 -->
                  <div class="text-[0.5625rem] text-dim font-mono mt-1 truncate opacity-50" :title="task.outputPath">
                    → {{ task.outputPath || appStore.t('decompress.config.output_auto') }}
                  </div>
                </div>

                <!-- 操作按钮组 -->
                <div class="flex items-center gap-1.5 shrink-0">
                  <button
                    v-if="task.outputPath"
                    @click.stop="openTaskFolder(task)"
                    class="w-7 h-7 rounded-lg flex items-center justify-center text-dim hover:text-primary hover:bg-primary/10 transition-all"
                    :title="appStore.t('tasks.open_folder')">
                    <i class="pi pi-folder-open text-[0.75rem]"></i>
                  </button>

                  <button
                    v-if="['failed', 'cancelled'].includes(task.status)"
                    @click.stop="retryTask(task)"
                    :disabled="retryingTaskId === task.id"
                    class="w-7 h-7 rounded-lg flex items-center justify-center text-dim hover:text-primary hover:bg-primary/10 transition-all disabled:opacity-50"
                    :title="appStore.t('tasks.retry')">
                    <i :class="retryingTaskId === task.id ? 'pi pi-spin pi-spinner' : 'pi pi-refresh'" class="text-[0.75rem]"></i>
                  </button>

                  <button
                    v-if="['preparing', 'running', 'extracting', 'compressing', 'finalizing'].includes(task.status)"
                    @click.stop="cancelTask(task)"
                    class="w-7 h-7 rounded-lg flex items-center justify-center text-dim hover:text-red-400 hover:bg-red-500/10 transition-all"
                    :title="appStore.t('tasks.cancel')">
                    <i class="pi pi-stop-circle text-[0.75rem]"></i>
                  </button>
                </div>
              </div>

              <!-- 失败详情 -->
              <div v-if="task.status === 'failed' && task.error"
                   class="px-3.5 pb-2.5 text-[0.625rem] text-red-400/80 font-mono break-all">
                <div class="flex items-start gap-2">
                  <span class="flex-1">{{ task.error }}</span>
                  <button
                    @click.stop="copyToClipboard(task.error || '')"
                    class="w-6 h-6 rounded flex items-center justify-center text-dim hover:text-content hover:bg-red-500/10 transition-colors shrink-0"
                    :title="appStore.t('common.copy')">
                    <i class="pi pi-copy text-[0.6875rem]"></i>
                  </button>
                </div>
              </div>
            </div>

            <div v-if="sortedTasks.length === 0" class="py-8 text-center text-[0.6875rem] text-dim">
              {{ appStore.t('tasks.empty') }}
            </div>
          </div>
        </div>
      </transition>
    </div>
  </transition>
</template>

<style scoped>
.global-progress-bar {
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.progress-slide-enter-active,
.progress-slide-leave-active {
  transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.progress-slide-enter-from,
.progress-slide-leave-to {
  opacity: 0;
  transform: translateY(20px);
}

.panel-slide-enter-active,
.panel-slide-leave-active {
  transition: all 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.panel-slide-enter-from,
.panel-slide-leave-to {
  opacity: 0;
  transform: translateY(10px) scale(0.97);
}
</style>
