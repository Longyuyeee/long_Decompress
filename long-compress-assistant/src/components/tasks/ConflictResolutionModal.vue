<script setup lang="ts">
import { computed } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import { useTaskStore, type ConflictInfo } from '@/stores/task'
import { useAppStore } from '@/stores/app'

const props = defineProps<{
  taskId: string
  visible: boolean
}>()

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void
  (e: 'resolve', action: 'overwrite' | 'skip' | 'rename', applyToAll: boolean): void
}>()

const taskStore = useTaskStore()
const appStore = useAppStore()
const task = computed(() => taskStore.tasks.find(t => t.id === props.taskId))
const currentConflict = computed(() => task.value?.conflicts[0])

const formatSize = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const formatDate = (timestamp: number) => {
  return new Date(timestamp).toLocaleString()
}

const resolve = (action: 'overwrite' | 'skip' | 'rename', applyToAll: boolean = false) => {
  emit('resolve', action, applyToAll)
  if (task.value) {
    if (applyToAll) task.value.conflicts = []
    else task.value.conflicts.shift()
    if (task.value.conflicts.length === 0) emit('update:visible', false)
  }
}
</script>

<template>
  <Modal
    :visible="visible"
    @update:visible="val => emit('update:visible', val)"
    :title="appStore.t('common.conflict_title') || '文件冲突检测'"
    icon="pi pi-exclamation-triangle"
    size="lg"
  >
    <div v-if="currentConflict" class="conflict-container space-y-8 p-2">
      <!-- 头部提示：强化路径显示 -->
      <div class="header-section bg-input/30 p-4 rounded-2xl border border-subtle/30 backdrop-blur-md">
        <p class="text-muted text-[10px] font-black uppercase tracking-widest mb-2 opacity-60">Target Path Conflict</p>
        <div class="text-content font-bold font-mono text-xs break-all leading-relaxed">
          {{ currentConflict.destPath }}
        </div>
      </div>

      <!-- 核心对比区 -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6 relative">
        <!-- 装饰性箭头 -->
        <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 z-20 hidden md:block">
          <div class="w-10 h-10 rounded-full bg-base border border-subtle/50 flex items-center justify-center shadow-xl">
            <i class="pi pi-arrow-right text-primary animate-pulse"></i>
          </div>
        </div>

        <!-- 现有文件 (Existing) -->
        <div class="compare-card existing p-6 rounded-[2rem] bg-input/40 border border-subtle/50 relative overflow-hidden group transition-all duration-500 hover:border-red-500/30">
          <div class="absolute -right-6 -top-6 w-24 h-24 bg-red-500/5 rounded-full blur-2xl"></div>
          <div class="flex items-center gap-3 mb-6">
            <div class="w-8 h-8 rounded-xl bg-red-500/10 flex items-center justify-center text-red-400">
              <i class="pi pi-file"></i>
            </div>
            <span class="text-[10px] font-black text-muted uppercase tracking-widest">现有文件</span>
          </div>
          <div class="space-y-4">
            <div class="flex flex-col">
              <span class="text-dim text-[9px] uppercase font-bold mb-1">物理体积</span>
              <span class="text-content font-mono font-black text-lg">{{ formatSize(currentConflict.destSize) }}</span>
            </div>
            <div class="flex flex-col">
              <span class="text-dim text-[9px] uppercase font-bold mb-1">最后修改</span>
              <span class="text-muted font-mono text-[10px]">{{ formatDate(currentConflict.destModified) }}</span>
            </div>
          </div>
        </div>

        <!-- 新入数据 (Incoming) -->
        <div class="compare-card incoming p-6 rounded-[2rem] bg-primary/5 border border-primary/20 relative overflow-hidden group transition-all duration-500 hover:border-primary/50 shadow-[inset_0_0_40px_rgba(var(--primary-rgb),0.02)]">
          <div class="absolute -right-6 -top-6 w-24 h-24 bg-primary/10 rounded-full blur-2xl"></div>
          <div class="flex items-center gap-3 mb-6">
            <div class="w-8 h-8 rounded-xl bg-primary/20 flex items-center justify-center text-primary">
              <i class="pi pi-download animate-bounce"></i>
            </div>
            <span class="text-[10px] font-black text-primary uppercase tracking-widest">即将解压</span>
          </div>
          <div class="space-y-4">
            <div class="flex flex-col">
              <span class="text-dim text-[9px] uppercase font-bold mb-1">物理体积</span>
              <span :class="currentConflict.sourceSize > currentConflict.destSize ? 'text-green-500' : 'text-content'" class="font-mono font-black text-lg">
                {{ formatSize(currentConflict.sourceSize) }}
              </span>
            </div>
            <div class="flex flex-col">
              <span class="text-dim text-[9px] uppercase font-bold mb-1">源修改时间</span>
              <span :class="currentConflict.sourceModified > currentConflict.destModified ? 'text-green-500' : 'text-muted'" class="font-mono text-[10px]">
                {{ formatDate(currentConflict.sourceModified) }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- 操作动作区 -->
      <div class="actions-section flex flex-col gap-5 pt-4">
        <div class="flex flex-wrap gap-4">
          <!-- 推荐操作：自动重命名 -->
          <button @click="resolve('rename')" 
                  class="flex-[1.5] min-w-[160px] group relative h-14 rounded-2xl bg-primary text-white overflow-hidden transition-all duration-300 hover:scale-[1.02] active:scale-95 shadow-lg shadow-primary/20">
            <div class="absolute inset-0 bg-gradient-to-r from-white/0 via-white/20 to-white/0 -translate-x-full group-hover:translate-x-full transition-transform duration-1000"></div>
            <div class="flex items-center justify-center gap-3 relative z-10">
              <i class="pi pi-copy text-sm"></i>
              <span class="text-[11px] font-black uppercase tracking-widest">自动重命名保留两者</span>
            </div>
          </button>

          <!-- 覆盖操作：红色警示 -->
          <button @click="resolve('overwrite')" 
                  class="flex-1 min-w-[120px] h-14 rounded-2xl bg-red-500/10 border border-red-500/20 text-red-400 hover:bg-red-500 hover:text-white transition-all duration-300 font-black text-[10px] uppercase tracking-widest">
            替换现有文件
          </button>

          <!-- 跳过操作：次要按钮 -->
          <button @click="resolve('skip')" 
                  class="flex-1 min-w-[120px] h-14 rounded-2xl bg-input border border-subtle text-muted hover:text-content transition-all duration-300 font-black text-[10px] uppercase tracking-widest">
            放弃本次解压
          </button>
        </div>

        <!-- 批量应用选项 -->
        <div class="flex justify-center">
          <button @click="resolve('rename', true)" 
                  class="group flex items-center gap-3 px-6 py-3 rounded-full hover:bg-primary/5 transition-all text-[9px] font-black text-dim hover:text-primary uppercase tracking-[0.3em]">
            <div class="w-4 h-4 rounded-full border border-subtle group-hover:border-primary flex items-center justify-center transition-all">
              <div class="w-1.5 h-1.5 rounded-full bg-primary scale-0 group-hover:scale-100 transition-transform"></div>
            </div>
            应用到后续所有冲突项
          </button>
        </div>
      </div>
    </div>

    <!-- 完成状态 (空槽位) -->
    <div v-else class="py-20 text-center space-y-6">
      <div class="relative inline-block">
        <div class="absolute inset-0 animate-ping bg-primary/20 rounded-full"></div>
        <i class="pi pi-check-circle text-6xl text-primary relative z-10"></i>
      </div>
      <div class="space-y-2">
        <p class="text-lg font-black text-content">处理完毕</p>
        <p class="text-[10px] text-muted uppercase tracking-[0.2em]">All conflicts have been resolved</p>
      </div>
    </div>
  </Modal>
</template>

<style scoped>
.compare-card {
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.05);
}

.compare-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.1);
}

.conflict-container {
  animation: modal-fade-in 0.6s cubic-bezier(0.34, 1.56, 0.64, 1);
}

@keyframes modal-fade-in {
  from { opacity: 0; transform: scale(0.95) translateY(20px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}
</style>
