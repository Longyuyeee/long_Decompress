<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useTaskStore } from '@/stores/task'
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { open } from '@tauri-apps/api/dialog'
import AeroTable from '@/components/tasks/AeroTable.vue'
import ConflictResolutionModal from '@/components/tasks/ConflictResolutionModal.vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'

const taskStore = useTaskStore()
const appStore = useAppStore()
const tauriCommands = useTauriCommands()

const selectedConflictTaskId = ref<string | null>(null)
const showConflictModal = ref(false)

// 全局配置状态
const globalOutputPath = ref('')
const isGlobalSameDir = ref(true) // 默认开启同目录
const globalExtractToSubfolder = ref(false)

onMounted(async () => {
  await taskStore.initListeners()
})

const onFilesSelected = async (files: any[]) => {
  for (const file of files) {
    const sourcePath = file.path
    const parentDir = sourcePath.substring(0, Math.max(sourcePath.lastIndexOf('/'), sourcePath.lastIndexOf('\\')))
    
    taskStore.addTask({
      id: Math.random().toString(36).substr(2, 9),
      name: file.name || sourcePath.split(/[\\/]/).pop() || 'Unknown',
      type: 'decompression',
      sourceFiles: [sourcePath],
      outputPath: isGlobalSameDir.value ? parentDir : globalOutputPath.value,
      extractToSubfolder: globalExtractToSubfolder.value
    })
  }
}

const handleGlobalSelectDir = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: appStore.t('decompress.config.output_select')
    })
    if (selected && typeof selected === 'string') {
      globalOutputPath.value = selected
      isGlobalSameDir.value = false
      // 同步到所有待处理任务
      taskStore.tasks.forEach(t => {
        if (t.status === 'pending') t.outputPath = selected
      })
    }
  } catch (err) {
    console.error('Failed to select global dir:', err)
  }
}

const handleGlobalSetSameDir = () => {
  isGlobalSameDir.value = true
  globalOutputPath.value = ''
  // 同步到所有待处理任务：设置各自的父目录
  taskStore.tasks.forEach(t => {
    if (t.status === 'pending' && t.sourceFiles.length > 0) {
      const sp = t.sourceFiles[0]
      t.outputPath = sp.substring(0, Math.max(sp.lastIndexOf('/'), sp.lastIndexOf('\\')))
    }
  })
}

const toggleGlobalSubfolder = () => {
  globalExtractToSubfolder.value = !globalExtractToSubfolder.value
  taskStore.tasks.forEach(t => {
    if (t.status === 'pending') t.extractToSubfolder = globalExtractToSubfolder.value
  })
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
      // 传入 task.id 避免重复生成新任务
      await tauriCommands.decompressFile(task.sourceFiles[0], options, task.id)
      } catch (error) {
      taskStore.updateTaskStatus(task.id, 'failed')
      appStore.setError(`${appStore.t('common.error')}: ${error}`)
    }
  }
}

const hasPendingTasks = computed(() => taskStore.tasks.some(t => t.status === 'pending'))
const isRunning = computed(() => taskStore.tasks.some(t => ['running', 'extracting', 'compressing', 'preparing'].includes(t.status)))

const cancelAllTasks = () => {
  taskStore.tasks.forEach(async t => {
    if (['running', 'extracting', 'compressing', 'preparing'].includes(t.status)) {
      await tauriCommands.cancelCompression(t.id)
    }
  })
}

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
          v-if="!isRunning && taskStore.tasks.some(t => ['completed', 'failed', 'cancelled'].includes(t.status))"
          @click="taskStore.clearFinishedTasks()"
          class="h-10 px-6 rounded-xl bg-input border border-subtle text-muted text-[10px] font-black uppercase tracking-widest hover:text-red-400 transition-all shadow-sm flex items-center gap-2"
        >
          <i class="pi pi-trash"></i>
          {{ appStore.t('decompress.clear_finished') }}
        </button>
        <button 
          v-if="isRunning"
          @click="cancelAllTasks"
          class="h-10 px-6 rounded-xl bg-red-500/10 text-red-500 border border-red-500/20 text-[10px] font-black uppercase tracking-widest hover:bg-red-500 hover:text-white transition-all shadow-sm flex items-center gap-2"
        >
          <i class="pi pi-stop-circle"></i>
          停止解压
        </button>
        <button 
          v-if="hasPendingTasks && !isRunning"
          @click="startDecompression"
          class="h-10 px-6 rounded-xl bg-primary text-white text-[10px] font-black uppercase tracking-widest hover:brightness-110 transition-all shadow-lg flex items-center gap-2"
        >
          <i class="pi pi-play-circle animate-pulse"></i>
          {{ appStore.t('decompress.start_queue') }}
        </button>
      </div>
    </header>

    <div class="flex-1 min-h-0 aero-card overflow-hidden flex flex-col mb-6 relative border border-subtle bg-card/40 shadow-2xl">
      <div class="flex-1 overflow-hidden flex flex-col relative">
        <!-- 核心逻辑：有内容时 100% 空间给表格 -->
        <AeroTable v-if="taskStore.tasks.length > 0" class="flex-1" />

        <!-- 空状态：极简居中引导 -->
        <div v-else class="flex-1 flex flex-col items-center justify-center p-20">
          <EnhancedFileDropzone 
            @files-selected="onFilesSelected" 
            accept=".zip,.7z,.rar,.tar,.gz,.bz2,.xz,.iso"
            class="w-full max-w-lg shadow-sm" 
          />
        </div>
      </div>

      <!-- 底部辅助操作区 (收缩至左侧，避让右侧悬浮面板) -->
      <div v-if="taskStore.tasks.length > 0" class="p-3 border-t border-subtle bg-input/10 flex justify-start">
        <div class="w-1/4 min-w-[200px] max-w-[320px]">
          <EnhancedFileDropzone 
            @files-selected="onFilesSelected" 
            :compact="true" 
            accept=".zip,.7z,.rar,.tar,.gz,.bz2,.xz,.iso"
            class="w-full h-10" 
          />
        </div>
      </div>
    </div>

    <!-- 全局配置悬浮控制台 (极致通透悬浮风格) -->
    <div v-if="taskStore.tasks.length > 0" class="fixed bottom-6 right-6 z-50 bg-card/40 backdrop-blur-2xl border border-subtle/50 p-4 rounded-2xl shadow-2xl flex flex-col gap-3 group/float">
      <!-- 装饰性微光 -->
      <div class="absolute inset-0 rounded-2xl bg-primary/[0.02] opacity-0 group-hover/float:opacity-100 transition-opacity pointer-events-none"></div>
      
      <div class="flex items-center gap-5 relative z-10">
        <!-- 路径核心区 (左侧) -->
        <div class="flex flex-col min-w-[120px] max-w-[240px]">
          <span class="text-[8px] font-black text-primary uppercase tracking-widest mb-1 opacity-80">{{ appStore.t('decompress.config.output') }}</span>
          <div class="text-[11px] font-mono text-content font-bold truncate leading-tight tracking-tight">
            {{ isGlobalSameDir ? appStore.t('decompress.config.output_auto') : (globalOutputPath || appStore.t('decompress.config.output_auto')) }}
          </div>
        </div>

        <!-- 垂直分隔线 -->
        <div class="w-px h-8 bg-subtle/20 shrink-0"></div>

        <!-- 操作区 (右侧) -->
        <div class="flex gap-2 shrink-0">
          <!-- 选择目录：作为核心主按钮 -->
          <button @click="handleGlobalSelectDir" 
                  class="h-7 px-3.5 rounded-lg bg-primary text-white hover:brightness-110 active:scale-95 transition-all text-[10px] font-black flex items-center gap-2 shadow-sm shadow-primary/20">
            <i class="pi pi-folder-open text-[11px]"></i>
            {{ appStore.t('decompress.config.output_select') }}
          </button>
          
          <!-- 同目录：浅色通透风格 -->
          <button @click="handleGlobalSetSameDir" 
                  :class="isGlobalSameDir ? 'bg-primary/10 text-primary border-primary/20 shadow-inner' : 'bg-input/30 text-muted border-subtle/50'"
                  class="h-7 px-3 rounded-lg border text-[10px] font-bold transition-all hover:bg-primary/5">
            {{ appStore.t('decompress.config.output_same') }}
          </button>
        </div>
      </div>

      <!-- 底部辅助区 (极细分隔线 + 复选框) -->
      <div class="pt-2 border-t border-subtle/10 flex justify-end relative z-10">
        <div class="flex items-center gap-2.5 cursor-pointer group/subcheck transition-all" @click="toggleGlobalSubfolder">
          <div class="w-3.5 h-3.5 rounded border border-primary/30 flex items-center justify-center transition-all group-hover/subcheck:border-primary" 
               :class="globalExtractToSubfolder ? 'bg-primary border-primary' : 'bg-transparent'">
            <i v-if="globalExtractToSubfolder" class="pi pi-check text-[7px] text-white"></i>
          </div>
          <span class="text-[9px] font-black text-muted group-hover/subcheck:text-primary transition-colors uppercase tracking-widest">
            {{ appStore.t('decompress.config.output_sub') }}
          </span>
        </div>
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
