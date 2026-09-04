<script setup lang="ts">
import { computed, ref } from 'vue'
import { useTaskStore, type Task } from '@/stores/task'
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { extractErrorMessage } from '@/utils'
import { formatProgressPercent } from '@/utils/progress'
import SmoothProgressValue from '@/components/ui/SmoothProgressValue.vue'

const taskStore = useTaskStore()
const appStore = useAppStore()
const tauriCommands = useTauriCommands()
const isExpanded = ref(false)
const isMinimized = ref(false)
const retryingTaskId = ref<string | null>(null)

const ACTIVE_STATUSES = new Set(['preparing', 'running', 'extracting', 'compressing', 'finalizing', 'paused', 'cancelling'])
const FINISHED_STATUSES = new Set(['completed', 'failed', 'cancelled'])
const STATUS_ORDER: Record<string, number> = {
  preparing: 0, running: 0, extracting: 0, compressing: 0, finalizing: 0, paused: 0, cancelling: 0,
  pending: 1, failed: 2, completed: 3, cancelled: 4
}

const taskStats = computed(() => {
  let activeCount = 0
  let completedCount = 0
  let progressTotal = 0
  let runningTask: Task | undefined

  for (const task of taskStore.tasks) {
    progressTotal += task.progress
    if (!FINISHED_STATUSES.has(task.status)) activeCount++
    if (task.status === 'completed') completedCount++
    if (!runningTask && ACTIVE_STATUSES.has(task.status)) runningTask = task
  }

  const totalCount = taskStore.tasks.length
  return {
    activeCount,
    completedCount,
    totalCount,
    overallProgress: totalCount === 0 ? 0 : Math.round(progressTotal / totalCount),
    runningTask
  }
})

const activeCount = computed(() => taskStats.value.activeCount)
const hasActiveTasks = computed(() => activeCount.value > 0)
const hasRunningTasks = computed(() => taskStore.tasks.some(task =>
  ['preparing', 'running', 'extracting', 'compressing', 'finalizing', 'cancelling'].includes(task.status)
))
const hasPausedTasks = computed(() => taskStore.tasks.some(task => task.status === 'paused'))
const completedCount = computed(() => taskStats.value.completedCount)
const totalCount = computed(() => taskStats.value.totalCount)
const overallProgress = computed(() => taskStats.value.overallProgress)
const runningTask = computed(() => taskStats.value.runningTask)

const currentTaskName = computed(() => runningTask.value?.name || '')

const taskTypeLabel = (task: Task) => {
  if (task.type === 'decompression') return appStore.t('tasks.type.decompress')
  if (task.workloadKind === 'image') return '图片压缩'
  if (task.workloadKind === 'video') return '视频压缩'
  if (task.workloadKind === 'pdf') return 'PDF 优化'
  return appStore.t('tasks.type.compress')
}

const taskTypeClass = (task: Task) => {
  if (task.type === 'decompression') return 'bg-blue-500/10 text-blue-400'
  if (task.workloadKind === 'image') return 'bg-cyan-500/10 text-cyan-400'
  if (task.workloadKind === 'video') return 'bg-violet-500/10 text-violet-400'
  if (task.workloadKind === 'pdf') return 'bg-rose-500/10 text-rose-400'
  return 'bg-purple-500/10 text-purple-400'
}

const isArchiveTask = (task: Task) => !task.workloadKind || task.workloadKind === 'archive'

const formatEta = (seconds?: number) => {
  if (seconds === undefined || !Number.isFinite(seconds)) return ''
  if (seconds < 60) return `${Math.max(1, Math.ceil(seconds))}s`
  const minutes = Math.ceil(seconds / 60)
  if (minutes < 60) return `${minutes}m`
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`
}

const isVisible = computed(() => taskStore.tasks.length > 0)

const sortedTasks = computed(() =>
  [...taskStore.tasks].sort((a, b) => {
    return (STATUS_ORDER[a.status] ?? 5) - (STATUS_ORDER[b.status] ?? 5)
  })
)

const statusIcon = (status: string) => {
  if (status === 'completed') return 'pi pi-check-circle text-green-400'
  if (status === 'failed') return 'pi pi-exclamation-circle text-red-400'
  if (status === 'cancelled') return 'pi pi-ban text-muted'
  if (status === 'paused') return 'pi pi-pause-circle text-amber-400'
  if (['preparing', 'running', 'extracting', 'compressing', 'finalizing', 'cancelling'].includes(status)) return 'pi pi-spin pi-spinner text-primary'
  return 'pi pi-clock text-muted'
}

const statusLabel = (status: string) => {
  switch (status) {
    case 'pending': return appStore.t('tasks.status.pending')
    case 'preparing': return appStore.t('tasks.status.preparing')
    case 'running': case 'extracting': case 'compressing': return appStore.t('tasks.status.running')
    case 'finalizing': return appStore.t('tasks.status.finalizing')
    case 'cancelling': return appStore.t('tasks.status.cancelling')
    case 'paused': return appStore.t('tasks.status.paused')
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
    case 'Verifying': return '验证输出'
    case 'Finalizing': return appStore.t('tasks.status.finalizing')
    case 'password-attempt': return '验证解压密码'
    case 'Probing': return '重新探测'
    case 'Encoding': return '视频编码'
    case 'Validating': return '完整验证'
    case 'Publishing': return '原子发布'
    case 'still-encoding': return '仍在编码'
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
        deleteAfter: task.recycleSourceAfterExtract ?? false,
        createSubdirectory: task.extractToSubfolder ?? false,
        password: task.password || undefined,
        fileFilter: task.fileFilter || null,
        selectedEntries: task.selectedEntries,
        conflictPolicy: appStore.settings.conflictPolicy
      }
      taskStore.updateTaskStatus(task.id, 'preparing')
      await tauriCommands.decompressFile(task.sourceFiles[0], options, task.id)
    } else if (task.type === 'compression') {
      taskStore.updateTaskStatus(task.id, 'pending')
      taskStore.updateTaskStatus(task.id, 'compressing')
      const options = task.compressionOptions || {
        format: task.format || 'zip',
        level: 6,
        password: task.password || undefined
      }
      await tauriCommands.compressFiles(
        task.id,
        task.sourceFiles,
        task.outputPath,
        options
      )
      taskStore.updateTaskStatus(task.id, 'completed')
    }
  } catch (error) {
    const finalReason = extractErrorMessage(error)
    taskStore.failTask(task.id, finalReason)
    appStore.setError(`${appStore.t('common.error')}: ${finalReason}`)
  } finally {
    retryingTaskId.value = null
  }
}

const cancelTask = async (task: Task) => {
  await taskStore.cancelTask(task.id)
}

const pauseTask = async (task: Task) => {
  await taskStore.pauseTask(task.id)
}

const resumeTask = async (task: Task) => {
  await taskStore.resumeTask(task.id)
}

const copyToClipboard = async (text: string) => {
  try { await navigator.clipboard.writeText(text) } catch { /* ignore */ }
}
</script>

<template>
  <transition name="progress-slide">
    <div v-if="isVisible && !isMinimized"
         class="global-progress-bar relative z-[600] select-none w-full" data-testid="global-progress-bar">

      <!-- 紧凑指示器：点击展开 -->
      <div
           @click="isExpanded = !isExpanded"
           data-testid="global-progress-summary"
           class="progress-summary flex items-center gap-2 px-3 py-2 rounded-xl bg-gradient-to-br from-card via-card/95 to-card/90 backdrop-blur-3xl border border-primary/40 shadow-lg cursor-pointer hover:border-primary/70 transition-all duration-300 w-full min-w-0">
        <!-- 环形进度 -->
        <div class="progress-ring-wrap relative w-10 h-10 shrink-0">
          <svg class="w-9 h-9 -rotate-90" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" fill="none" class="text-input opacity-75"/>
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" fill="none" stroke-dasharray="62.83"
                    :stroke-dashoffset="62.83 - (62.83 * overallProgress) / 100"
                    class="text-primary transition-all duration-1000 progress-ring"/>
          </svg>
          <span class="absolute inset-0 flex items-center justify-center text-xs font-black font-mono text-content">
            <SmoothProgressValue :value="overallProgress" :decimals="0" />
          </span>
        </div>

        <!-- 摘要信息 -->
        <div class="progress-copy flex min-w-0 flex-1 flex-col gap-1 pr-12">
          <div class="flex min-w-0 items-center gap-2 whitespace-nowrap">
            <i v-if="hasRunningTasks" class="pi pi-spin pi-spinner text-[0.75rem] text-primary"></i>
            <i v-else-if="hasPausedTasks" class="pi pi-pause-circle text-[0.75rem] text-amber-400"></i>
            <i v-else class="pi pi-check-circle text-[0.75rem] text-green-400"></i>
            <span class="text-[0.75rem] font-bold text-content whitespace-nowrap">
              {{ hasActiveTasks ? `${activeCount} ${appStore.t('tasks.active')}` : appStore.t('tasks.all_done') }}
            </span>
          </div>
          <div v-if="runningTask" class="flex min-w-0 items-center gap-1.5 whitespace-nowrap text-xs">
            <span v-if="currentTaskName" class="min-w-0 flex-1 truncate text-muted" :title="currentTaskName">{{ currentTaskName }}</span>
            <span v-if="runningTask.stage" class="shrink-0 text-dim">· {{ stageLabel(runningTask.stage) }}</span>
            <i v-if="runningTask.password" class="pi pi-lock text-xs text-amber-400" :title="appStore.t('progress.password_used')"></i>
            <i v-if="runningTask.passwordRequired" class="pi pi-exclamation-triangle text-xs text-rose-400" :title="appStore.t('progress.password_needed')"></i>
          </div>
          <div v-if="runningTask" class="flex min-w-0 items-center gap-2 whitespace-nowrap text-xs font-mono">
            <SmoothProgressValue :value="runningTask.progress" class="font-black text-primary" />
            <span v-if="runningTask.speed" class="shrink-0 text-primary/70">{{ runningTask.speed }}</span>
            <span v-if="runningTask.currentFile" class="min-w-0 flex-1 truncate text-dim" :title="runningTask.currentFile">{{ runningTask.currentFile.split(/[\\/]/).pop() }}</span>
          </div>
        </div>

        <div class="progress-summary-actions absolute right-2 top-2 flex items-center gap-0.5">
          <span class="w-6 h-6 rounded-md flex items-center justify-center text-dim" :title="isExpanded ? '收起任务监控' : '展开任务监控'">
            <i :class="isExpanded ? 'pi pi-chevron-left' : 'pi pi-chevron-right'" class="progress-chevron text-xs"></i>
          </span>
          <button @click.stop="isMinimized = true" class="w-6 h-6 rounded-md flex items-center justify-center text-dim hover:text-content hover:bg-primary/10 transition-all" :title="appStore.t('common.minimize')">
            <i class="pi pi-minus text-xs"></i>
          </button>
        </div>
      </div>

      <!-- 展开的任务列表面板 -->
      <transition name="panel-slide">
        <div v-if="isExpanded"
              class="progress-panel absolute rounded-2xl bg-card border border-primary/40 shadow-2xl overflow-hidden flex flex-col">
          <!-- 面板头部 -->
          <div class="px-5 py-3.5 border-b border-subtle/20 shrink-0">
            <div class="flex items-center justify-between mb-2">
              <span class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('tasks.monitor') }}</span>
              <div class="flex items-center gap-3">
                <span class="text-sm font-mono text-primary font-black">{{ completedCount }}/{{ totalCount }}</span>
                <button
                  v-if="taskStore.tasks.some(t => ['completed', 'failed', 'cancelled'].includes(t.status))"
                  @click.stop="taskStore.clearFinishedTasks()"
                  class="text-sm text-dim hover:text-red-400 transition-colors font-bold uppercase tracking-wider">
                  {{ appStore.t('tasks.clear_done') }}
                </button>
              </div>
            </div>
            <!-- 总进度条 -->
            <div class="h-2 bg-input rounded-full overflow-hidden relative">
              <div class="h-full bg-primary rounded-full transition-all duration-1000 progress-bar-fill"
                   :style="{ width: `${overallProgress}%` }"></div>
              <div v-if="hasRunningTasks" class="absolute inset-0 shimmer-overlay"></div>
            </div>
            <!-- 实时信息摘要栏 -->
            <div v-if="runningTask" class="flex items-center gap-4 mt-2 text-xs text-dim">
              <span v-if="runningTask.stage">{{ appStore.t('progress.stage') }}: {{ stageLabel(runningTask.stage) }}</span>
              <span v-if="runningTask.speed">{{ appStore.t('progress.speed') }}: {{ runningTask.speed }}</span>
              <span v-if="runningTask.etaSeconds !== undefined">{{ appStore.t('progress.remaining') }}: {{ formatEta(runningTask.etaSeconds) }}</span>
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
                    <span class="text-sm font-bold text-content truncate">{{ task.name }}</span>
                    <span class="text-xs font-black uppercase tracking-wider px-2 py-0.5 rounded-md shrink-0"
                          data-testid="global-task-kind"
                          :class="taskTypeClass(task)">
                      {{ taskTypeLabel(task) }}
                    </span>
                    <!-- 密码/锁图标 -->
                    <i v-if="task.password" class="pi pi-lock text-xs text-amber-400 shrink-0" :title="appStore.t('progress.password_used')"></i>
                    <i v-if="task.passwordRequired" class="pi pi-exclamation-triangle text-sm text-rose-400 shrink-0 animate-pulse" :title="appStore.t('progress.password_needed')"></i>
                  </div>
                  <!-- 状态 + 进度 -->
                  <div class="flex items-center gap-2 mt-1">
                    <span class="text-sm text-dim uppercase tracking-tight">{{ statusLabel(task.status) }}</span>
                    <SmoothProgressValue v-if="['preparing', 'running', 'extracting', 'compressing', 'finalizing', 'paused', 'cancelling'].includes(task.status)" :value="task.progress" class="text-sm font-mono text-primary font-bold" />
                    <span v-if="task.speed && ACTIVE_STATUSES.has(task.status)" class="text-xs font-mono text-dim ml-1">{{ task.speed }}</span>
                    <span v-if="task.etaSeconds !== undefined" class="text-xs font-mono text-dim ml-1">ETA {{ formatEta(task.etaSeconds) }}</span>
                  </div>
                  <!-- 进度条 -->
                  <div v-if="['preparing', 'running', 'extracting', 'compressing', 'finalizing', 'paused', 'cancelling'].includes(task.status)"
                       class="h-1 bg-input rounded-full mt-1.5 overflow-hidden relative">
                    <div class="h-full bg-primary rounded-full transition-all duration-700 progress-bar-fill"
                         :style="{ width: `${task.progress}%` }"></div>
                    <div v-if="task.status !== 'paused'" class="absolute inset-0 shimmer-overlay"></div>
                  </div>
                  <!-- 当前处理文件 + 阶段 -->
                  <div v-if="task.currentFile" class="text-xs text-dim font-mono mt-1 truncate opacity-90" :title="task.currentFile">
                    {{ task.currentFile.split(/[\\/]/).pop() }}
                  </div>
                  <div v-if="task.currentPassword" class="text-xs text-primary font-mono mt-1 truncate opacity-90" :title="task.currentPassword">
                    正在验证 {{ task.currentPassword }}
                    <template v-if="task.passwordAttemptCurrent">
                      · {{ task.passwordAttemptCurrent }}<template v-if="task.passwordAttemptTotal">/{{ task.passwordAttemptTotal }}</template>
                    </template>
                  </div>
                  <!-- 输出路径 -->
                  <div class="text-xs text-dim font-mono mt-1 truncate opacity-85" :title="task.outputPath">
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
                    v-if="['failed', 'cancelled'].includes(task.status) && (!task.workloadKind || task.workloadKind === 'archive')"
                    @click.stop="retryTask(task)"
                    :disabled="retryingTaskId === task.id"
                    class="w-7 h-7 rounded-lg flex items-center justify-center text-dim hover:text-primary hover:bg-primary/10 transition-all disabled:opacity-85"
                    :title="appStore.t('tasks.retry')">
                    <i :class="retryingTaskId === task.id ? 'pi pi-spin pi-spinner' : 'pi pi-refresh'" class="text-[0.75rem]"></i>
                  </button>

                  <button
                    v-if="isArchiveTask(task) && ['preparing', 'running', 'extracting', 'compressing', 'finalizing'].includes(task.status)"
                    @click.stop="pauseTask(task)"
                    data-testid="pause-task"
                    class="w-7 h-7 rounded-lg flex items-center justify-center text-dim hover:text-amber-400 hover:bg-amber-500/10 transition-all"
                    :title="appStore.t('tasks.pause_one')"
                    :aria-label="appStore.t('tasks.pause_one')">
                    <i class="pi pi-pause text-[0.75rem]"></i>
                  </button>

                  <button
                    v-if="isArchiveTask(task) && task.status === 'paused'"
                    @click.stop="resumeTask(task)"
                    data-testid="resume-task"
                    class="w-7 h-7 rounded-lg flex items-center justify-center text-dim hover:text-green-400 hover:bg-green-500/10 transition-all"
                    :title="appStore.t('tasks.resume_one')"
                    :aria-label="appStore.t('tasks.resume_one')">
                    <i class="pi pi-play text-[0.75rem]"></i>
                  </button>

                  <button
                    v-if="['preparing', 'running', 'extracting', 'compressing', 'finalizing', 'paused', 'cancelling'].includes(task.status)"
                    @click.stop="cancelTask(task)"
                    data-testid="cancel-task"
                    class="w-7 h-7 rounded-lg flex items-center justify-center text-dim hover:text-red-400 hover:bg-red-500/10 transition-all"
                    :title="appStore.t('tasks.cancel')"
                    :aria-label="appStore.t('tasks.cancel')">
                    <i class="pi pi-stop-circle text-[0.75rem]"></i>
                  </button>
                </div>
              </div>

              <!-- 失败详情 -->
              <div v-if="task.status === 'failed' && task.error"
                   class="px-3.5 pb-2.5 text-sm text-red-400/80 font-mono break-all">
                <div class="flex items-start gap-2">
                  <span class="flex-1">{{ task.error }}</span>
                  <button
                    @click.stop="copyToClipboard(task.error || '')"
                    class="w-6 h-6 rounded flex items-center justify-center text-dim hover:text-content hover:bg-red-500/10 transition-colors shrink-0"
                    :title="appStore.t('common.copy')">
                    <i class="pi pi-copy text-sm"></i>
                  </button>
                </div>
              </div>
            </div>

            <div v-if="sortedTasks.length === 0" class="py-8 text-center text-sm text-dim">
              {{ appStore.t('tasks.empty') }}
            </div>
          </div>
        </div>
      </transition>
    </div>
  </transition>

  <!-- 最小化状态：显示一个小圆点指示器 -->
  <transition name="dot-fade">
    <button
      v-if="isVisible && isMinimized"
      @click="isMinimized = false"
      class="relative z-[600] mx-auto block w-3 h-3 rounded-full bg-primary shadow-[0_0_12px_rgba(14,165,233,0.6)] hover:scale-125 hover:shadow-[0_0_20px_rgba(14,165,233,0.8)] transition-all duration-300 cursor-pointer"
      :class="{ 'animate-pulse': hasActiveTasks }"
      :title="appStore.t('tasks.show_progress')">
    </button>
  </transition>
</template>

<style scoped>
.global-progress-bar {
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.progress-summary { min-height: 4.5rem; }

.progress-panel {
  left: calc(100% + 0.75rem);
  bottom: 0;
  width: min(30rem, calc(100vw - 16rem));
  min-width: 20rem;
  max-height: min(65vh, 32rem);
}

@media (max-width: 840px) {
  .progress-summary {
    justify-content: center;
    padding: 0.35rem;
  }
  .progress-copy,
  .progress-chevron,
  .progress-summary > button {
    display: none;
  }
  .progress-panel {
    width: calc(100vw - 7rem);
    min-width: 18rem;
  }
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

/* 进度条流光动效 */
.progress-bar-fill {
  position: relative;
  background: linear-gradient(90deg,
    rgb(14, 165, 233) 0%,
    rgb(59, 130, 246) 50%,
    rgb(14, 165, 233) 100%);
  background-size: 200% 100%;
  animation: progress-shimmer 2s ease-in-out infinite;
}

@keyframes progress-shimmer {
  0%, 100% {
    background-position: 0% 50%;
  }
  50% {
    background-position: 100% 50%;
  }
}

/* 流光遮罩 */
.shimmer-overlay {
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.3) 50%,
    transparent 100%
  );
  background-size: 200% 100%;
  animation: shimmer-move 1.5s ease-in-out infinite;
  pointer-events: none;
}

@keyframes shimmer-move {
  0% {
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
  }
}

/* 环形进度脉冲 */
.progress-ring {
  filter: drop-shadow(0 0 6px rgba(14, 165, 233, 0.6));
  animation: ring-pulse 2s ease-in-out infinite;
}

@keyframes ring-pulse {
  0%, 100% {
    filter: drop-shadow(0 0 6px rgba(14, 165, 233, 0.6));
  }
  50% {
    filter: drop-shadow(0 0 12px rgba(14, 165, 233, 0.9));
  }
}

/* 最小化圆点淡入淡出 */
.dot-fade-enter-active,
.dot-fade-leave-active {
  transition: all 0.3s ease;
}

.dot-fade-enter-from,
.dot-fade-leave-to {
  opacity: 0;
  transform: scale(0.5);
}
</style>
