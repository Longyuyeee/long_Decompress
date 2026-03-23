<script setup lang="ts">
import { ref, computed } from 'vue'
import { useTaskStore, type Task } from '@/stores/task'
import { useAppStore } from '@/stores/app'
import { open } from '@tauri-apps/api/dialog'

const taskStore = useTaskStore()
const appStore = useAppStore()
const expandedTasks = ref<Set<string>>(new Set())

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
    console.error('Failed to open directory dialog:', err)
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
  <div class="aero-table-container w-full h-full flex flex-col overflow-hidden">
    <!-- 智慧表格 (重构为极简列表模式) -->
    <div class="glass-table w-full flex-1 flex flex-col overflow-hidden">
      <!-- 表头 (高度压缩，字体减小) -->
      <div class="table-header sticky top-0 z-20 flex items-center px-6 py-3 border-b border-subtle bg-input/90 backdrop-blur-xl text-dim text-[8px] font-black tracking-[0.15em] uppercase shrink-0">
        <div class="flex-[1.5] min-w-[180px]">{{ appStore.t('decompress.column.name') }}</div>
        <div class="w-60 hidden lg:block">{{ appStore.t('decompress.column.path') }}</div>
        <div class="flex-1 min-w-[180px]">{{ appStore.t('decompress.column.status') }}</div>
        <div class="w-10"></div>
      </div>

      <!-- 表格内容 (高密度布局 + 物理隔断) -->
      <div class="table-body flex-1 overflow-y-auto custom-scrollbar p-3">
        <div v-for="task in taskStore.tasks" :key="task.id" class="task-row-container mb-1.5 last:mb-0 group/row">
          <div 
            class="task-row flex items-center px-4 py-1.5 bg-card/30 border border-subtle/30 rounded-lg hover:border-primary/40 hover:bg-primary/[0.02] transition-all duration-300 cursor-pointer relative overflow-hidden shadow-sm"
            @click="toggleExpand(task.id)"
          >
            <!-- 状态指示条 (极细) -->
            <div class="absolute left-0 top-0 bottom-0 w-0.5 bg-primary opacity-0 group-hover/row:opacity-100 transition-opacity"></div>

            <!-- 文件识别区 (极致紧凑) -->
            <div class="flex-[1.5] min-w-[180px] overflow-hidden flex items-center gap-3">
              <div class="text-content font-bold truncate text-[11px] tracking-tight group-hover/row:text-primary transition-colors leading-tight">{{ task.name }}</div>
              <span class="text-dim text-[7px] uppercase font-black tracking-widest bg-input/50 px-1 py-0 rounded border border-subtle/20 shrink-0">
                {{ task.format?.toUpperCase() }}
              </span>
            </div>

            <!-- 物理路径 -->
            <div class="w-60 text-muted text-[9px] truncate italic px-4 hidden lg:block font-mono font-light opacity-30">
              {{ task.sourceFiles[0] }}
            </div>

            <!-- 状态与执行进度 (横向一行化) -->
            <div class="flex-1 min-w-[180px] flex items-center gap-4 px-4">
              <span class="text-[9px] text-muted font-bold truncate flex-1 opacity-60">
                {{ task.status === 'pending' 
                  ? appStore.t('decompress.waiting')
                  : (task.logs.length > 0 ? task.logs[task.logs.length - 1].message : '...') }}
              </span>
              <div class="w-24 flex items-center gap-2 shrink-0">
                <div class="h-0.5 flex-1 bg-input border border-subtle/20 rounded-full overflow-hidden">
                  <div class="h-full bg-primary rounded-full transition-all duration-1000" 
                       :style="{ width: `${task.progress}%` }"></div>
                </div>
                <span class="text-[9px] text-primary font-mono font-black w-6 text-right">{{ task.progress }}%</span>
              </div>
            </div>

            <div class="w-6 flex justify-end">
              <i :class="['pi text-[7px] transition-all duration-500', 
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
              <div class="task-detail-card flex h-44 rounded-2xl bg-card border border-dashed border-primary/30 shadow-2xl overflow-hidden relative group/detail">

                <!-- 详情区内容布局：改为弹性分配，防止溢出 -->
                <div class="flex w-full h-full relative z-10">
                  <!-- 左侧：核心配置 (设置最小宽度和弹性边界) -->
                  <div class="flex-initial min-w-[320px] max-w-[45%] p-5 border-r border-subtle/20 flex flex-col justify-center space-y-4 pl-8 transition-colors group-hover/detail:bg-primary/[0.01]">
                    <div class="flex items-center justify-between">
                      <h4 class="text-primary text-[9px] font-black uppercase tracking-[0.2em] flex items-center gap-2">
                        <i class="pi pi-cog text-[10px]"></i>
                        {{ appStore.t('decompress.column.config') }}
                      </h4>
                    </div>

                    <div class="space-y-3.5">
                      <!-- 路径行：增加 flex-wrap 兜底，但在大多数状态下保持并排 -->
                      <div class="space-y-2">
                        <div class="flex items-center justify-between gap-3">
                          <span class="text-muted text-[8px] uppercase font-black tracking-widest opacity-60 shrink-0">{{ appStore.t('decompress.config.output') }}</span>
                          <div class="flex gap-1.5 flex-nowrap shrink-0 overflow-x-auto no-scrollbar">
                            <button @click.stop="handleSetSameDir(task)" 
                                    class="h-6 px-2.5 rounded-md bg-primary/10 text-primary hover:bg-primary hover:text-white transition-all text-[9px] font-black whitespace-nowrap">
                              {{ appStore.t('decompress.config.output_same') }}
                            </button>
                            <button @click.stop="handleSelectOutputDir(task)" 
                                    class="h-6 px-2.5 rounded-md bg-input border border-subtle text-muted hover:text-content transition-all text-[9px] font-black whitespace-nowrap flex items-center gap-1.5">
                              <i class="pi pi-external-link text-[8px]"></i>
                              {{ appStore.t('decompress.config.output_select') }}
                            </button>
                          </div>
                        </div>
                        <div class="px-3 py-2 rounded-xl bg-input/50 border border-subtle/50 font-mono text-[10px] text-content/80 truncate shadow-inner">
                          {{ task.outputPath || appStore.t('decompress.config.output_auto') }}
                        </div>
                      </div>

                      <div class="flex items-center gap-3 cursor-pointer group/check" @click.stop="task.extractToSubfolder = !task.extractToSubfolder">
                        <div class="w-4 h-4 rounded border border-subtle flex items-center justify-center transition-all group-hover/check:border-primary" 
                             :class="task.extractToSubfolder ? 'bg-primary border-primary' : 'bg-input'">
                          <i v-if="task.extractToSubfolder" class="pi pi-check text-[8px] text-white"></i>
                        </div>
                        <span class="text-[11px] font-bold text-muted group-hover/check:text-content transition-colors uppercase tracking-tight">{{ appStore.t('decompress.config.output_sub') }}</span>
                      </div>
                    </div>
                  </div>

                  <!-- 右侧：执行日志 -->
                  <div class="flex-1 p-5 flex flex-col overflow-hidden">
                    <h4 class="text-muted text-[8px] font-black uppercase tracking-[0.2em] mb-3 flex items-center justify-between opacity-60">
                      <span class="flex items-center gap-2">
                        <i class="pi pi-align-left text-[9px]"></i>
                        {{ appStore.t('decompress.config.logs_title') }}
                      </span>
                    </h4>
                    <div class="log-viewport flex-1 overflow-y-auto pr-2 space-y-1.5 custom-scrollbar">
                      <div v-for="(log, idx) in task.logs" :key="idx" class="flex gap-3 items-start group/log border-l-2 border-subtle/20 pl-3 py-0.5">
                        <span class="text-dim font-mono text-[8px] mt-0.5 opacity-40 shrink-0">{{ new Date(log.timestamp).toLocaleTimeString([], {hour12: false}) }}</span>
                        <div class="flex-1 text-[10px] leading-relaxed font-mono" :class="getSeverityClass(log.severity)">
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
      </div>
    </div>
  </div>
</template>

<style scoped>
.aero-table-container {
  /* 解决展开时滚动条出现导致的布局跳动 */
  scrollbar-gutter: stable;
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
</style>
