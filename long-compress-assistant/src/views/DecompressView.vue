<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useTaskStore } from '@/stores/task'
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'
import AeroTable from '@/components/tasks/AeroTable.vue'
import ConflictResolutionModal from '@/components/tasks/ConflictResolutionModal.vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import GlassCard from '@/components/ui/GlassCard.vue'

const taskStore = useTaskStore()
const appStore = useAppStore()
const tauriCommands = useTauriCommands()

const isDecompressing = ref(false)
const selectedConflictTaskId = ref<string | null>(null)
const showConflictModal = ref(false)

onMounted(async () => {
  await taskStore.initListeners()
})

const onFilesSelected = async (files: any[]) => {
  for (const file of files) {
    try {
      // 默认解压选项
      const options = {
        outputPath: appStore.settings.defaultOutputPath || file.path.substring(0, file.path.lastIndexOf(/[\\/]/)),
        keepStructure: true,
        overwrite: false,
        deleteAfter: appStore.settings.autoDeleteSource
      }
      
      // 调用智能解压 (MS2-2 逻辑)
      await tauriCommands.decompressFile(file.path, options)
    } catch (error) {
      console.error('Failed to start decompression:', error)
      appStore.setError(`启动解压失败: ${error}`)
    }
  }
}

// 监听冲突任务
const handleConflict = (taskId: string) => {
  selectedConflictTaskId.value = taskId
  showConflictModal.value = true
}

// 监测 store 中的任务冲突并触发弹窗
taskStore.$subscribe((mutation, state) => {
  const taskWithConflict = state.tasks.find(t => t.conflicts.length > 0)
  if (taskWithConflict && !showConflictModal.value) {
    handleConflict(taskWithConflict.id)
  }
})
</script>

<template>
  <div class="decompress-view p-8 min-h-screen">
    <!-- 极简页头 -->
    <header class="mb-10 flex justify-between items-end">
      <div>
        <h1 class="text-4xl font-black text-white tracking-tighter mb-2">解压中心</h1>
        <p class="text-white/40 text-sm font-medium tracking-wide uppercase">Extraction Workspace v2.1</p>
      </div>
      
      <div class="flex gap-4">
        <button class="px-6 py-2 rounded-xl bg-white/5 border border-white/10 text-white/60 text-xs font-bold uppercase tracking-widest hover:bg-white/10 transition-all">
          清空已完成
        </button>
      </div>
    </header>

    <div class="grid grid-cols-1 gap-8">
      <!-- 拖拽感应区 (无感导入) -->
      <section v-if="taskStore.tasks.length === 0" class="flex flex-col items-center justify-center py-20">
        <EnhancedFileDropzone @files-selected="onFilesSelected" class="w-full max-w-2xl" />
        <div class="mt-8 flex gap-8">
           <div class="text-center">
             <div class="text-2xl font-bold text-white/80">ZIP / 7Z / RAR</div>
             <div class="text-[10px] text-white/30 uppercase tracking-[0.2em] mt-1 text-center">Supported Formats</div>
           </div>
           <div class="w-px h-10 bg-white/10"></div>
           <div class="text-center">
             <div class="text-2xl font-bold text-blue-400">Smart Predict</div>
             <div class="text-[10px] text-white/30 uppercase tracking-[0.2em] mt-1 text-center">Password Fingerprint</div>
           </div>
        </div>
      </section>

      <!-- 智慧表格工作区 -->
      <section v-else class="space-y-6">
        <AeroTable />
        
        <!-- 底部添加更多按钮 (小巧) -->
        <div class="flex justify-center">
          <button @click="() => {}" class="group flex items-center gap-3 px-8 py-3 rounded-2xl bg-white/5 border border-white/10 hover:border-white/20 transition-all">
            <i class="pi pi-plus text-white/40 group-hover:text-blue-400 transition-colors"></i>
            <span class="text-white/40 text-xs font-bold uppercase tracking-widest group-hover:text-white/80 transition-colors">添加更多文件</span>
          </button>
        </div>
      </section>
    </div>

    <!-- 冲突处理弹窗 -->
    <ConflictResolutionModal 
      v-if="selectedConflictTaskId"
      v-model:visible="showConflictModal"
      :taskId="selectedConflictTaskId"
    />
  </div>
</template>

<style scoped>
.decompress-view {
  background: radial-gradient(circle at 0% 0%, rgba(59, 130, 246, 0.05) 0%, transparent 50%),
              radial-gradient(circle at 100% 100%, rgba(139, 92, 246, 0.05) 0%, transparent 50%);
}
</style>
