<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'
import CompressionSettingsPanel, { type CompressionOptions } from '@/components/compression/CompressionSettingsPanel.vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'

const appStore = useAppStore()
const tauriCommands = useTauriCommands()

const selectedFiles = ref<any[]>([])
const outputPath = ref('')
const compressionOptions = ref<CompressionOptions>({
  format: 'zip',
  level: 6,
  password: '',
  filename: '',
  splitArchive: false,
  splitSize: '1024',
  keepStructure: true,
  deleteAfter: false,
  createSolidArchive: false
})

const onFilesSelected = (files: any[]) => {
  selectedFiles.value = [...selectedFiles.value, ...files]
  if (selectedFiles.value.length > 0 && !compressionOptions.value.filename) {
    const firstFile = selectedFiles.value[0].name
    compressionOptions.value.filename = firstFile.split('.')[0] + '_compressed'
  }
}

const removeFile = (index: number) => {
  selectedFiles.value.splice(index, 1)
}

const handleCompress = async () => {
  if (selectedFiles.value.length === 0) return
  
  try {
    const filePaths = selectedFiles.value.map(f => f.path)
    await tauriCommands.compressFiles(filePaths, outputPath.value, compressionOptions.value)
    appStore.successMessage = appStore.t('common.success')
    selectedFiles.value = []
  } catch (error) {
    appStore.setError(`${appStore.t('common.error')}: ${error}`)
  }
}

const totalSize = computed(() => {
  const bytes = selectedFiles.value.reduce((sum, f) => sum + (f.size || 0), 0)
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
})
</script>

<template>
  <div class="compression-view p-responsive p-8 min-h-screen flex flex-col gap-8 transition-colors duration-700">
    <header class="flex justify-between items-end">
      <div>
        <h1 class="text-4xl font-black text-content tracking-tighter mb-2">{{ appStore.t('nav.compress') }}</h1>
        <p class="text-muted text-[10px] font-bold uppercase tracking-[0.3em] ml-1">Archive Construction Kit</p>
      </div>
      
      <div v-if="selectedFiles.length > 0" class="flex items-center gap-6">
        <div class="text-right">
          <div class="text-[8px] text-muted font-black uppercase tracking-widest">Payload</div>
          <div class="text-sm font-black text-primary">{{ totalSize }}</div>
        </div>
        <button 
          @click="handleCompress"
          class="px-8 py-3 rounded-2xl bg-primary text-white text-[10px] font-black uppercase tracking-widest shadow-lg shadow-primary/20 hover:scale-105 active:scale-95 transition-all"
        >
          Begin Compression
        </button>
      </div>
    </header>

    <div class="grid grid-cols-1 xl:grid-cols-12 gap-8 flex-1">
      <!-- 左侧：文件管理 (找回丢失的列表) -->
      <div class="xl:col-span-5 flex flex-col gap-6">
        <section class="aero-card p-8 flex-1 flex flex-col">
          <h2 class="text-[10px] font-black text-muted uppercase tracking-[0.3em] mb-6">Source Selection</h2>
          
          <EnhancedFileDropzone @files-selected="onFilesSelected" class="mb-6" />

          <div class="flex-1 overflow-auto custom-scrollbar pr-2">
            <TransitionGroup name="list" tag="div" class="space-y-3">
              <div v-for="(file, idx) in selectedFiles" :key="file.path" 
                   class="flex items-center justify-between p-4 rounded-2xl bg-input border border-subtle group hover:border-primary/30 transition-all">
                <div class="flex items-center gap-4 overflow-hidden">
                  <div class="w-10 h-10 rounded-xl bg-card border border-subtle flex items-center justify-center text-primary shadow-sm">
                    <i class="pi pi-file text-sm"></i>
                  </div>
                  <div class="overflow-hidden">
                    <div class="text-xs font-bold text-content truncate max-w-[200px]" :title="file.name">{{ file.name }}</div>
                    <div class="text-[9px] text-dim font-mono uppercase mt-1">{{ (file.size / 1024 / 1024).toFixed(2) }} MB</div>
                  </div>
                </div>
                <button @click="removeFile(idx)" class="w-8 h-8 rounded-full flex items-center justify-center text-dim hover:text-red-500 hover:bg-red-500/10 transition-all">
                  <i class="pi pi-times text-[10px]"></i>
                </button>
              </div>
            </TransitionGroup>
            
            <div v-if="selectedFiles.length === 0" class="flex flex-col items-center justify-center py-20 text-dim">
              <i class="pi pi-inbox text-3xl mb-4 opacity-20"></i>
              <p class="text-[10px] font-black uppercase tracking-widest">Queue Empty</p>
            </div>
          </div>
        </section>
      </div>

      <!-- 右侧：高级配置 -->
      <div class="xl:col-span-7 flex flex-col gap-6">
        <CompressionSettingsPanel 
          v-model="compressionOptions"
          v-model:outputPath="outputPath"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.compression-view {
  background: radial-gradient(circle at 0% 100%, color-mix(in srgb, var(--dynamic-accent) 4%, transparent) 0%, transparent 40%);
}

.list-enter-active, .list-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.list-enter-from { opacity: 0; transform: translateX(-20px); }
.list-leave-to { opacity: 0; transform: scale(0.9); }
</style>
