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
    title="文件冲突检测"
    icon="pi pi-exclamation-triangle"
    size="lg"
  >
    <div v-if="currentConflict" class="space-y-8 bg-modal text-content p-1">
      <div class="text-muted text-sm leading-relaxed">
        目标路径已存在同名文件：<span class="text-primary font-bold font-mono">{{ currentConflict.fileName }}</span>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <!-- 现有文件 -->
        <div class="p-6 rounded-[1.5rem] bg-input border border-subtle relative overflow-hidden group">
          <div class="absolute -right-4 -top-4 w-16 h-16 bg-red-500/5 rounded-full"></div>
          <div class="text-[9px] font-black text-muted uppercase tracking-[0.2em] mb-4">Existing Registry</div>
          <div class="space-y-3">
            <div class="flex justify-between text-xs items-center">
              <span class="text-muted">Capacity</span>
              <span class="text-content font-bold">{{ formatSize(currentConflict.destSize) }}</span>
            </div>
            <div class="flex justify-between text-[10px] items-center">
              <span class="text-dim">Last Mod</span>
              <span class="text-muted font-mono">{{ formatDate(currentConflict.destModified) }}</span>
            </div>
          </div>
        </div>

        <!-- 新文件 -->
        <div class="p-6 rounded-[1.5rem] bg-primary/5 border border-primary/20 relative overflow-hidden group">
          <div class="absolute -right-4 -top-4 w-16 h-16 bg-primary/10 rounded-full"></div>
          <div class="text-[9px] font-black text-primary uppercase tracking-[0.2em] mb-4">Incoming Data</div>
          <div class="space-y-3">
            <div class="flex justify-between text-xs items-center">
              <span class="text-muted">Capacity</span>
              <span :class="currentConflict.sourceSize > currentConflict.destSize ? 'text-green-500 font-black' : 'text-content font-bold'">
                {{ formatSize(currentConflict.sourceSize) }}
              </span>
            </div>
            <div class="flex justify-between text-[10px] items-center">
              <span class="text-dim">Source Mod</span>
              <span :class="currentConflict.sourceModified > currentConflict.destModified ? 'text-green-500 font-black' : 'text-muted font-mono'">
                {{ formatDate(currentConflict.sourceModified) }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <div class="flex flex-col gap-4 pt-4">
        <div class="flex flex-wrap gap-3">
          <button @click="resolve('overwrite')" class="flex-1 min-w-[120px] py-3.5 rounded-xl bg-red-500/80 hover:bg-red-500 text-white text-xs font-black transition-all shadow-lg shadow-red-500/20">
            OVERWRITE
          </button>
          <button @click="resolve('rename')" class="flex-1 min-w-[120px] py-3.5 rounded-xl bg-primary text-white text-xs font-black transition-all shadow-lg shadow-primary/20">
            RENAME AUTO
          </button>
          <button @click="resolve('skip')" class="flex-1 min-w-[120px] py-3.5 rounded-xl bg-input border border-subtle text-muted text-xs font-bold hover:text-content transition-all">
            SKIP FILE
          </button>
        </div>
        <button @click="resolve('overwrite', true)" class="w-full py-2 text-[9px] text-dim hover:text-primary transition-colors uppercase font-black tracking-[0.3em]">
          Apply To All Following Conflicts
        </button>
      </div>
    </div>
    
    <div v-else class="py-16 text-center text-dim space-y-4">
      <i class="pi pi-check-circle text-5xl text-primary/20"></i>
      <p class="text-xs font-black uppercase tracking-widest">Conflict Resolution Complete</p>
    </div>
  </Modal>
</template>
