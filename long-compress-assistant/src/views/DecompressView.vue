<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useTaskStore } from '@/stores/task'
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'
import AeroTable from '@/components/tasks/AeroTable.vue'
import ConflictResolutionModal from '@/components/tasks/ConflictResolutionModal.vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'

const taskStore = useTaskStore()
const appStore = useAppStore()
const tauriCommands = useTauriCommands()

const selectedConflictTaskId = ref<string | null>(null)
const showConflictModal = ref(false)

onMounted(async () => {
  await taskStore.initListeners()
})

const onFilesSelected = async (files: any[]) => {
  for (const file of files) {
    // 仅添加到任务列表，不立即开始
    taskStore.addTask({
      id: Math.random().toString(36).substr(2, 9),
      name: file.name || file.path.split(/[\\/]/).pop() || 'Unknown',
      type: 'decompression',
      sourceFiles: [file.path],
      outputPath: appStore.settings.defaultOutputPath || '',
    })
  }
}

const startDecompression = async () => {
  const pendingTasks = taskStore.tasks.filter(t => t.status === 'pending')
  if (pendingTasks.length === 0) return

  for (const task of pendingTasks) {
    try {
      taskStore.updateTaskStatus(task.id, 'preparing')
      const options = {
        outputPath: task.outputPath,
        keepStructure: true,
        overwrite: false,
        deleteAfter: appStore.settings.autoDeleteSource
      }
      // 真正启动后端解压
      await tauriCommands.decompressFile(task.sourceFiles[0], options, task.id)
    } catch (error) {
      taskStore.updateTaskStatus(task.id, 'failed')
      appStore.setError(`${appStore.t('common.error')}: ${error}`)
    }
  }
}

const hasPendingTasks = computed(() => taskStore.tasks.some(t => t.status === 'pending'))

const handleConflict = (taskId: string) => {
  selectedConflictTaskId.value = taskId
  showConflictModal.value = true
}

taskStore.$subscribe((mutation, state) => {
  const taskWithConflict = state.tasks.find(t => t.conflicts.length > 0)
  if (taskWithConflict && !showConflictModal.value) {
    handleConflict(taskWithConflict.id)
  }
})
</script>

<template>
  <div class="decompress-view p-responsive p-8 h-screen flex flex-col gap-8 transition-colors duration-700 relative overflow-hidden">
    <header class="flex justify-between items-center shrink-0">
      <div>
        <h1 class="text-4xl font-black text-content tracking-tighter mb-1">{{ appStore.t('nav.decompress') }}</h1>
        <p class="text-muted text-[10px] font-bold uppercase tracking-[0.3em] ml-1">{{ appStore.t('app.tagline') }}</p>
      </div>
      
      <!-- 合理的操作按钮分配 -->
      <div class="flex gap-3">
        <button 
          @click="taskStore.clearFinishedTasks()"
          class="h-10 px-6 rounded-xl bg-input border border-subtle text-muted text-[10px] font-black uppercase tracking-widest hover:text-red-400 transition-all shadow-sm flex items-center gap-2"
        >
          <i class="pi pi-trash"></i>
          {{ appStore.t('decompress.clear_finished') }}
        </button>
        <button 
          v-if="hasPendingTasks"
          @click="startDecompression"
          class="h-10 px-6 rounded-xl bg-primary text-white text-[10px] font-black uppercase tracking-widest hover:brightness-110 transition-all shadow-lg flex items-center gap-2"
        >
          <i class="pi pi-play-circle animate-pulse"></i>
          {{ appStore.t('decompress.start_queue') }}
        </button>
      </div>
    </header>

    <div class="flex-1 min-h-0 aero-card overflow-hidden flex flex-col mb-12 relative border border-subtle bg-card/40 shadow-2xl">
      <div class="flex-1 overflow-hidden flex flex-col relative">
        <!-- 核心逻辑：有内容时 100% 空间给表格 -->
        <AeroTable v-if="taskStore.tasks.length > 0" class="flex-1" />

        <!-- 空状态：极简居中引导 -->
        <div v-else class="flex-1 flex flex-col items-center justify-center p-20">
          <EnhancedFileDropzone @files-selected="onFilesSelected" class="w-full max-w-lg shadow-sm" />
          <div class="mt-10 flex gap-10 items-center opacity-10 grayscale transition-all duration-700">
             <div class="text-xl font-black text-content tracking-tighter italic">LongEngine v2.7</div>
          </div>
        </div>
      </div>

      <!-- 底部辅助操作区 (极简紧凑型) -->
      <div v-if="taskStore.tasks.length > 0" class="p-2 border-t border-subtle bg-input/10">
        <EnhancedFileDropzone @files-selected="onFilesSelected" :compact="true" class="w-full" />
      </div>
    </div>

    <ConflictResolutionModal 
      v-if="selectedConflictTaskId"
      v-model:visible="showConflictModal"
      :taskId="selectedConflictTaskId"
    />
  </div>
</template>

<style scoped>
.decompress-view {
  background: radial-gradient(circle at 0% 0%, color-mix(in srgb, var(--dynamic-accent) 4%, transparent) 0%, transparent 40%);
}

.fade-morph-enter-active, .fade-morph-leave-active { transition: all 0.6s cubic-bezier(0.34, 1.56, 0.64, 1); }
.fade-morph-enter-from { opacity: 0; transform: scale(0.98); }
.fade-morph-leave-to { opacity: 0; transform: scale(1.02); }
</style>
