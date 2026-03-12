<script setup lang="ts">
import { ref, computed } from 'vue'
import { useTaskStore, type Task } from '@/stores/task'
import { useAppStore } from '@/stores/app'

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
</script>

<template>
  <div class="aero-table-container w-full overflow-hidden">
    <!-- 顶部格式徽章栏 (动态高亮) -->
    <div class="format-badges flex gap-3 mb-8 px-2 flex-wrap">
      <div v-for="fmt in ['ZIP', '7Z', 'RAR', 'TAR', 'GZ', 'BZ2', 'XZ']" :key="fmt"
        class="px-5 py-2 rounded-2xl border backdrop-blur-xl transition-all duration-700 text-[10px] font-black tracking-widest uppercase cursor-default select-none"
        :class="taskStore.tasks.some(t => t.format?.toUpperCase() === fmt || (t.format === 'tar.gz' && fmt === 'GZ')) 
          ? 'bg-primary text-white border-primary shadow-[0_0_20px_color-mix(in_srgb,var(--dynamic-accent),transparent_70%)] scale-110 -translate-y-1' 
          : 'bg-card text-muted border-subtle hover:border-primary/50'"
      >
        {{ fmt }}
      </div>
    </div>

    <!-- 智慧表格 -->
    <div class="glass-table w-full aero-card overflow-hidden">
      <!-- 表头 -->
      <div class="table-header flex items-center px-10 py-6 border-b border-subtle bg-input/50 text-dim text-[9px] font-black tracking-[0.2em] uppercase">
        <div class="w-16">Icon</div>
        <div class="flex-1 min-w-[150px]">Archive Identification</div>
        <div class="w-48 hidden md:block">Source Path</div>
        <div class="w-24 text-center">Settings</div>
        <div class="flex-1 max-w-[280px]">Status & Core Lifecycle</div>
        <div class="w-10"></div>
      </div>

      <!-- 表格内容 -->
      <div class="table-body">
        <div v-for="task in taskStore.tasks" :key="task.id" class="task-row-container border-b border-subtle last:border-0 group/row">
          <div 
            class="task-row flex items-center px-10 py-7 hover:bg-primary/[0.03] transition-all duration-500 cursor-pointer relative overflow-hidden"
            @click="toggleExpand(task.id)"
          >
            <div class="absolute left-0 top-0 bottom-0 w-1 bg-primary scale-y-0 group-hover/row:scale-y-100 transition-transform duration-500"></div>

            <div class="w-16 flex items-center justify-start">
              <div :class="['w-11 h-11 flex items-center justify-center rounded-2xl border transition-all duration-700 group-hover/row:rotate-6 shadow-sm', getFormatBadgeColor(task.format)]">
                <i class="pi pi-file text-lg"></i>
              </div>
            </div>

            <div class="flex-1 min-w-[150px] px-6 overflow-hidden">
              <div class="text-content font-bold truncate text-sm tracking-tight group-hover/row:text-primary transition-colors">{{ task.name }}</div>
              <div class="flex items-center gap-2 mt-2">
                 <span class="text-dim text-[9px] uppercase font-black tracking-widest">
                   {{ (task.sourceFiles.length > 1 ? `Batch (${task.sourceFiles.length})` : 'Single') }}
                 </span>
                 <div class="w-1 h-1 rounded-full bg-subtle"></div>
                 <span class="text-dim text-[9px] font-mono">{{ task.format?.toUpperCase() }}</span>
              </div>
            </div>

            <div class="w-48 text-muted text-[10px] truncate italic px-2 hidden md:block font-mono font-light">
              {{ task.sourceFiles[0] }}
            </div>

            <div class="w-24 flex justify-center gap-4">
              <div class="w-8 h-8 rounded-full bg-input flex items-center justify-center hover:bg-primary/20 hover:text-primary transition-all text-dim border border-subtle">
                <i class="pi pi-key text-[10px]"></i>
              </div>
              <div class="w-8 h-8 rounded-full bg-input flex items-center justify-center hover:bg-green-500/20 hover:text-green-400 transition-all text-dim border border-subtle">
                <i class="pi pi-folder text-[10px]"></i>
              </div>
            </div>

            <div class="flex-1 max-w-[280px] px-6">
              <div class="flex justify-between items-end mb-3">
                <span class="text-[10px] text-muted font-bold truncate pr-6 tracking-wide">
                  {{ task.logs.length > 0 ? task.logs[task.logs.length - 1].message : 'Initializing Node...' }}
                </span>
                <span class="text-[10px] text-primary font-mono font-black">{{ task.progress }}%</span>
              </div>
              <div class="h-1.5 w-full bg-input border border-subtle rounded-full overflow-hidden shadow-inner p-[1px]">
                <div class="h-full bg-primary rounded-full transition-all duration-1000" 
                     :style="{ width: `${task.progress}%` }"></div>
              </div>
            </div>

            <div class="w-10 flex justify-end">
              <div :class="['w-6 h-6 rounded-full border flex items-center justify-center transition-all duration-700', 
                 expandedTasks.has(task.id) ? 'bg-primary border-primary shadow-lg' : 'bg-input border-subtle']">
                <i :class="['pi text-[8px] transition-all duration-700', 
                   expandedTasks.has(task.id) ? 'pi-chevron-up text-white' : 'pi-chevron-down text-muted']"></i>
              </div>
            </div>
          </div>

          <Transition name="aero-drawer">
            <div v-if="expandedTasks.has(task.id)" class="details-drawer relative overflow-hidden bg-input/30 border-t border-subtle">
              <div class="grid grid-cols-1 lg:grid-cols-5 gap-0">
                <div class="lg:col-span-2 p-10 border-r border-subtle">
                  <h4 class="text-muted text-[9px] font-black uppercase tracking-[0.3em] mb-8">Metadata Analysis</h4>
                  <div class="space-y-6">
                    <div class="group/meta">
                      <div class="text-dim text-[8px] uppercase font-black mb-2 tracking-widest group-hover/meta:text-primary transition-colors">Physical Hash (SHA256)</div>
                      <div class="p-4 rounded-2xl bg-card border border-subtle font-mono text-[10px] text-muted break-all leading-relaxed tracking-tighter">
                        {{ task.id.repeat(4).substring(0, 64) }}...
                      </div>
                    </div>
                    <div>
                      <div class="text-dim text-[8px] uppercase font-black mb-2 tracking-widest">Deployment Target</div>
                      <div class="text-[11px] text-muted font-mono italic">{{ task.outputPath }}</div>
                    </div>
                  </div>
                </div>

                <div class="lg:col-span-3 p-10 bg-card/40">
                  <h4 class="text-muted text-[9px] font-black uppercase tracking-[0.3em] mb-8 flex items-center justify-between">
                    Process IO Logs
                    <span class="w-2 h-2 rounded-full bg-primary animate-pulse"></span>
                  </h4>
                  <div class="log-viewport h-48 overflow-y-auto pr-6 space-y-3 custom-scrollbar">
                    <div v-for="(log, idx) in task.logs" :key="idx" class="flex gap-4 items-start group/log">
                      <span class="text-dim font-mono text-[9px] mt-1">{{ new Date(log.timestamp).toLocaleTimeString() }}</span>
                      <div class="flex-1 text-[11px] leading-relaxed transition-all group-hover/log:translate-x-1" :class="getSeverityClass(log.severity)">
                        {{ log.message }}
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
.aero-drawer-enter-active, .aero-drawer-leave-active {
  transition: all 0.6s cubic-bezier(0.34, 1.56, 0.64, 1);
  max-height: 800px;
}
.aero-drawer-enter-from, .aero-drawer-leave-to {
  max-height: 0;
  opacity: 0;
  transform: translateY(-20px);
}
</style>
