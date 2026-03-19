<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '@/stores/app'

const props = defineProps({
  compact: {
    type: Boolean,
    default: false
  }
})

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
    :class="{ 
      'is-dragging': isDragging, 
      'p-12 rounded-[2.5rem]': !compact, 
      'p-3 rounded-xl border-dashed opacity-40 hover:opacity-100': compact 
    }"
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
    
    <div v-if="!compact" class="flex flex-col items-center justify-center space-y-4">
      <div class="w-16 h-16 rounded-[1.5rem] bg-input border border-subtle flex items-center justify-center text-dim group-hover:text-primary group-hover:scale-110 transition-all duration-500 shadow-sm">
        <i class="pi pi-cloud-upload text-2xl"></i>
      </div>
      
      <div class="text-center">
        <p class="text-xs font-black text-content tracking-tight mb-0.5">{{ appStore.t('decompress.drop_hint') }}</p>
        <p class="text-[8px] text-muted font-bold uppercase tracking-widest opacity-40">ZIP, 7Z, RAR, TAR</p>
      </div>

      <div class="pt-1">
        <span class="px-4 py-1.5 rounded-full bg-primary/10 border border-primary/20 text-primary text-[8px] font-black uppercase tracking-widest group-hover:bg-primary group-hover:text-white transition-all">
          Browse Files
        </span>
      </div>
    </div>

    <div v-else class="flex items-center justify-center gap-3">
      <i class="pi pi-plus-circle text-primary text-xs"></i>
      <span class="text-[9px] font-black text-muted uppercase tracking-[0.2em] group-hover:text-content transition-colors">
        {{ appStore.t('decompress.add_files') }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.drop-area {
  @apply relative border-2 border-dashed border-subtle cursor-pointer transition-all duration-500;
  background-color: transparent;
}

.drop-area:hover {
  border-color: var(--dynamic-accent);
  background-color: color-mix(in srgb, var(--dynamic-accent) 2%, transparent);
}

.is-dragging {
  border-color: var(--dynamic-accent);
  background-color: color-mix(in srgb, var(--dynamic-accent) 8%, transparent);
  transform: scale(1.01);
}
</style>
