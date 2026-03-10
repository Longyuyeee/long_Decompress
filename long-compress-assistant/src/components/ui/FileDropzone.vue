<template>
  <div
    class="file-dropzone"
    :class="{ 'is-dragging': isDragging, 'is-disabled': disabled }"
    @dragover.prevent="onDragOver"
    @dragleave.prevent="onDragLeave"
    @drop.prevent="onDrop"
    @click="selectFiles"
  >
    <div class="dropzone-content">
      <i class="pi pi-cloud-upload text-4xl mb-4 text-primary"></i>
      <h3 class="text-lg font-semibold mb-2">点击或拖拽文件到此处</h3>
      <p class="text-sm text-gray-500">支持 ZIP, RAR, 7Z 等多种格式</p>
      <div v-if="error" class="mt-4 text-red-500 text-sm">
        {{ error }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'

export interface Props {
  multiple?: boolean
  disabled?: boolean
  maxSize?: number
  accept?: string[]
}

const props = withDefaults(defineProps<Props>(), {
  multiple: true,
  disabled: false,
  maxSize: 1024 * 1024 * 100 // 100MB
})

const emit = defineEmits<{
  (e: 'files-selected', files: string[]): void
  (e: 'error', message: string): void
}>()

const isDragging = ref(false)
const error = ref<string | null>(null)

const onDragOver = () => {
  if (props.disabled) return
  isDragging.value = true
}

const onDragLeave = () => {
  isDragging.value = false
}

const onDrop = (event: DragEvent) => {
  if (props.disabled) return
  isDragging.value = false
  // 拖拽逻辑通常在 Tauri 中通过 native 处理，或者处理 Web 文件
  const files = event.dataTransfer?.files
  if (files) {
    const paths = Array.from(files).map(f => (f as any).path || f.name)
    emit('files-selected', paths)
  }
}

const selectFiles = async () => {
  if (props.disabled) return
  try {
    const selected = await open({
      multiple: props.multiple,
      filters: props.accept ? [{ name: 'Archive', extensions: props.accept }] : []
    })
    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected]
      emit('files-selected', paths)
    }
  } catch (err) {
    error.value = '选择文件失败'
    emit('error', '选择文件失败')
  }
}

defineExpose({
  clearError: () => (error.value = null)
})
</script>

<style scoped>
.file-dropzone {
  @apply border-2 border-dashed border-gray-300 rounded-xl p-12 text-center cursor-pointer transition-all hover:border-primary hover:bg-primary/5;
}
.is-dragging {
  @apply border-primary bg-primary/10 scale-[1.02];
}
.is-disabled {
  @apply opacity-50 cursor-not-allowed grayscale;
}
</style>
