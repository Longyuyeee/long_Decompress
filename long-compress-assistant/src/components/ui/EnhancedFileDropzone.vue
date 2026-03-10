<template>
  <div class="enhanced-file-dropzone space-y-4">
    <div
      class="drop-area"
      :class="{ 'is-dragging': isDragging, 'is-disabled': disabled }"
      @dragover.prevent="onDragOver"
      @dragleave.prevent="onDragLeave"
      @drop.prevent="onDrop"
      @click="selectFiles"
    >
      <div class="text-center">
        <i class="pi pi-inbox text-5xl text-primary mb-4"></i>
        <h3 class="text-xl font-semibold">拖拽文件到这里或点击选择</h3>
        <p class="text-gray-500 mt-2">支持批量选择多个压缩包</p>
      </div>
    </div>

    <!-- 已选择文件列表 -->
    <div v-if="selectedFiles.length > 0" class="file-list space-y-2">
      <div
        v-for="file in selectedFiles"
        :key="file.path"
        class="flex items-center justify-between p-3 glass-card"
      >
        <div class="flex items-center space-x-3 truncate">
          <i class="pi pi-file text-primary"></i>
          <span class="truncate">{{ file.name }}</span>
        </div>
        <button @click.stop="removeFile(file.path)" class="text-red-500">
          <i class="pi pi-times"></i>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'

export interface FileItem {
  name: string
  path: string
  size: number
}

export interface Props {
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  disabled: false
})

const emit = defineEmits<{
  (e: 'files-selected', files: FileItem[]): void
}>()

const isDragging = ref(false)
const selectedFiles = ref<FileItem[]>([])

const onDragOver = () => { if (!props.disabled) isDragging.value = true }
const onDragLeave = () => { isDragging.value = false }

const onDrop = (event: DragEvent) => {
  isDragging.value = false
  if (props.disabled) return
  // Tauri 拖拽处理
}

const selectFiles = async () => {
  if (props.disabled) return
  try {
    const selected = await open({ multiple: true })
    if (selected && Array.isArray(selected)) {
      const newFiles = selected.map(p => ({
        name: p.split(/[\\/]/).pop() || p,
        path: p,
        size: 0
      }))
      selectedFiles.value = [...selectedFiles.value, ...newFiles]
      emit('files-selected', selectedFiles.value)
    }
  } catch (err) {
    console.error('选择文件失败', err)
  }
}

const removeFile = (path: string) => {
  selectedFiles.value = selectedFiles.value.filter(f => f.path !== path)
  emit('files-selected', selectedFiles.value)
}

defineExpose({
  clear: () => (selectedFiles.value = [])
})
</script>

<style scoped>
.drop-area {
  @apply border-2 border-dashed border-gray-300 rounded-2xl p-10 cursor-pointer transition-all hover:border-primary hover:bg-primary/5;
}
.is-dragging {
  @apply border-primary bg-primary/10;
}
.glass-card {
  @apply bg-white/5 backdrop-blur-md border border-white/10 rounded-xl;
}
</style>
