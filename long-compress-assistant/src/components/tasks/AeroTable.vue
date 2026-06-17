<script setup lang="ts">
import { ref, computed } from 'vue'
import { useTaskStore, type Task } from '@/stores/task'
import { useAppStore } from '@/stores/app'
import { usePasswordStore } from '@/stores/password'
import { useTauriCommands } from '@/composables/useTauriCommands'
import Modal from '@/components/ui/Modal.vue'
import { open } from '@tauri-apps/api/dialog'

const props = defineProps<{
  selectedTaskIds?: Set<string>
  statusFilter?: string | string[]
}>()

const emit = defineEmits<{
  'toggle-task': [taskId: string]
  'select-all-pending': []
  'deselect-all': []
}>()

const taskStore = useTaskStore()
const appStore = useAppStore()
const passwordStore = usePasswordStore()

const displayTasks = computed(() => {
  if (!props.statusFilter || props.statusFilter.length === 0) return taskStore.tasks
  const filters = Array.isArray(props.statusFilter) ? props.statusFilter : [props.statusFilter]
  return taskStore.tasks.filter(t => filters.includes(t.status))
})
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
  <div class="aero-table-container w-full h-full flex flex-col">
    <!-- 智慧表格 (重构为极简列表模式) -->
    <div class="glass-table w-full flex-1 flex flex-col min-h-0">
      <!-- 表头 (高度压缩，字体减小) -->
      <div class="table-header sticky top-0 z-20 flex items-center px-6 py-3 border-b border-subtle bg-input/90 backdrop-blur-xl text-dim text-[0.5rem] font-black tracking-[0.15em] uppercase shrink-0">
        <!-- 复选框列 -->
        <div class="w-8 shrink-0 flex items-center justify-center">
          <button
            v-if="taskStore.tasks.some(t => t.status === 'pending')"
            class="w-4 h-4 rounded border border-subtle/50 flex items-center justify-center hover:border-primary transition-colors"
            :class="taskStore.tasks.filter(t => t.status === 'pending').every(t => isSelected(t.id)) ? 'bg-primary border-primary' : 'bg-input/50'"
            @click.stop="taskStore.tasks.filter(t => t.status === 'pending').every(t => isSelected(t.id)) ? emit('deselect-all') : emit('select-all-pending')"
            :title="appStore.t('tasks.toggle_all')"
          >
            <i v-if="taskStore.tasks.filter(t => t.status === 'pending').every(t => isSelected(t.id))" class="pi pi-check text-[0.4375rem] text-white"></i>
          </button>
        </div>
        <div class="flex-[1.5] min-w-[180px]">{{ appStore.t('decompress.column.name') }}</div>
        <div class="w-60 hidden lg:block">{{ appStore.t('decompress.column.path') }}</div>
        <div class="flex-1 min-w-[180px]">{{ appStore.t('decompress.column.status') }}</div>
        <div class="w-10"></div>
      </div>

      <!-- 表格内容 (高密度布局 + 物理隔断) -->
      <div class="table-body flex-1 overflow-y-auto custom-scrollbar p-3">
        <TransitionGroup name="task-depart">
        <div v-for="task in displayTasks" :key="task.id" class="task-row-container mb-1.5 last:mb-0 group/row">
          <div 
            class="task-row flex items-center px-4 py-1.5 bg-card/30 border border-subtle/30 rounded-lg hover:border-primary/40 hover:bg-primary/[0.02] transition-all duration-300 cursor-pointer relative overflow-hidden shadow-sm"
            @click="toggleExpand(task.id)"
          >
            <!-- 状态指示条 (极细) -->
            <div class="absolute left-0 top-0 bottom-0 w-0.5 bg-primary opacity-0 group-hover/row:opacity-100 transition-opacity"></div>

            <!-- 复选框 -->
            <div class="w-8 shrink-0 flex items-center justify-center" @click.stop>
              <button
                v-if="task.status === 'pending'"
                class="w-4 h-4 rounded border flex items-center justify-center transition-all"
                :class="isSelected(task.id) ? 'bg-primary border-primary' : 'border-subtle/50 bg-input/50 hover:border-primary'"
                @click="emit('toggle-task', task.id)"
              >
                <i v-if="isSelected(task.id)" class="pi pi-check text-[0.4375rem] text-white"></i>
              </button>
              <i v-else-if="task.status === 'completed'" class="pi pi-check-circle text-green-400/50 text-[0.6875rem]"></i>
              <i v-else-if="task.status === 'failed'" class="pi pi-exclamation-circle text-red-400/50 text-[0.6875rem]"></i>
              <i v-else-if="task.status === 'cancelled'" class="pi pi-ban text-muted/30 text-[0.6875rem]"></i>
              <div v-else class="w-4 h-4"></div>
            </div>

            <!-- 文件识别区 (极致紧凑) -->
            <div class="flex-[1.5] min-w-[180px] overflow-hidden flex items-center gap-3">
              <div class="text-content font-bold truncate text-[0.6875rem] tracking-tight group-hover/row:text-primary transition-colors leading-tight">{{ task.name }}</div>
              <span class="text-dim text-[0.4375rem] uppercase font-black tracking-widest bg-input/50 px-1 py-0 rounded border border-subtle/20 shrink-0">
                {{ task.format?.toUpperCase() }}
              </span>
            </div>

            <!-- 物理路径 -->
            <div class="w-60 text-muted text-[0.5625rem] truncate italic px-4 hidden lg:block font-mono font-light opacity-30">
              {{ task.sourceFiles[0] }}
            </div>

            <!-- 状态与执行进度 (横向一行化) -->
            <div class="flex-1 min-w-[180px] flex items-center gap-4 px-4">
              <!-- 活动任务旋转图标 -->
              <i v-if="['preparing', 'running', 'extracting', 'compressing', 'finalizing'].includes(task.status)"
                 class="pi pi-spin pi-spinner text-primary text-[0.625rem] shrink-0"></i>
              <i v-else-if="task.status === 'completed'"
                 class="pi pi-check-circle text-green-400 text-[0.625rem] shrink-0"></i>
              <i v-else-if="task.status === 'failed'"
                 class="pi pi-exclamation-circle text-red-400 text-[0.625rem] shrink-0"></i>
              <i v-else-if="task.status === 'cancelled'"
                 class="pi pi-ban text-muted text-[0.625rem] shrink-0"></i>
              <span class="text-[0.5625rem] font-bold truncate flex-1"
                    :class="{
                      'text-primary': ['preparing', 'running', 'extracting', 'compressing', 'finalizing'].includes(task.status),
                      'text-muted opacity-60': task.status === 'pending',
                      'text-green-400': task.status === 'completed',
                      'text-red-400': task.status === 'failed',
                      'text-muted line-through': task.status === 'cancelled'
                    }">
                {{ task.status === 'pending'
                  ? appStore.t('decompress.waiting')
                  : task.status === 'preparing' ? appStore.t('tasks.status.preparing_short')
                  : task.status === 'completed' ? appStore.t('tasks.status.completed_short')
                  : task.status === 'failed' ? appStore.t('tasks.status.failed_short')
                  : task.status === 'cancelled' ? appStore.t('tasks.status.cancelled_short')
                  : (task.logs.length > 0 ? task.logs[task.logs.length - 1].message : appStore.t('tasks.status.processing_short')) }}
              </span>
              <div class="w-24 flex items-center gap-2 shrink-0">
                <div class="h-0.5 flex-1 bg-input border border-subtle/20 rounded-full overflow-hidden">
                  <div class="h-full rounded-full transition-all duration-1000"
                       :class="task.status === 'failed' ? 'bg-red-400' : task.status === 'completed' ? 'bg-green-400' : 'bg-primary'"
                       :style="{ width: `${task.progress}%` }"></div>
                </div>
                <span class="text-[0.5625rem] font-mono font-black w-6 text-right"
                      :class="task.status === 'failed' ? 'text-red-400' : task.status === 'completed' ? 'text-green-400' : 'text-primary'">
                  {{ task.progress }}%
                </span>
              </div>
            </div>

            <!-- 密码内联输入 (自动破解失败时在行内显示) -->
            <div v-if="task.passwordRequired" class="flex items-center gap-1 shrink-0 px-2" @click.stop>
              <input
                :type="showPasswordInput === task.id ? 'text' : 'password'"
                :value="task.password || ''"
                @input="(e: Event) => { task.password = (e.target as HTMLInputElement).value; task.passwordRequired = false }"
                :placeholder="appStore.t('tasks.password.placeholder')"
                class="h-7 w-36 rounded-lg bg-yellow-500/5 border border-yellow-500/50 text-[0.5625rem] px-2 font-mono outline-none focus:border-yellow-400 text-yellow-400 placeholder:text-yellow-500/50"
              />
              <button @click.stop="() => { const candidates = passwordStore.findCandidatePasswords(task.name || task.sourceFiles[0]?.split(/[\\/]/).pop() || ''); if (candidates.length > 0) { task.password = candidates[0]; task.passwordRequired = false } }"
                class="h-7 w-7 rounded-lg border border-yellow-500/50 bg-yellow-500/10 flex items-center justify-center text-yellow-400 hover:bg-yellow-500/20 transition-colors shrink-0"
                :title="appStore.t('tasks.password.fill_vault')">
                <i class="pi pi-key text-[0.5rem]"></i>
              </button>
            </div>

            <!-- 删除按钮 (仅 pending 状态可见) -->
            <div class="w-6 flex justify-end" @click.stop>
              <button
                v-if="task.status === 'pending'"
                @click="handleRemoveTask(task.id)"
                class="w-5 h-5 rounded-md flex items-center justify-center text-dim hover:text-red-400 hover:bg-red-500/10 transition-all"
                :title="appStore.t('tasks.remove')">
                <i class="pi pi-times text-[0.5625rem]"></i>
              </button>
            </div>

            <div class="w-6 flex justify-end">
              <i :class="['pi text-[0.4375rem] transition-all duration-500',
                 expandedTasks.has(task.id) ? 'pi-chevron-up text-primary' : 'pi-chevron-down text-muted']"></i>
            </div>
          </div>

          <Transition 
            name="aero-drawer"
            @before-enter="onBeforeEnter"
            @enter="onEnter"
            @before-leave="onBeforeLeave"
            @leave="onLeave"
          >
            <div v-if="expandedTasks.has(task.id)" class="details-drawer relative px-6 pb-6 pt-2">
              <!-- 交互增强：task-detail-card 增加 hover 动效 -->
              <div class="task-detail-card flex rounded-2xl bg-card border border-dashed border-primary/30 shadow-2xl overflow-hidden relative group/detail">

                <!-- 详情区内容布局：改为弹性分配，防止溢出 -->
                <div class="flex w-full relative z-10">
                  <!-- 左侧：核心配置 -->
                  <div class="flex-initial min-w-[320px] max-w-[45%] p-5 border-r border-subtle/20 flex flex-col space-y-3 pl-8 transition-colors group-hover/detail:bg-primary/[0.01] overflow-y-auto custom-scrollbar max-h-56">
                    <div class="flex items-center justify-between">
                      <h4 class="text-primary text-[0.5625rem] font-black uppercase tracking-[0.2em] flex items-center gap-2">
                        <i class="pi pi-cog text-[0.625rem]"></i>
                        {{ appStore.t('decompress.column.config') }}
                      </h4>
                    </div>

                    <div class="space-y-3.5">
                      <!-- 路径行：增加 flex-wrap 兜底，但在大多数状态下保持并排 -->
                      <div class="space-y-2">
                        <div class="flex items-center justify-between gap-3">
                          <span class="text-muted text-[0.5rem] uppercase font-black tracking-widest opacity-60 shrink-0">{{ appStore.t('decompress.config.output') }}</span>
                          <div class="flex gap-1.5 flex-nowrap shrink-0 overflow-x-auto no-scrollbar">
                            <button @click.stop="handleSetSameDir(task)" 
                                    class="h-6 px-2.5 rounded-md bg-primary/10 text-primary hover:bg-primary hover:text-white transition-all text-[0.5625rem] font-black whitespace-nowrap">
                              {{ appStore.t('decompress.config.output_same') }}
                            </button>
                            <button @click.stop="handleSelectOutputDir(task)"
                                    class="h-7 px-3 rounded-lg bg-primary text-white hover:brightness-110 active:scale-95 transition-all text-[0.5625rem] font-black whitespace-nowrap flex items-center gap-1.5 shadow-sm">
                              <i class="pi pi-folder-open text-[0.5625rem]"></i>
                              {{ appStore.t('decompress.config.output_select') }}
                            </button>
                          </div>
                        </div>
                        <div class="px-3 py-2 rounded-xl bg-input/50 border border-subtle/50 font-mono text-[0.625rem] text-content/80 truncate shadow-inner">
                          {{ task.outputPath || appStore.t('decompress.config.output_auto') }}
                        </div>
                      </div>

                      <div class="flex items-center gap-3 cursor-pointer group/check" @click.stop="task.extractToSubfolder = !task.extractToSubfolder">
                        <div class="w-4 h-4 rounded border border-subtle flex items-center justify-center transition-all group-hover/check:border-primary"
                             :class="task.extractToSubfolder ? 'bg-primary border-primary' : 'bg-input'">
                          <i v-if="task.extractToSubfolder" class="pi pi-check text-[0.5rem] text-white"></i>
                        </div>
                        <span class="text-[0.6875rem] font-bold text-muted group-hover/check:text-content transition-colors uppercase tracking-tight">{{ appStore.t('decompress.config.output_sub') }}</span>
                      </div>

                      <!-- 密码输入区 (仅在自动破解失败时显示) -->
                      <div v-if="task.passwordRequired" class="space-y-1.5 p-3 rounded-xl border border-yellow-500/30 bg-yellow-500/5">
                        <div class="flex items-center gap-2">
                          <i class="pi pi-lock text-[0.625rem] text-yellow-400"></i>
                          <span class="text-yellow-400 text-[0.5625rem] font-black uppercase tracking-widest">{{ appStore.t('tasks.password.crack_failed') }}</span>
                        </div>
                        <div class="flex items-center gap-2">
                          <input
                            :type="showPasswordInput === task.id ? 'text' : 'password'"
                            :value="task.password || ''"
                            @input="(e: Event) => { task.password = (e.target as HTMLInputElement).value; task.passwordRequired = false; }"
                            @click.stop
                            :placeholder="appStore.t('tasks.password.placeholder')"
                            class="flex-1 h-7 rounded-lg bg-input/50 border border-yellow-500/50 text-[0.625rem] px-3 font-mono outline-none focus:border-yellow-400 text-yellow-400 placeholder:text-yellow-500/50"
                          />
                          <button
                            @click.stop="() => { const candidates = passwordStore.findCandidatePasswords(task.name || task.sourceFiles[0]?.split(/[\\/]/).pop() || ''); if (candidates.length > 0) { task.password = candidates[0]; task.passwordRequired = false; } }"
                            class="h-7 w-7 rounded-lg border border-yellow-500/50 bg-yellow-500/10 flex items-center justify-center text-yellow-400 hover:bg-yellow-500/20 transition-colors shrink-0"
                            :title="appStore.t('tasks.password.fill_vault')">
                            <i class="pi pi-key text-[0.5625rem]"></i>
                          </button>
                        </div>
                      </div>

                    </div>
                  </div>

                  <!-- 右侧：执行日志 -->
                  <div class="flex-1 p-5 flex flex-col overflow-hidden">
                    <h4 class="text-muted text-[0.5rem] font-black uppercase tracking-[0.2em] mb-3 flex items-center justify-between opacity-60">
                      <span class="flex items-center gap-2">
                        <i class="pi pi-align-left text-[0.5625rem]"></i>
                        {{ appStore.t('decompress.config.logs_title') }}
                      </span>
                    </h4>
                    <div class="log-viewport flex-1 overflow-y-auto pr-2 space-y-1.5 custom-scrollbar">
                      <div v-for="(log, idx) in task.logs" :key="idx" class="flex gap-3 items-start group/log border-l-2 border-subtle/20 pl-3 py-0.5">
                        <span class="text-dim font-mono text-[0.5rem] mt-0.5 opacity-40 shrink-0">{{ new Date(log.timestamp).toLocaleTimeString([], {hour12: false}) }}</span>
                        <div class="flex-1 text-[0.625rem] leading-relaxed font-mono" :class="getSeverityClass(log.severity)">
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
      <i class="pi pi-info-circle text-2xl mb-2 block opacity-30"></i>
      {{ appStore.t('tasks.contents.unable') }}
    </div>
    <div v-else class="space-y-1 max-h-64 overflow-y-auto custom-scrollbar">
      <div class="text-[0.5625rem] font-bold text-muted mb-2 uppercase tracking-widest">
        {{ contentsList.length }} {{ appStore.t('tasks.contents.files') }}
      </div>
      <div v-for="(item, idx) in contentsList" :key="idx"
           class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-input/30 text-[0.625rem] font-mono text-content/80 hover:bg-primary/10 transition-colors">
        <i :class="item.endsWith('/') ? 'pi pi-folder text-yellow-400/70' : 'pi pi-file text-muted/50'" class="text-[0.5625rem] shrink-0"></i>
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
          <button @click="taskToRemove = null" class="flex-1 py-2 rounded-xl bg-input text-muted text-[0.5625rem] font-bold border border-subtle">{{ appStore.t('vault.confirm.cancel') }}</button>
          <button @click="confirmRemoveTask" class="flex-1 py-2 rounded-xl bg-red-500 text-white text-[0.5625rem] font-black">{{ appStore.t('vault.confirm.delete_btn') }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.aero-table-container {
  /* 解决展开时滚动条出现导致的布局跳动 */
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
  scrollbar-gutter: stable;
}

.details-drawer {
  /* 增加更有深度的内阴影和背景色差，与主行区分 */
  background-color: transparent;
}

.task-detail-card {
  animation: border-flow 20s linear infinite;
  background-image: linear-gradient(to bottom, rgba(var(--color-card-rgb), 0.92), rgba(var(--color-card-rgb), 0.98));
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  border: 1px dashed color-mix(in srgb, var(--dynamic-accent) 20%, transparent);
}

.task-detail-card:hover {
  /* 彻底移除位移和缩放，保持物理位置绝对不动 */
  border: 2px dashed var(--dynamic-accent);
  border-style: solid; /* 悬浮时变为实线，提供强视觉反馈 */
  box-shadow: 
    0 25px 50px -12px rgba(0, 0, 0, 0.5),
    0 0 15px color-mix(in srgb, var(--dynamic-accent) 30%, transparent),
    inset 0 0 20px color-mix(in srgb, var(--dynamic-accent) 10%, transparent);
}

@keyframes border-flow {
  from { border-color: color-mix(in srgb, var(--dynamic-accent) 20%, transparent); }
  50% { border-color: color-mix(in srgb, var(--dynamic-accent) 40%, transparent); }
  to { border-color: color-mix(in srgb, var(--dynamic-accent) 20%, transparent); }
}

/* 虚线流动增强层 */
.task-detail-card::after {
  content: '';
  position: absolute;
  inset: -2px; /* 稍微扩大一点，确保加粗时不被遮挡 */
  border: 2px dashed var(--dynamic-accent);
  border-radius: 1.1rem;
  opacity: 0.1;
  pointer-events: none;
  animation: dash-slide 40s linear infinite;
  transition: all 0.3s ease;
}

.task-detail-card:hover::after {
  opacity: 0.6;
  inset: -1px;
  animation-duration: 10s; /* Hover 时动画加速 */
}

@keyframes dash-slide {
  to { stroke-dashoffset: -1000; }
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
  /* 严格控制磁贴高度 */
  height: 38px;
  min-height: 38px;
}

.details-drawer {
  /* 移除静态 margin，由动画钩子精准控制 */
  background-color: transparent;
}

.pop-enter-active, .pop-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.pop-enter-from, .pop-leave-to { opacity: 0; transform: scale(0.95) translateY(10px); }
</style>
