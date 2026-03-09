<template>
  <div
    ref="dropzoneRef"
    :class="[
      'file-dropzone',
      'rounded-xl',
      'border-2',
      'border-dashed',
      'p-8',
      'text-center',
      'transition-all',
      'duration-300',
      'cursor-pointer',
      isDragging ? 'border-primary bg-primary/5 scale-105' : 'border-gray-300 dark:border-gray-600 hover:border-primary',
      className
    ]"
    @click="handleClick"
    @dragover="handleDragOver"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
  >
    <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-primary/10 flex items-center justify-center">
      <i class="pi pi-cloud-upload text-primary text-2xl"></i>
    </div>

    <p class="text-gray-700 dark:text-gray-300 mb-2">
      <span v-if="isDragging">释放文件到此�?/span>
      <span v-else>
        拖放文件到此处，�?
        <button class="text-primary hover:underline font-medium">点击选择文件</button>
      </span>
    </p>

    <p class="text-gray-500 dark:text-gray-400 text-sm">
      支持 {{ acceptedFormats.join(', ') }} 等格�?
      <span v-if="maxSize">，最�?{{ formatFileSize(maxSize) }}</span>
    </p>

    <!-- 文件列表 -->
    <div v-if="files.length > 0" class="mt-6">
      <h4 class="font-medium text-gray-900 dark:text-white mb-3 text-left">已选择文件 ({{ files.length }})</h4>
      <div class="space-y-2 max-h-60 overflow-y-auto">
        <div
          v-for="file in files"
          :key="file.id"
          class="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800 group hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
        >
          <div class="flex items-center min-w-0">
            <i :class="getFileIcon(file)" class="text-gray-500 mr-3 flex-shrink-0"></i>
            <div class="min-w-0">
              <p class="font-medium text-gray-900 dark:text-white truncate">{{ file.name }}</p>
              <p class="text-sm text-gray-500 dark:text-gray-400">{{ formatFileSize(file.size) }}</p>
            </div>
          </div>
          <button
            @click.stop="removeFile(file.id)"
            class="text-gray-400 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-opacity"
          >
            <i class="pi pi-times"></i>
          </button>
        </div>
      </div>
    </div>

    <!-- 错误提示 -->
    <div v-if="error" class="mt-4 p-3 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
      <div class="flex items-center">
        <i class="pi pi-exclamation-triangle text-red-500 mr-2"></i>
        <span class="text-red-700 dark:text-red-300 text-sm">{{ error }}</span>
      </div>
    </div>

    <!-- 隐藏的文件输�?-->
    <input
      ref="fileInputRef"
      type="file"
      :multiple="multiple"
      :accept="accept"
      class="hidden"
      @change="handleFileInput"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'

interface FileItem {
  id: string
  name: string
  size: number
  type: string
  file: File
}

interface Props {
  multiple?: boolean
  accept?: string
  maxSize?: number // bytes
  maxFiles?: number
  className?: string
}

const props = withDefaults(defineProps<Props>(), {
  multiple: true,
  accept: '.zip,.rar,.7z,.tar,.gz,.bz2',
  maxSize: undefined,
  maxFiles: undefined,
  className: ''
})

const emit = defineEmits<{
  'files-selected': [files: FileItem[]]
  'file-removed': [fileId: string]
  'error': [error: string]
}>()

const dropzoneRef = ref<HTMLDivElement>()
const fileInputRef = ref<HTMLInputElement>()
const files = ref<FileItem[]>([])
const isDragging = ref(false)
const error = ref('')

const acceptedFormats = computed(() => {
  return props.accept.split(',').map(format => format.trim().replace('.', '').toUpperCase())
})

const formatFileSize = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const getFileIcon = (file: FileItem): string => {
  const extension = file.name.split('.').pop()?.toLowerCase()

  switch (extension) {
    case 'zip':
      return 'pi pi-file-archive text-blue-500'
    case 'rar':
      return 'pi pi-file-archive text-red-500'
    case '7z':
      return 'pi pi-file-archive text-green-500'
    case 'tar':
    case 'gz':
    case 'bz2':
      return 'pi pi-file-archive text-purple-500'
    default:
      return 'pi pi-file text-gray-500'
  }
}

const handleClick = () => {
  if (fileInputRef.value) {
    fileInputRef.value.click()
  }
}

const handleDragOver = (event: DragEvent) => {
  event.preventDefault()
  event.stopPropagation()
  isDragging.value = true
}

const handleDragLeave = (event: DragEvent) => {
  event.preventDefault()
  event.stopPropagation()

  // 只有当鼠标离开dropzone元素时才取消拖拽状�?
  if (dropzoneRef.value && event.relatedTarget && !dropzoneRef.value.contains(event.relatedTarget as Node)) {
    isDragging.value = false
  }
}

const handleDrop = (event: DragEvent) => {
  event.preventDefault()
  event.stopPropagation()
  isDragging.value = false

  if (!event.dataTransfer?.files.length) return

  const droppedFiles = Array.from(event.dataTransfer.files)
  processFiles(droppedFiles)
}

const handleFileInput = (event: Event) => {
  const input = event.target as HTMLInputElement
  if (!input.files?.length) return

  const selectedFiles = Array.from(input.files)
  processFiles(selectedFiles)

  // 重置input以便可以选择相同的文�?
  input.value = ''
}

const processFiles = (fileList: File[]) => {
  error.value = ''

  // 检查文件数量限�?
  if (props.maxFiles && files.value.length + fileList.length > props.maxFiles) {
    error.value = `最多只能选择 ${props.maxFiles} 个文件`
    emit('error', error.value)
    return
  }

  const newFiles: FileItem[] = []

  for (const file of fileList) {
    // 检查文件大小限�?
    if (props.maxSize && file.size > props.maxSize) {
      error.value = `文件 "${file.name}" 超过最大大小限�?(${formatFileSize(props.maxSize)})`
      emit('error', error.value)
      continue
    }

    // 检查文件类�?
    const extension = '.' + file.name.split('.').pop()?.toLowerCase()
    if (props.accept && !props.accept.split(',').some(format => {
      const trimmedFormat = format.trim()
      return trimmedFormat === extension || trimmedFormat === file.type
    })) {
      error.value = `文件 "${file.name}" 格式不支持`
      emit('error', error.value)
      continue
    }

    const fileItem: FileItem = {
      id: Date.now() + Math.random().toString(36).substr(2, 9),
      name: file.name,
      size: file.size,
      type: file.type,
      file
    }

    newFiles.push(fileItem)
  }

  if (newFiles.length > 0) {
    files.value = [...files.value, ...newFiles]
    emit('files-selected', newFiles)
  }
}

const removeFile = (fileId: string) => {
  const fileToRemove = files.value.find(f => f.id === fileId)
  if (fileToRemove) {
    files.value = files.value.filter(f => f.id !== fileId)
    emit('file-removed', fileId)
  }
}

const clearFiles = () => {
  files.value = []
}

const reset = () => {
  files.value = []
  error.value = ''
  isDragging.value = false
}

// 暴露方法给父组件
defineExpose({
  clearFiles,
  reset
})
</script>

<style scoped>
.file-dropzone {
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.05) 0%,
    rgba(255, 255, 255, 0.02) 100%
  );
}

.dark .file-dropzone {
  background: linear-gradient(
    135deg,
    rgba(0, 0, 0, 0.05) 0%,
    rgba(0, 0, 0, 0.02) 100%
  );
}
</style>
