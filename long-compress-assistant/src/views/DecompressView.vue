<script setup lang="ts">
import { ref, onMounted } from 'vue'
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
    try {
      const options = {
        outputPath: appStore.settings.defaultOutputPath || '',
        keepStructure: true,
        overwrite: false,
        deleteAfter: appStore.settings.autoDeleteSource
      }
      await tauriCommands.decompressFile(file.path, options)
    } catch (error) {
      appStore.setError(`${appStore.t('common.error')}: ${error}`)
    }
  }
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
  <div class="decompress-view p-responsive p-8 min-h-screen flex flex-col gap-8 transition-colors duration-700">
    <header class="flex justify-between items-end">
      <div>
        <h1 class="text-4xl font-black text-content tracking-tighter mb-2">{{ appStore.t('nav.decompress') }}</h1>
        <p class="text-muted text-[10px] font-bold uppercase tracking-[0.3em] ml-1">{{ appStore.t('app.tagline') }}</p>
      </div>
      
      <div class="flex gap-4">
        <button 
          @click="taskStore.clearCompletedTasks()"
          class="px-6 py-2.5 rounded-[1.2rem] bg-input border border-subtle text-muted text-[10px] font-black uppercase tracking-widest hover:text-primary transition-all shadow-sm"
        >
          {{ appStore.t('common.delete') }} {{ appStore.language === 'zh-CN' ? '已完成' : 'Completed' }}
        </button>
      </div>
    </header>

    <div class="flex-1 min-h-0 flex flex-col gap-8">
      <transition name="fade-morph" mode="out-in">
        <!-- 拖拽感应区 (风格对齐：无感大背景) -->
        <section v-if="taskStore.tasks.length === 0" class="flex-1 flex flex-col items-center justify-center py-20 bg-card border border-subtle rounded-[3rem] shadow-glass border-dashed transition-all duration-700">
          <EnhancedFileDropzone @files-selected="onFilesSelected" class="w-full max-w-2xl" />
          
          <div class="mt-16 flex gap-12 items-center opacity-40 grayscale group-hover:grayscale-0 transition-all duration-700">
             <div class="text-center">
               <div class="text-2xl font-black text-content tracking-tighter">7Z / ZIP / RAR</div>
               <div class="text-[8px] font-bold text-muted uppercase tracking-[0.4em] mt-2">Core Engine</div>
             </div>
             <div class="w-px h-8 bg-subtle"></div>
             <div class="text-center">
               <div class="text-2xl font-black text-primary tracking-tighter">Smart Predict</div>
               <div class="text-[8px] font-bold text-primary uppercase tracking-[0.4em] mt-2 opacity-50">Automation</div>
             </div>
          </div>
        </section>

        <!-- 智慧表格工作区 -->
        <section v-else class="flex-1 flex flex-col gap-6">
          <AeroTable />
          
          <div class="flex justify-center">
            <button @click="() => {}" class="group flex items-center gap-4 px-10 py-4 rounded-[2rem] bg-card border border-subtle hover:border-primary/50 transition-all shadow-xl hover:shadow-glass-hover">
              <div class="w-6 h-6 rounded-full bg-primary/10 flex items-center justify-center group-hover:bg-primary transition-all">
                <i class="pi pi-plus text-primary group-hover:text-white transition-colors text-[10px]"></i>
              </div>
              <span class="text-muted text-[10px] font-black uppercase tracking-[0.2em] group-hover:text-content transition-colors">
                {{ appStore.language === 'zh-CN' ? '添加更多文件' : 'Add More Files' }}
              </span>
            </button>
          </div>
        </section>
      </transition>
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
