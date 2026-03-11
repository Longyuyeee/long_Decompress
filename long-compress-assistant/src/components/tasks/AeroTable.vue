<script setup lang="ts">
import { ref, computed } from 'vue'
import { useTaskStore, type Task } from '@/stores/task'
import GlassCard from '@/components/ui/GlassCard.vue'
import ProgressBar from '@/components/ui/ProgressBar.vue'

const taskStore = useTaskStore()
const expandedTasks = ref<Set<string>>(new Set())

const toggleExpand = (taskId: string) => {
  if (expandedTasks.value.has(taskId)) {
    expandedTasks.value.delete(taskId)
  } else {
    expandedTasks.value.add(taskId)
  }
}

const getFormatBadgeColor = (format?: string) => {
  switch (format?.toLowerCase()) {
    case 'zip': return 'bg-blue-500/20 text-blue-400 border-blue-500/30'
    case '7z': return 'bg-purple-500/20 text-purple-400 border-purple-500/30'
    case 'rar': return 'bg-red-500/20 text-red-400 border-red-500/30'
    default: return 'bg-gray-500/20 text-gray-400 border-gray-500/30'
  }
}

const getSeverityClass = (severity: string) => {
  switch (severity) {
    case 'error': return 'text-red-400'
    case 'warning': return 'text-yellow-400'
    case 'success': return 'text-green-400'
    default: return 'text-blue-300'
  }
}
</script>

<template>
  <div class="aero-table-container w-full overflow-hidden">
    <!-- 顶部格式徽章栏 -->
    <div class="format-badges flex gap-3 mb-6 px-2 flex-wrap">
      <div v-for="fmt in ['ZIP', '7Z', 'RAR', 'TAR', 'GZ', 'BZ2', 'XZ']" :key="fmt"
        class="px-4 py-1.5 rounded-full border backdrop-blur-md transition-all duration-500 text-[10px] font-black tracking-widest"
        :class="taskStore.tasks.some(t => t.format?.toUpperCase() === fmt || (t.format === 'tar.gz' && fmt === 'GZ')) 
          ? 'bg-blue-500/30 text-white border-blue-400/50 shadow-[0_0_15px_rgba(59,130,246,0.4)] scale-110' 
          : 'bg-white/5 text-white/40 border-white/10 hover:border-white/20'"
      >
        {{ fmt }}
      </div>
    </div>

    <!-- 智慧表格 -->
    <div class="glass-table w-full border border-white/10 rounded-2xl overflow-hidden backdrop-blur-xl bg-white/5">
      <!-- 表头 -->
      <div class="table-header flex items-center px-6 py-4 border-b border-white/10 text-white/50 text-xs font-bold tracking-widest uppercase">
        <div class="w-12">图标</div>
        <div class="flex-1 min-w-[120px]">文件名</div>
        <div class="w-48 hidden md:block">源路径</div>
        <div class="w-20 text-center">配置</div>
        <div class="flex-1 max-w-[240px]">状态与进度</div>
        <div class="w-10"></div>
      </div>

      <!-- 表格行 -->
      <div class="table-body">
        <div v-for="task in taskStore.tasks" :key="task.id" class="task-row-container border-b border-white/5 last:border-0">
          <div 
            class="task-row flex items-center px-6 py-5 hover:bg-white/5 transition-all duration-300 group cursor-pointer"
            @click="toggleExpand(task.id)"
          >
            <!-- 图标 -->
            <div class="w-12 flex items-center justify-center">
              <div :class="['p-2 rounded-lg border transition-all duration-500', getFormatBadgeColor(task.format)]">
                <i class="pi pi-file-o text-lg"></i>
              </div>
            </div>

            <!-- 文件名 -->
            <div class="flex-1 min-w-[120px] px-4 overflow-hidden">
              <div class="text-white font-medium truncate text-sm">{{ task.name }}</div>
              <div class="text-white/40 text-[10px] mt-1 uppercase tracking-tighter">
                {{ (task.sourceFiles.length > 1 ? `Batch (${task.sourceFiles.length})` : 'Single') }}
              </div>
            </div>

            <!-- 源路径 (窄屏隐藏) -->
            <div class="w-48 text-white/20 text-[10px] truncate italic px-2 hidden md:block font-mono">
              {{ task.sourceFiles[0] }}
            </div>

            <!-- 局部配置图标 -->
            <div class="w-20 flex justify-center gap-3">
              <i class="pi pi-key text-white/30 hover:text-blue-400 transition-colors cursor-pointer text-xs"></i>
              <i class="pi pi-folder text-white/30 hover:text-green-400 transition-colors cursor-pointer text-xs"></i>
            </div>

            <!-- 状态与进度 -->
            <div class="flex-1 max-w-[240px] relative px-4">
              <div class="flex justify-between items-end mb-1.5">
                <span class="text-[9px] text-white/50 font-bold truncate pr-4">
                  {{ task.logs.length > 0 ? task.logs[task.logs.length - 1].message : 'Pending' }}
                </span>
                <span class="text-[9px] text-blue-400 font-mono">{{ task.progress }}%</span>
              </div>
              <div class="h-1 w-full bg-white/5 rounded-full overflow-hidden">
                <div class="h-full bg-blue-500 transition-all duration-500" 
                     :style="{ width: `${task.progress}%` }"></div>
              </div>
            </div>

            <!-- 更多 -->
            <div class="w-16 flex justify-end">
              <i :class="['pi text-white/20 transition-transform duration-500', 
                 expandedTasks.has(task.id) ? 'pi-chevron-down rotate-180' : 'pi-chevron-down']"></i>
            </div>
          </div>

          <!-- 展开详情卡片 -->
          <Transition name="expand">
            <div v-if="expandedTasks.has(task.id)" class="details-card px-6 py-6 bg-black/20 border-t border-white/5">
              <div class="grid grid-cols-2 gap-8">
                <!-- 元数据 -->
                <div class="metadata-section">
                  <h4 class="text-white/60 text-[10px] font-bold uppercase tracking-widest mb-4">文件指纹与元数据</h4>
                  <div class="space-y-3">
                    <div class="flex justify-between items-center text-xs">
                      <span class="text-white/30">MD5</span>
                      <span class="text-white/70 font-mono text-[10px]">Calculating...</span>
                    </div>
                    <div class="flex justify-between items-center text-xs">
                      <span class="text-white/30">SHA256</span>
                      <span class="text-white/70 font-mono text-[10px]">Pending...</span>
                    </div>
                    <div class="flex justify-between items-center text-xs">
                      <span class="text-white/30">目标路径</span>
                      <span class="text-white/70 truncate ml-4">{{ task.outputPath }}</span>
                    </div>
                  </div>
                </div>

                <!-- IO 日志流 -->
                <div class="logs-section">
                  <h4 class="text-white/60 text-[10px] font-bold uppercase tracking-widest mb-4">底层 IO 日志流</h4>
                  <div class="log-stream h-32 overflow-y-auto pr-2 space-y-1.5 custom-scrollbar">
                    <div v-for="(log, idx) in task.logs" :key="idx" class="text-[10px] font-mono leading-relaxed">
                      <span class="text-white/20 mr-2">{{ new Date(log.timestamp).toLocaleTimeString() }}</span>
                      <span :class="getSeverityClass(log.severity)">{{ log.message }}</span>
                    </div>
                    <div v-if="task.logs.length === 0" class="text-white/10 italic text-[10px]">等待操作开始...</div>
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
.expand-enter-active,
.expand-leave-active {
  transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1);
  max-height: 400px;
  opacity: 1;
}

.expand-enter-from,
.expand-leave-to {
  max-height: 0;
  opacity: 0;
  padding-top: 0;
  padding-bottom: 0;
  overflow: hidden;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 3px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: rgba(255, 255, 255, 0.05);
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.2);
}
</style>
