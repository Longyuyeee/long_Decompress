<script setup lang="ts">
import { ref, computed } from 'vue'
import { useTaskStore, type Task, type TaskType } from '@/stores/task'
import { useAppStore } from '@/stores/app'
import { usePasswordStore } from '@/stores/password'
import { useTauriCommands } from '@/composables/useTauriCommands'
import Modal from '@/components/ui/Modal.vue'
import ResourcePreflightCard from '@/components/tasks/ResourcePreflightCard.vue'
import SmoothProgressValue from '@/components/ui/SmoothProgressValue.vue'
import OverflowTooltip from '@/components/ui/OverflowTooltip.vue'
import { open } from '@tauri-apps/api/dialog'
import { formatProgressPercent } from '@/utils/progress'
import { formatFileSize } from '@/utils'
import { sortTasksByName } from '@/utils/taskOrdering'

const props = defineProps<{
  selectedTaskIds?: Set<string>
  statusFilter?: string | string[]
  taskType?: TaskType
}>()

const emit = defineEmits<{
  'toggle-task': [taskId: string]
  'select-all-pending': []
  'deselect-all': []
  'retry-with-password': [taskId: string]
  'cancel-task': [taskId: string]
  'pause-task': [taskId: string]
  'resume-task': [taskId: string]
  'set-config-mode': [taskId: string, mode: 'global' | 'individual']
}>()

const taskStore = useTaskStore()
const appStore = useAppStore()
const passwordStore = usePasswordStore()
const displayTasks = computed(() => {
  const typedTasks = props.taskType
    ? taskStore.tasks.filter(task => task.type === props.taskType)
    : taskStore.tasks
  const filteredTasks = !props.statusFilter || props.statusFilter === 'all'
    ? typedTasks
    : typedTasks.filter(task => {
        const filters = Array.isArray(props.statusFilter) ? props.statusFilter : [props.statusFilter]
        return filters.includes(task.status)
      })
  return sortTasksByName(filteredTasks)
})
const pendingDisplayTasks = computed(() => displayTasks.value.filter(task => task.status === 'pending'))
const tauriCommands = useTauriCommands()
const expandedTasks = ref<Set<string>>(new Set())
const showPasswordInput = ref<string | null>(null)
const showContentsModal = ref(false)
const contentsList = ref<string[]>([])
const contentsFile = ref('')
// contentsLoading now computed above
const taskToRemove = ref<string | null>(null)
const loadingCounter = ref(0)
const contentsLoading = computed(() => loadingCounter.value > 0)

const isSelected = (taskId: string) => props.selectedTaskIds?.has(taskId) ?? false

const previewContents = async (task: Task) => {
  loadingCounter.value++
  contentsFile.value = task.name || task.sourceFiles[0] || 'Archive'
  try {
    contentsList.value = await tauriCommands.listArchiveContents(
      task.sourceFiles[0],
      task.password || undefined
    )
    showContentsModal.value = true
  } catch (e) {
    contentsList.value = []
    showContentsModal.value = true
  } finally {
    loadingCounter.value--
  }
}

const testIntegrity = async (task: Task) => {
  loadingCounter.value++
  try {
    const result = await tauriCommands.testArchiveIntegrity(
      task.sourceFiles[0],
      task.password || undefined
    )
    appStore.setSuccess(result)
  } catch (e: any) {
    appStore.setError(`Integrity check failed: ${e}`)
  } finally {
    loadingCounter.value--
  }
}

const handleRemoveTask = (taskId: string) => {
  taskToRemove.value = taskId
}
const confirmRemoveTask = () => {
  if (taskToRemove.value) {
    taskStore.removeTask(taskToRemove.value)
    taskToRemove.value = null
  }
}

const toggleExpand = (taskId: string) => {
  if (expandedTasks.value.has(taskId)) {
    expandedTasks.value.delete(taskId)
  } else {
    expandedTasks.value.add(taskId)
  }
}

const handleSelectOutputDir = async (task: Task) => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: appStore.t('decompress.config.output_select')
    })
    if (selected && typeof selected === 'string') {
      task.outputPath = selected
    }
  } catch (err) {
    appStore.setError(`${appStore.t('common.error')}: ${err}`)
  }
}

const handleSetSameDir = (task: Task) => {
  if (task.sourceFiles.length > 0) {
    const sourcePath = task.sourceFiles[0]
    // 提取父目录
    const parentDir = sourcePath.substring(0, Math.max(sourcePath.lastIndexOf('/'), sourcePath.lastIndexOf('\\')))
    task.outputPath = parentDir
  }
}

const getStatusIcon = (status: string) => {
  switch (status) {
    case 'pending': return '🕒'
    case 'preparing': return '⚙️'
    case 'running': return '▶️'
    case 'extracting': return '📦'
    case 'compressing': return '🗜️'
    case 'cancelling': return '⏳'
    case 'paused': return '⏸️'
    case 'completed': return '✅'
    case 'failed': return '❌'
    case 'cancelled': return '⛔'
    default: return '❓'
  }
}

const getStatusColor = (status: string) => {
  switch (status) {
    case 'pending': return 'text-gray-500'
    case 'preparing': return 'text-blue-400'
    case 'running': return 'text-blue-500 animate-pulse'
    case 'extracting': return 'text-blue-500 animate-pulse'
    case 'compressing': return 'text-blue-500 animate-pulse'
    case 'cancelling': return 'text-orange-400 animate-pulse'
    case 'paused': return 'text-amber-400'
    case 'completed': return 'text-green-500'
    case 'failed': return 'text-red-500'
    case 'cancelled': return 'text-orange-500'
    default: return 'text-muted'
  }
}

const getStatusText = (status: string) => {
  return appStore.t(`decompress.status.${status}`) || status
}

const getStageText = (task: Task) => {
  if (task.status === 'paused') return getStatusText(task.status)
  switch (task.stage) {
    case 'password-attempt': return '验证解压密码'
    case 'Pre-checking': return '执行预检'
    case 'Extracting': return '正在解压'
    case 'Verifying': return '验证输出'
    case 'Finalizing': return '提交结果'
    case 'Probing': return '重新探测'
    case 'Encoding': return '视频编码'
    case 'Validating': return '完整验证'
    case 'Publishing': return '原子发布'
    case 'still-encoding': return '仍在编码'
    default: return getStatusText(task.status)
  }
}

const getFormatBadgeColor = (format?: string) => {
  switch (format?.toLowerCase()) {
    case 'zip': return `bg-primary/20 text-primary border-primary/30 shadow-[0_0_10px_rgba(var(--color-primary-rgb),0.15)]`
    case '7z': return 'bg-purple-500/20 text-purple-400 border-purple-500/30 shadow-[0_0_10px_rgba(168,85,247,0.15)]'
    case 'rar': return 'bg-red-500/20 text-red-400 border-red-500/30 shadow-[0_0_10px_rgba(239,68,68,0.15)]'
    default: return 'bg-input text-muted border-subtle'
  }
}

const getSeverityClass = (severity: string) => {
  switch (severity) {
    case 'error': return 'text-red-400'
    case 'warning': return 'text-yellow-400'
    case 'success': return 'text-green-400'
    default: return 'text-muted'
  }
}

// 物理高度过渡钩子：解决收缩跳动
const onBeforeEnter = (el: any) => {
  el.style.height = '0'
  el.style.opacity = '0'
  el.style.marginTop = '0'
  el.style.marginBottom = '0'
}

const onEnter = (el: any) => {
  el.style.height = el.scrollHeight + 'px'
  el.style.opacity = '1'
  el.style.marginTop = '4px'
  el.style.marginBottom = '8px'
}

const onBeforeLeave = (el: any) => {
  el.style.height = el.scrollHeight + 'px'
  el.style.opacity = '1'
  el.style.marginTop = '4px'
  el.style.marginBottom = '8px'
}

const onLeave = (el: any) => {
  el.offsetHeight // 强制物理重绘
  el.style.height = '0'
  el.style.opacity = '0'
  el.style.marginTop = '0'
  el.style.marginBottom = '0'
}
</script>

<template>
  <div class="aero-table-container w-full h-full min-w-0 max-w-full overflow-x-hidden flex flex-col">
    <!-- 智慧表格 (重构为极简列表模式) -->
    <div class="glass-table w-full max-w-full flex-1 flex flex-col min-h-0 min-w-0 overflow-x-hidden">
      <!-- 表头 (高度压缩，字体减小) -->
      <div class="table-header sticky top-0 z-20 grid min-w-0 max-w-full items-center px-6 py-2.5 border-b border-subtle bg-card/95 backdrop-blur-xl text-dim text-xs font-bold tracking-[0.1em] uppercase shrink-0">
        <!-- 复选框列 -->
        <div class="w-8 shrink-0 flex items-center justify-center">
          <button
            v-if="props.selectedTaskIds && pendingDisplayTasks.length > 0"
            class="w-4 h-4 rounded border border-subtle/50 flex items-center justify-center hover:border-primary transition-colors"
            :class="pendingDisplayTasks.every(t => isSelected(t.id)) ? 'bg-primary border-primary' : 'bg-input/50'"
            @click.stop="pendingDisplayTasks.every(t => isSelected(t.id)) ? emit('deselect-all') : emit('select-all-pending')"
            :title="appStore.t('tasks.toggle_all')"
          >
            <i v-if="pendingDisplayTasks.every(t => isSelected(t.id))" class="pi pi-check text-sm text-white"></i>
          </button>
        </div>
        <div class="min-w-0">{{ appStore.t('decompress.column.name') }}</div>
        <div class="min-w-0 text-center">{{ appStore.t('decompress.column.status') }}</div>
        <div aria-hidden="true"></div>
      </div>

      <!-- 表格内容 (高密度布局 + 物理隔断) -->
      <div class="table-body flex-1 min-w-0 max-w-full overflow-y-auto overflow-x-hidden custom-scrollbar p-3">
        <TransitionGroup name="task-depart">
        <div v-for="task in displayTasks" :key="task.id" class="task-row-container mb-1.5 last:mb-0 group/row">
          <div
            class="task-row grid min-w-0 max-w-full items-center px-4 py-2 bg-card/40 border border-subtle/40 rounded-lg hover:border-primary/30 hover:bg-card/60 transition-all duration-200 cursor-pointer relative overflow-hidden shadow-sm"
            :class="{ 'has-password-input': task.passwordRequired }"
            data-testid="task-row"
            :data-task-id="task.id"
            @click="toggleExpand(task.id)"
            role="button"
            tabindex="0"
            :aria-expanded="expandedTasks.has(task.id)"
            @keydown.enter="toggleExpand(task.id)"
            @keydown.space.prevent="toggleExpand(task.id)"
          >
            <!-- 状态指示条 -->
            <div class="absolute left-0 top-0 bottom-0 w-1 bg-primary opacity-0 group-hover/row:opacity-100 transition-opacity duration-200"></div>

            <!-- 复选框 -->
            <div class="w-8 shrink-0 flex items-center justify-center" @click.stop>
              <button
                v-if="props.selectedTaskIds && task.status === 'pending'"
                class="w-4 h-4 rounded border flex items-center justify-center transition-all"
                :class="isSelected(task.id) ? 'bg-primary border-primary' : 'border-subtle/50 bg-input/50 hover:border-primary'"
                @click="emit('toggle-task', task.id)"
              >
                <i v-if="isSelected(task.id)" class="pi pi-check text-sm text-white"></i>
              </button>
              <i v-else-if="task.status === 'completed'" class="pi pi-check-circle text-green-400/50 text-sm"></i>
              <i v-else-if="task.status === 'failed'" class="pi pi-exclamation-circle text-red-400/50 text-sm"></i>
              <i v-else-if="task.status === 'cancelled'" class="pi pi-ban text-muted/30 text-sm"></i>
              <i v-else-if="task.status === 'pending'" class="pi pi-clock text-muted/40 text-sm"></i>
              <div v-else class="w-4 h-4"></div>
            </div>

            <!-- 文件识别区 (极致紧凑) -->
            <div class="task-name-cell min-w-0 overflow-hidden flex items-center gap-3">
              <OverflowTooltip
                :text="task.name"
                class="min-w-0 flex-1 text-content font-bold text-sm tracking-tight group-hover/row:text-primary transition-colors leading-tight"
              >
                {{ task.name }}
              </OverflowTooltip>
              <span class="text-dim text-sm uppercase font-black tracking-widest bg-input/50 px-1 py-0 rounded border border-subtle/20 shrink-0">
                {{ task.format?.toUpperCase() }}
              </span>
            </div>

            <!-- 状态与执行进度 -->
            <div
              class="task-status-cell min-w-0 grid items-center gap-2"
              :class="{ 'is-terminal': !['running', 'extracting', 'compressing', 'preparing', 'finalizing', 'cancelling'].includes(task.status) }"
            >
              <!-- 状态图标和文字 -->
              <div class="task-status-primary flex min-w-0 items-center justify-center gap-1.5">
                <span class="text-lg">{{ getStatusIcon(task.status) }}</span>
                <span
                  class="text-xs font-black uppercase tracking-widest transition-all"
                  :class="getStatusColor(task.status)"
                >
                  {{ getStatusText(task.status) }}
                </span>
              </div>

              <div v-if="['running', 'extracting', 'compressing', 'preparing', 'finalizing', 'cancelling'].includes(task.status)" class="task-status-runtime min-w-0">
                <div class="flex min-w-0 items-center gap-2 whitespace-nowrap text-xs">
                  <span class="shrink-0 font-bold text-muted">{{ getStageText(task) }}</span>
                  <span v-if="task.currentFile" class="min-w-0 flex-1 truncate font-mono text-dim" :title="task.currentFile">{{ task.currentFile.split(/[\\/]/).pop() }}</span>
                  <span v-if="task.speed" class="shrink-0 font-mono text-dim">{{ task.speed }}</span>
                </div>

              <!-- 进度条（仅运行时显示） -->
              <div
                class="mt-1 h-1.5 bg-input/50 rounded-full overflow-hidden"
              >
                <div
                  class="archive-progress-fill h-full bg-primary transition-all duration-300 rounded-full"
                  :style="{ width: `${task.progress || 0}%` }"
                ></div>
              </div>
              </div>

              <!-- 进度百分比 -->
              <span
                v-if="['running', 'extracting', 'compressing', 'preparing', 'finalizing', 'paused', 'cancelling'].includes(task.status)"
                class="task-status-percent shrink-0 text-xs font-mono text-primary font-bold"
              >
                <SmoothProgressValue :value="task.progress" />
              </span>
            </div>

            <!-- 密码内联输入 (自动破解失败时在行内显示) -->
            <div v-if="task.passwordRequired" class="task-password-cell flex items-center gap-1 shrink-0 px-2" @click.stop>
              <input
                :type="showPasswordInput === task.id ? 'text' : 'password'"
                :value="task.password || ''"
                @input="(e: Event) => { task.password = (e.target as HTMLInputElement).value }"
                :placeholder="appStore.t('tasks.password.placeholder')"
                class="h-7 w-36 rounded-lg bg-yellow-500/5 border border-yellow-500/50 text-xs px-2 font-mono outline-none focus:border-yellow-400 text-yellow-400 placeholder:text-yellow-500/50"
              />
              <button @click.stop="() => { const candidates = passwordStore.findCandidatePasswords(task.name || task.sourceFiles[0]?.split(/[\\/]/).pop() || ''); if (candidates.length > 0) { task.password = candidates[0] } }"
                class="h-7 w-7 rounded-lg border border-yellow-500/50 bg-yellow-500/10 flex items-center justify-center text-yellow-400 hover:bg-yellow-500/20 transition-colors shrink-0"
                :title="appStore.t('tasks.password.fill_vault')">
                <i class="pi pi-key text-xs"></i>
              </button>
              <button
                @click.stop="emit('retry-with-password', task.id)"
                :disabled="!task.password"
                class="h-7 w-7 rounded-lg bg-yellow-500 text-white flex items-center justify-center hover:brightness-110 disabled:opacity-40 disabled:cursor-not-allowed shrink-0"
                :title="appStore.t('tasks.password.retry')"
              ><i class="pi pi-play text-xs"></i></button>
            </div>

            <!-- 行级操作：运行控制在前，展开/收起固定在最右侧 -->
            <div class="task-action-cell" data-testid="task-action-cell" @click.stop>
              <button
                v-if="task.status === 'pending'"
                type="button"
                @click="task.type === 'compression' ? emit('cancel-task', task.id) : handleRemoveTask(task.id)"
                class="task-row-action text-dim hover:text-red-400 hover:bg-red-500/10"
                :data-testid="`remove-archive-task-${task.id}`"
                :title="task.type === 'compression' ? '取消排队任务' : appStore.t('tasks.remove')">
                <i :class="task.type === 'compression' ? 'pi pi-stop-circle' : 'pi pi-times'" class="text-xs"></i>
              </button>
              <button
                v-else-if="['preparing', 'running', 'compressing', 'extracting', 'finalizing'].includes(task.status)"
                type="button"
                @click="emit('pause-task', task.id)"
                :data-testid="`pause-archive-task-${task.id}`"
                class="task-row-action text-amber-400 hover:bg-amber-500/10"
                :title="appStore.t('tasks.pause_one')">
                <i class="pi pi-pause text-xs"></i>
              </button>
              <button
                v-else-if="task.status === 'paused'"
                type="button"
                @click="emit('resume-task', task.id)"
                :data-testid="`resume-archive-task-${task.id}`"
                class="task-row-action text-green-400 hover:bg-green-500/10"
                :title="appStore.t('tasks.resume_one')">
                <i class="pi pi-play text-xs"></i>
              </button>
              <button
                v-if="['preparing', 'running', 'compressing', 'extracting', 'finalizing', 'paused'].includes(task.status)"
                type="button"
                @click="emit('cancel-task', task.id)"
                class="task-row-action text-red-400 hover:bg-red-500/10"
                :data-testid="`stop-archive-task-${task.id}`"
                :title="appStore.t('tasks.stop_one')">
                <i class="pi pi-stop-circle text-xs"></i>
              </button>
              <span
                v-if="['pending', 'preparing', 'running', 'compressing', 'extracting', 'finalizing', 'paused'].includes(task.status)"
                class="task-action-divider"
                aria-hidden="true"
              ></span>
              <button
                type="button"
                class="task-row-action task-expand-action text-muted hover:text-primary hover:bg-primary/10"
                :data-testid="`toggle-archive-task-${task.id}`"
                :title="expandedTasks.has(task.id) ? '收起任务详情' : '展开任务详情'"
                :aria-label="expandedTasks.has(task.id) ? '收起任务详情' : '展开任务详情'"
                :aria-expanded="expandedTasks.has(task.id)"
                @click="toggleExpand(task.id)"
              >
                <i :class="['pi text-sm transition-transform duration-300',
                   expandedTasks.has(task.id) ? 'pi-chevron-up text-primary' : 'pi-chevron-down']"></i>
              </button>
            </div>
          </div>

          <Transition 
            name="aero-drawer"
            @before-enter="onBeforeEnter"
            @enter="onEnter"
            @before-leave="onBeforeLeave"
            @leave="onLeave"
          >
            <div v-if="expandedTasks.has(task.id)" class="details-drawer relative min-w-0 max-w-full px-2 md:px-3 pb-4 pt-2">
              <!-- 交互增强：task-detail-card 增加 hover 动效 -->
              <div data-testid="decompression-task-details" class="task-detail-card rounded-2xl bg-card border border-dashed border-primary/30 shadow-2xl overflow-hidden relative group/detail">

                <!-- 详情区内容布局：改为弹性分配，防止溢出 -->
                <div class="task-detail-layout w-full min-w-0 relative z-10">
                  <!-- 左侧：核心配置 -->
                  <div data-testid="decompression-config-panel" class="task-config-panel min-w-0 p-4 border-r border-subtle/20 flex flex-col space-y-3 transition-colors group-hover/detail:bg-primary/[0.01] overflow-y-auto overflow-x-hidden custom-scrollbar">
                    <div class="flex min-w-0 items-center justify-between">
                      <h4 class="task-config-heading min-w-0 text-primary text-xs font-black uppercase tracking-[0.2em] flex items-center gap-2 break-words">
                        <i class="pi pi-cog text-sm"></i>
                        {{ appStore.t('decompress.column.config') }}
                      </h4>
                      <div v-if="task.type === 'decompression' && task.status === 'pending'" class="config-source-switch" @click.stop>
                        <button type="button" :class="{ active: task.configurationMode === 'global' }" @click="emit('set-config-mode', task.id, 'global')">{{ appStore.t('tasks.config.global') }}</button>
                        <button type="button" :class="{ active: task.configurationMode !== 'global' }" @click="emit('set-config-mode', task.id, 'individual')">{{ appStore.t('tasks.config.individual') }}</button>
                      </div>
                    </div>

                    <div v-if="task.type === 'decompression'" class="space-y-3.5" :class="{ 'config-follows-global': task.configurationMode === 'global' }">
                      <!-- 路径行：增加 flex-wrap 兜底，但在大多数状态下保持并排 -->
                      <div class="inherited-config-control space-y-2">
                        <div class="task-output-row flex min-w-0 items-center justify-between gap-3">
                          <span class="task-output-label text-muted text-xs uppercase font-black tracking-widest opacity-90 shrink-0">{{ appStore.t('decompress.config.output') }}</span>
                          <div class="task-output-actions flex min-w-0 flex-wrap justify-end gap-1.5">
                            <button @click.stop="handleSetSameDir(task)" 
                                    class="h-6 px-2.5 rounded-md bg-primary/10 text-primary hover:bg-primary hover:text-white transition-all text-xs font-black whitespace-nowrap">
                              {{ appStore.t('decompress.config.output_same') }}
                            </button>
                            <button @click.stop="handleSelectOutputDir(task)"
                                    class="h-7 px-3 rounded-lg bg-primary text-white hover:brightness-110 active:scale-95 transition-all text-xs font-black whitespace-nowrap flex items-center gap-1.5 shadow-sm">
                              <i class="pi pi-folder-open text-xs"></i>
                              {{ appStore.t('decompress.config.output_select') }}
                            </button>
                          </div>
                        </div>
                        <div class="min-w-0 truncate px-3 py-2 rounded-xl bg-input/50 border border-subtle/50 font-mono text-sm text-content/80 shadow-inner" :title="task.outputPath || appStore.t('decompress.config.output_auto')">
                          {{ task.outputPath || appStore.t('decompress.config.output_auto') }}
                        </div>
                      </div>

                      <ResourcePreflightCard :report="task.resourcePreflight" compact />

                      <div class="inherited-config-control flex min-w-0 flex-wrap items-center gap-x-5 gap-y-3">
                        <button type="button" role="switch" :aria-checked="task.extractToSubfolder" :disabled="task.status !== 'pending'" class="task-subfolder-option flex min-w-0 items-center gap-3 cursor-pointer group/check text-left disabled:cursor-not-allowed disabled:opacity-60" @click.stop="task.extractToSubfolder = !task.extractToSubfolder">
                          <span class="w-4 h-4 shrink-0 rounded border border-subtle flex items-center justify-center transition-all group-hover/check:border-primary"
                                :class="task.extractToSubfolder ? 'bg-primary border-primary' : 'bg-input'">
                            <i v-if="task.extractToSubfolder" class="pi pi-check text-xs text-white"></i>
                          </span>
                          <span class="min-w-0 whitespace-nowrap text-sm font-bold text-muted group-hover/check:text-content transition-colors tracking-tight">{{ appStore.t('decompress.config.output_sub') }}</span>
                        </button>
                        <button type="button" role="switch" data-testid="task-recycle-source-switch" :aria-checked="task.recycleSourceAfterExtract" :disabled="task.status !== 'pending'" :title="appStore.t('decompress.config.recycle_source.desc')" class="flex min-w-0 items-center gap-3 cursor-pointer group/recycle text-left disabled:cursor-not-allowed disabled:opacity-60" @click.stop="task.recycleSourceAfterExtract = !task.recycleSourceAfterExtract">
                          <span class="w-4 h-4 shrink-0 rounded border border-subtle flex items-center justify-center transition-all group-hover/recycle:border-primary"
                                :class="task.recycleSourceAfterExtract ? 'bg-primary border-primary' : 'bg-input'">
                            <i v-if="task.recycleSourceAfterExtract" class="pi pi-check text-xs text-white"></i>
                          </span>
                          <span class="min-w-0 whitespace-nowrap text-sm font-bold text-muted group-hover/recycle:text-content transition-colors tracking-tight">{{ appStore.t('decompress.config.recycle_source') }}</span>
                        </button>
                      </div>

                      <!-- 密码输入区 (仅在自动破解失败时显示) -->
                      <div v-if="task.passwordRequired" class="space-y-1.5 p-3 rounded-xl border border-yellow-500/30 bg-yellow-500/5">
                        <div class="flex items-center gap-2">
                          <i class="pi pi-lock text-sm text-yellow-400"></i>
                          <span class="text-yellow-400 text-xs font-black uppercase tracking-widest">{{ appStore.t('tasks.password.crack_failed') }}</span>
                        </div>
                        <div class="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] gap-2">
                          <input
                            :type="showPasswordInput === task.id ? 'text' : 'password'"
                            :value="task.password || ''"
                            @input="(e: Event) => { task.password = (e.target as HTMLInputElement).value }"
                            @click.stop
                            :placeholder="appStore.t('tasks.password.placeholder')"
                            class="col-span-2 h-7 w-full min-w-0 rounded-lg bg-input/50 border border-yellow-500/50 text-sm px-3 font-mono outline-none focus:border-yellow-400 text-yellow-400 placeholder:text-yellow-500/50"
                          />
                          <button
                            @click.stop="() => { const candidates = passwordStore.findCandidatePasswords(task.name || task.sourceFiles[0]?.split(/[\\/]/).pop() || ''); if (candidates.length > 0) { task.password = candidates[0] } }"
                            class="h-7 w-7 justify-self-start rounded-lg border border-yellow-500/50 bg-yellow-500/10 flex items-center justify-center text-yellow-400 hover:bg-yellow-500/20 transition-colors shrink-0"
                            :title="appStore.t('tasks.password.fill_vault')">
                            <i class="pi pi-key text-xs"></i>
                          </button>
                          <button
                            @click.stop="emit('retry-with-password', task.id)"
                            :disabled="!task.password"
                            class="h-7 px-3 rounded-lg bg-yellow-500 text-white text-xs font-bold hover:brightness-110 disabled:opacity-40 disabled:cursor-not-allowed whitespace-nowrap"
                          >{{ appStore.t('tasks.password.retry') }}</button>
                        </div>
                      </div>

                    </div>

                    <div v-else class="space-y-3 text-xs">
                      <div class="grid min-w-0 grid-cols-[minmax(0,90px)_minmax(0,1fr)] gap-x-3 gap-y-2">
                        <span class="text-muted font-black uppercase">输出文件</span>
                        <span class="min-w-0 break-words [overflow-wrap:anywhere] font-mono text-content" :title="task.outputPath">{{ task.outputPath }}</span>
                        <span class="text-muted font-black uppercase">压缩格式</span>
                        <span class="font-mono text-primary font-black">{{ task.format?.toUpperCase() }}</span>
                        <span class="text-muted font-black uppercase">压缩等级</span>
                        <span class="font-mono text-content">{{ task.compressionOptions?.level ?? '—' }}</span>
                        <span class="text-muted font-black uppercase">源文件</span>
                        <span class="font-mono text-content">{{ task.sourceFiles.length }} 项</span>
                        <span class="text-muted font-black uppercase">保留路径</span>
                        <span class="text-content">{{ task.compressionOptions?.preserve_paths ? '是' : '否' }}</span>
                        <span class="text-muted font-black uppercase">分卷</span>
                        <span class="text-content">{{ task.compressionOptions?.split_size ? `${task.compressionOptions.split_size} MB` : '关闭' }}</span>
                        <span class="text-muted font-black uppercase">加密</span>
                        <span class="text-content">{{ task.compressionOptions?.password ? '已启用' : '关闭' }}</span>
                      </div>
                    </div>
                    <ResourcePreflightCard v-if="task.type !== 'decompression'" :report="task.resourcePreflight" compact />
                  </div>

                  <!-- 右侧：执行日志 -->
                  <div data-testid="decompression-execution-panel" class="task-execution-panel min-w-0 p-4 flex flex-col overflow-hidden">
                    <div class="execution-summary grid grid-cols-2 gap-2 mb-3 text-xs">
                      <div class="rounded-lg bg-input/40 border border-subtle/40 px-3 py-2">
                        <span class="text-muted">阶段</span>
                        <div class="font-black text-content truncate mt-0.5">{{ getStageText(task) }}</div>
                      </div>
                      <div class="rounded-lg bg-input/40 border border-subtle/40 px-3 py-2">
                        <span class="text-muted">进度</span>
                        <div class="mt-0.5 flex min-w-0 items-center gap-2 whitespace-nowrap font-mono font-black text-primary"><SmoothProgressValue :value="task.progress" /><span v-if="task.speed && !['completed', 'failed', 'cancelled'].includes(task.status)" class="min-w-0 truncate text-muted">{{ task.speed }}</span></div>
                      </div>
                      <div v-if="task.currentFile" class="task-current-file col-span-2 min-w-0 truncate rounded-lg bg-input/40 border border-subtle/40 px-3 py-2 font-mono text-content" :title="task.currentFile">
                        {{ task.currentFile }}
                      </div>
                      <div v-if="task.currentPassword" class="col-span-2 min-w-0 rounded-lg bg-primary/5 border border-primary/20 px-3 py-2">
                        <div class="flex min-w-0 items-center justify-between gap-3">
                          <span class="text-muted">正在验证</span>
                          <span v-if="task.passwordAttemptCurrent" class="shrink-0 font-mono font-black text-primary">
                            {{ task.passwordAttemptCurrent }}<template v-if="task.passwordAttemptTotal"> / {{ task.passwordAttemptTotal }}</template>
                          </span>
                        </div>
                        <div class="mt-1 min-w-0 break-words [overflow-wrap:anywhere] font-mono text-content" :title="task.currentPassword">
                          {{ task.currentPassword }}
                        </div>
                        <div class="mt-1 text-xs text-dim">为保护隐私，不在执行日志中记录密码明文</div>
                      </div>
                      <div
                        v-if="task.type === 'decompression' && (task.outputBytes !== undefined || task.totalBytes)"
                        class="col-span-2 grid min-w-0 grid-cols-2 gap-2"
                      >
                        <div class="rounded-lg bg-input/40 border border-subtle/40 px-3 py-2 min-w-0">
                          <span class="whitespace-nowrap text-muted">已产出</span>
                          <div class="mt-0.5 truncate font-mono font-black text-primary" :title="`${task.outputBytes || 0} B`">
                            {{ task.outputBytesEstimated ? '约 ' : '' }}{{ formatFileSize(task.outputBytes || 0) }}
                          </div>
                        </div>
                        <div class="rounded-lg bg-input/40 border border-subtle/40 px-3 py-2 min-w-0">
                          <span class="whitespace-nowrap text-muted">预计总量</span>
                          <div class="mt-0.5 truncate font-mono font-black text-content" :title="`${task.totalBytes || 0} B`">
                            {{ task.totalBytes ? formatFileSize(task.totalBytes) : '计算中' }}
                          </div>
                        </div>
                      </div>
                    </div>
                    <h4 class="task-log-heading text-muted text-xs font-black uppercase tracking-[0.2em] mb-3 flex items-center justify-between opacity-90">
                      <span class="flex items-center gap-2">
                        <i class="pi pi-align-left text-xs"></i>
                        {{ appStore.t('decompress.config.logs_title') }}
                      </span>
                    </h4>
                    <div data-testid="decompression-log-viewport" class="log-viewport flex-1 min-w-0 overflow-y-auto overflow-x-hidden pr-2 space-y-1.5 custom-scrollbar">
                      <div v-for="(log, idx) in task.logs" :key="idx" class="task-log-entry flex min-w-0 gap-3 items-start group/log border-l-2 border-subtle/20 pl-3 py-0.5">
                        <span class="task-log-time text-dim font-mono text-xs mt-0.5 opacity-80 shrink-0">{{ new Date(log.timestamp).toLocaleTimeString([], {hour12: false}) }}</span>
                        <div class="task-log-message min-w-0 flex-1 truncate whitespace-nowrap text-sm leading-relaxed font-mono" :class="getSeverityClass(log.severity)" :title="log.message">
                          {{ log.message }}
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </Transition>

        </div>
        </TransitionGroup>
      </div>
    </div>
  </div>

  <!-- 归档内容预览弹窗 -->
  <Modal :visible="showContentsModal" :title="contentsFile" icon="pi pi-list" @update:visible="showContentsModal = $event">
    <div v-if="contentsLoading" class="flex items-center justify-center py-8">
      <i class="pi pi-spin pi-spinner text-primary text-lg"></i>
    </div>
    <div v-else-if="contentsList.length === 0" class="text-center py-8 text-muted text-xs">
      <i class="pi pi-info-circle text-2xl mb-2 block opacity-75"></i>
      {{ appStore.t('tasks.contents.unable') }}
    </div>
    <div v-else class="space-y-1 max-h-64 overflow-y-auto custom-scrollbar">
      <div class="text-xs font-bold text-muted mb-2 uppercase tracking-widest">
        {{ contentsList.length }} {{ appStore.t('tasks.contents.files') }}
      </div>
      <div v-for="(item, idx) in contentsList" :key="idx"
           class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-input/30 text-sm font-mono text-content/80 hover:bg-primary/10 transition-colors">
        <i :class="item.endsWith('/') ? 'pi pi-folder text-yellow-400/70' : 'pi pi-file text-muted/50'" class="text-xs shrink-0"></i>
        <span class="truncate">{{ item }}</span>
      </div>
    </div>
  </Modal>

  <!-- 删除确认弹窗 -->
  <transition name="pop">
    <div v-if="taskToRemove" class="fixed inset-0 z-[200] flex items-center justify-center bg-black/50 backdrop-blur-sm" @click.self="taskToRemove = null">
      <div class="modal-no-glass rounded-2xl p-6 w-full max-w-xs text-center shadow-2xl text-content">
        <p class="text-xs font-black mb-4 uppercase tracking-widest">{{ appStore.t('tasks.confirm_delete') }}</p>
        <div class="flex gap-2">
          <button @click="taskToRemove = null" class="flex-1 py-2 rounded-xl bg-input text-muted text-xs font-bold border border-subtle">{{ appStore.t('vault.confirm.cancel') }}</button>
          <button @click="confirmRemoveTask" class="flex-1 py-2 rounded-xl bg-red-500 text-white text-xs font-black">{{ appStore.t('vault.confirm.delete_btn') }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.config-source-switch{display:flex;flex:0 0 auto;gap:.15rem;border:1px solid var(--border-subtle);border-radius:.65rem;background:var(--bg-input);padding:.15rem}.config-source-switch button{border-radius:.5rem;padding:.3rem .55rem;color:var(--text-muted);font-size:.65rem;font-weight:850;transition:all .18s ease}.config-source-switch button.active{background:var(--dynamic-accent);color:#fff;box-shadow:0 5px 14px -9px var(--dynamic-accent)}.config-follows-global>.inherited-config-control{pointer-events:none;opacity:.58}
.aero-table-container {
  /* 解决展开时滚动条出现导致的布局跳动 */
  min-width: 0;
  max-width: 100%;
  overflow-x: hidden;
  scrollbar-gutter: stable;
}

/* 任务离开动画 - 向左下角斜飞消失 */
.task-depart-leave-active {
  transition: all 0.4s cubic-bezier(0.55, 0, 1, 0.45);
  overflow: hidden;
}

.task-depart-leave-to {
  opacity: 0;
  transform: translateX(-40px) translateY(10px) scale(0.9);
  max-height: 0;
  padding-top: 0;
  padding-bottom: 0;
  margin-bottom: 0;
}

.task-depart-move {
  transition: all 0.35s cubic-bezier(0.4, 0, 0.2, 1);
}

.table-body {
  /* 确保主体区域也有稳定的间隙 */
  min-width: 0;
  max-width: 100%;
  overflow-x: hidden;
  scrollbar-gutter: stable;
}

.details-drawer {
  /* 增加更有深度的内阴影和背景色差，与主行区分 */
  background-color: transparent;
}

.task-detail-card {
  width: 100%;
  box-sizing: border-box;
  min-width: 0;
  max-width: 100%;
  background-image: linear-gradient(to bottom, rgba(var(--color-card-rgb), 0.92), rgba(var(--color-card-rgb), 0.98));
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  border: 1px dashed color-mix(in srgb, var(--dynamic-accent) 20%, transparent);
}

.task-detail-layout {
  display: grid;
  grid-template-columns: minmax(16rem, 0.82fr) minmax(19rem, 1.18fr);
  align-items: stretch;
  height: clamp(22rem, 54vh, 32rem);
  min-height: 0;
  overflow-x: hidden;
}

.task-config-panel,
.task-execution-panel,
.log-viewport {
  min-width: 0;
  max-width: 100%;
  overflow-x: hidden;
}

.task-config-panel,
.task-execution-panel {
  height: 100%;
  min-height: 0;
  max-height: none;
}

.task-config-panel > * {
  flex-shrink: 0;
}

.task-execution-panel {
  overflow: hidden;
}

.log-viewport {
  min-height: 0;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
}

.task-log-entry,
.task-log-message {
  max-width: 100%;
}

.archive-progress-fill {
  background-image: linear-gradient(
    100deg,
    var(--dynamic-accent) 0%,
    color-mix(in srgb, var(--dynamic-accent) 68%, white) 48%,
    var(--dynamic-accent) 100%
  );
  background-size: 220% 100%;
  animation: archive-progress-flow 1.35s linear infinite;
}

@keyframes archive-progress-flow {
  from { background-position: 100% 0; }
  to { background-position: -120% 0; }
}

.task-current-file {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-log-message {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.task-detail-card:hover {
  border-color: var(--dynamic-accent);
  border-style: solid;
  box-shadow: 
    0 25px 50px -12px rgba(0, 0, 0, 0.5),
    0 0 15px color-mix(in srgb, var(--dynamic-accent) 30%, transparent),
    inset 0 0 20px color-mix(in srgb, var(--dynamic-accent) 10%, transparent);
}

/* 虚线流动增强层 */
.task-detail-card::after {
  content: '';
  position: absolute;
  inset: 0;
  border: 1px dashed var(--dynamic-accent);
  border-radius: 1.1rem;
  opacity: 0.1;
  pointer-events: none;
  transition: all 0.3s ease;
}

.task-detail-card:hover::after {
  opacity: 0.6;
  inset: 0;
}

.aero-drawer-enter-active, .aero-drawer-leave-active {
  transition: height 0.35s cubic-bezier(0.4, 0, 0.2, 1), 
              opacity 0.25s linear,
              margin 0.35s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden; /* 动画期间必须裁剪 */
}
.aero-drawer-enter-from, .aero-drawer-leave-to {
  height: 0 !important;
  opacity: 0 !important;
  margin-top: 0 !important;
  margin-bottom: 0 !important;
}

.task-row {
  height: 52px;
  min-height: 52px;
}

.table-header,
.task-row {
  grid-template-columns: 2rem minmax(15rem, 1.15fr) minmax(20rem, 1fr) minmax(5.75rem, auto);
  column-gap: 0.75rem;
}

.task-action-cell {
  grid-column: 4;
  display: flex;
  min-width: 5.75rem;
  align-items: center;
  justify-content: flex-end;
  gap: 0.25rem;
  white-space: nowrap;
}

.task-row-action {
  display: inline-flex;
  width: 1.5rem;
  height: 1.5rem;
  flex: 0 0 1.5rem;
  align-items: center;
  justify-content: center;
  border-radius: 0.4rem;
  transition: color 180ms ease, background-color 180ms ease, transform 180ms ease;
}

.task-row-action:hover {
  transform: translateY(-1px);
}

.task-row-action:focus-visible {
  outline: 2px solid var(--dynamic-accent);
  outline-offset: 2px;
}

.task-action-divider {
  width: 1px;
  height: 1rem;
  flex: 0 0 1px;
  margin-inline: 0.125rem;
  background: var(--border-subtle);
  opacity: 0.75;
}

.task-password-cell {
  grid-column: 2 / 4;
  grid-row: 2;
  justify-self: end;
  max-width: 100%;
}

.task-row.has-password-input {
  height: auto;
  min-height: 52px;
  row-gap: 0.5rem;
}

.task-status-cell {
  grid-template-columns: minmax(6.5rem, auto) minmax(0, 1fr) auto;
}

.task-status-cell.is-terminal {
  grid-template-columns: minmax(0, 1fr);
}

.task-status-cell.is-terminal .task-status-primary {
  justify-self: center;
}

@media (max-width: 980px) {
  .task-detail-layout { grid-template-columns: minmax(16rem, .92fr) minmax(17rem, 1.08fr); }
  .table-header,
  .task-row { grid-template-columns: 2rem minmax(12rem, .9fr) minmax(17rem, 1.1fr) minmax(5.75rem, auto); }
}

.details-drawer {
  /* 移除静态 margin，由动画钩子精准控制 */
  background-color: transparent;
}

.pop-enter-active, .pop-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.pop-enter-from, .pop-leave-to { opacity: 0; transform: scale(0.95) translateY(10px); }

@media (max-width: 760px) {
  .task-config-panel {
    padding: 1rem;
  }

  .task-execution-panel {
    padding: 1rem;
  }

  .table-header,
  .task-row {
    column-gap: 0.5rem;
    padding-inline: 0.75rem;
  }

  .task-row {
    height: auto;
    min-height: 42px;
  }

  .details-drawer {
    padding-inline: 0.25rem;
  }
}

@media (max-width: 520px) {
  .task-config-panel {
    padding: 0.75rem;
  }

  .task-config-heading {
    gap: 0.375rem;
    letter-spacing: 0.08em;
  }

  .task-output-row {
    align-items: flex-start;
    flex-direction: column;
    gap: 0.5rem;
  }

  .task-output-label,
  .task-output-actions {
    width: 100%;
  }

  .task-output-actions {
    justify-content: flex-start;
  }

  .task-output-actions button {
    max-width: 100%;
    height: auto;
    min-height: 1.75rem;
    white-space: normal;
  }

  .task-subfolder-option {
    align-items: flex-start;
  }

  .task-log-heading {
    letter-spacing: 0.08em;
  }

  .log-viewport {
    padding-right: 0;
  }

  .task-log-entry {
    align-items: stretch;
    flex-direction: column;
    gap: 0.25rem;
    padding-left: 0.5rem;
  }

  .task-log-time,
  .task-log-message {
    max-width: 100%;
  }

  .table-header {
    padding-inline: 0.5rem;
    letter-spacing: 0.04em;
  }

  .task-row {
    grid-template-columns: 2rem minmax(7.5rem, 1fr) minmax(8rem, 1fr) minmax(5.75rem, auto);
    padding-inline: 0.5rem;
  }

  .task-status-cell {
    gap: 0.375rem;
  }

  .task-status-runtime,
  .task-status-percent {
    display: none;
  }
}
</style>
