<script setup lang="ts">
import { computed } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import { useTaskStore, type ConflictInfo } from '@/stores/task'

const props = defineProps<{
  taskId: string
  visible: boolean
}>()

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void
  (e: 'resolve', action: 'overwrite' | 'skip' | 'rename', applyToAll: boolean): void
}>()

const taskStore = useTaskStore()
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
  // 模拟从 store 中移除已处理的冲突
  if (task.value) {
    if (applyToAll) {
      task.value.conflicts = []
    } else {
      task.value.conflicts.shift()
    }
    
    if (task.value.conflicts.length === 0) {
      emit('update:visible', false)
    }
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
    <div v-if="currentConflict" class="space-y-6">
      <div class="text-white/80 text-sm">
        目标路径已存在同名文件：<span class="text-blue-400 font-mono">{{ currentConflict.fileName }}</span>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <!-- 现有文件 -->
        <div class="p-4 rounded-xl bg-white/5 border border-white/10">
          <div class="text-[10px] font-bold text-white/40 uppercase tracking-widest mb-3">现有文件 (目标)</div>
          <div class="space-y-2">
            <div class="flex justify-between text-xs">
              <span class="text-white/30">大小</span>
              <span class="text-white/70">{{ formatSize(currentConflict.destSize) }}</span>
            </div>
            <div class="flex justify-between text-xs">
              <span class="text-white/30">修改时间</span>
              <span class="text-white/70">{{ formatDate(currentConflict.destModified) }}</span>
            </div>
          </div>
        </div>

        <!-- 新文件 -->
        <div class="p-4 rounded-xl bg-blue-500/10 border border-blue-500/20">
          <div class="text-[10px] font-bold text-blue-400/60 uppercase tracking-widest mb-3">新文件 (源)</div>
          <div class="space-y-2">
            <div class="flex justify-between text-xs">
              <span class="text-white/30">大小</span>
              <span :class="currentConflict.sourceSize > currentConflict.destSize ? 'text-green-400' : 'text-white/70'">
                {{ formatSize(currentConflict.sourceSize) }}
              </span>
            </div>
            <div class="flex justify-between text-xs">
              <span class="text-white/30">修改时间</span>
              <span :class="currentConflict.sourceModified > currentConflict.destModified ? 'text-green-400' : 'text-white/70'">
                {{ formatDate(currentConflict.sourceModified) }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <div class="flex flex-col gap-3 pt-4">
        <div class="flex gap-3">
          <button @click="resolve('overwrite')" class="flex-1 py-2.5 rounded-xl bg-red-500/20 hover:bg-red-500/30 border border-red-500/30 text-red-400 text-sm transition-all">
            覆盖现有文件
          </button>
          <button @click="resolve('rename')" class="flex-1 py-2.5 rounded-xl bg-blue-500/20 hover:bg-blue-500/30 border border-blue-500/30 text-blue-400 text-sm transition-all">
            自动重命名
          </button>
          <button @click="resolve('skip')" class="flex-1 py-2.5 rounded-xl bg-white/10 hover:bg-white/20 border border-white/10 text-white/60 text-sm transition-all">
            跳过此文件
          </button>
        </div>
        <button @click="resolve('overwrite', true)" class="w-full py-2 text-[10px] text-white/30 hover:text-white/60 transition-colors uppercase tracking-widest">
          应用于所有后续冲突
        </button>
      </div>
    </div>
    
    <div v-else class="py-12 text-center text-white/20">
      <i class="pi pi-check-circle text-4xl mb-4"></i>
      <p>所有冲突已解决</p>
    </div>
  </Modal>
</template>
