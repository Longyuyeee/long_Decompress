<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '@/stores/app'

const appStore = useAppStore()
const isDragging = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)

const emit = defineEmits(['files-selected'])

const onDragOver = (e: DragEvent) => {
  e.preventDefault()
  isDragging.value = true
}

const onDragLeave = () => {
  isDragging.value = false
}

const onDrop = (e: DragEvent) => {
  e.preventDefault()
  isDragging.value = false
  const files = e.dataTransfer?.files
  if (files && files.length > 0) {
    handleFiles(Array.from(files))
  }
}

const onFileChange = (e: Event) => {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    handleFiles(Array.from(target.files))
  }
}

const triggerFileInput = () => {
  fileInput.value?.click()
}

const handleFiles = (files: File[]) => {
  const fileData = files.map(file => ({
    name: file.name,
    path: (file as any).path || file.name,
    size: file.size,
    type: file.type
  }))
  emit('files-selected', fileData)
}
</script>

<template>
  <div 
    class="drop-area group"
    :class="{ 'is-dragging': isDragging }"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="onDrop"
    @click="triggerFileInput"
  >
    <input 
      type="file" 
      ref="fileInput" 
      class="hidden" 
      multiple 
      @change="onFileChange"
    >
    
    <div class="flex flex-col items-center justify-center space-y-4">
      <div class="w-20 h-20 rounded-[2rem] bg-input border border-subtle flex items-center justify-center text-dim group-hover:text-primary group-hover:scale-110 group-hover:rotate-3 transition-all duration-500 shadow-sm group-hover:shadow-lg group-hover:shadow-primary/10">
        <i class="pi pi-cloud-upload text-3xl"></i>
      </div>
      
      <div class="text-center">
        <p class="text-sm font-black text-content tracking-tight mb-1">{{ appStore.t('decompress.drop_hint') }}</p>
        <p class="text-[10px] text-muted font-bold uppercase tracking-widest opacity-60">Supports ZIP, 7Z, RAR, TAR & more</p>
      </div>

      <div class="pt-2">
        <span class="px-4 py-1.5 rounded-full bg-primary/10 border border-primary/20 text-primary text-[9px] font-black uppercase tracking-widest group-hover:bg-primary group-hover:text-white transition-all">
          Browse Files
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.drop-area {
  @apply relative border-2 border-dashed border-subtle rounded-[2.5rem] p-12 cursor-pointer transition-all duration-500;
  background-color: var(--bg-card);
}

.drop-area:hover {
  border-color: var(--dynamic-accent);
  background-color: color-mix(in srgb, var(--dynamic-accent) 3%, var(--bg-card));
  transform: translateY(-2px);
}

.is-dragging {
  border-color: var(--dynamic-accent);
  background-color: color-mix(in srgb, var(--dynamic-accent) 8%, var(--bg-card));
  transform: scale(1.02);
}
</style>
