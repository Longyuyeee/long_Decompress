<template>
  <div
    ref="dropzoneRef"
    :class="[
      'enhanced-file-dropzone',
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
    <!-- 加载状态 -->
    <div v-if="isLoading" class="flex flex-col items-center justify-center py-8">
      <i class="pi pi-spin pi-spinner text-primary text-3xl mb-4"></i>
      <p class="text-gray-700 dark:text-gray-300">正在处理文件...</p>
    </div>

    <!-- 正常状态 -->
    <template v-else>
      <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-primary/10 flex items-center justify-center">
        <i class="pi pi-cloud-upload text-primary text-2xl"></i>
      </div>

      <p class="text-gray-700 dark:text-gray-300 mb-2">
        <span v-if="isDragging">释放文件到此处</span>
        <span v-else>
          拖放文件到此处，或
          <button class="text-primary hover:underline font-medium" @click.stop="handleClick">
            点击选择文件
          </button>
        </span>
      </p>

      <p class="text-gray-500 dark:text-gray-400 text-sm">
        支持 {{ acceptedFormats.join(', ') }} 等格式
        <span v-if="maxSize">，最大 {{ formatFileSize(maxSize) }}</span>
      </p>

      <!-- 使用Tauri对话框按钮 -->
      <div class="mt-4">
        <GlassButton
          variant="secondary"
          size="sm"
          @click.stop="openTauriFileDialog"
          class="mx-auto"
        >
          <i class="pi pi-folder-open mr-2"></i>
          使用系统对话框选择
        </GlassButton>
      </div>

      <!-- 文件列表 -->
      <div v-if="files.length > 0" class="mt-6">
        <div class="flex items-center justify-between mb-3">
          <h4 class="font-medium text-gray-900 dark:text-white">已选择文件 ({{ files.length }})</h4>
          <button
            @click.stop="clearFiles"
            class="text-sm text-gray-500 hover:text-red-500 transition-colors"
            v-if="files.length > 0"
          >
            <i class="pi pi-trash mr-1"></i>
            清空
          </button>
        </div>
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
                <div class="flex items-center space-x-3">
                  <p class="text-sm text-gray-500 dark:text-gray-400">{{ formatFileSize(file.size) }}</p>
                  <span
                    v-if="file.format"
                    class="text-xs px-2 py-0.5 rounded-full bg-primary/10 text-primary"
                  >
                    {{ file.format.toUpperCase() }}
                  </span>
                  <span
                    v-if="file.encrypted"
                    class="text-xs px-2 py-0.5 rounded-full bg-warning/10 text-warning"
                  >
                    加密
                  </span>
                </div>
              </div>
            </div>
            <div class="flex items-center space-x-2">
              <button
                v-if="showPreview && file.previewUrl"
                @click.stop="previewFile(file)"
                class="text-gray-400 hover:text-primary transition-colors"
                :title="`预览 ${file.name}`"
              >
                <i class="pi pi-eye"></i>
              </button>
              <button
                @click.stop="removeFile(file.id)"
                class="text-gray-400 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-opacity"
                :title="`删除 ${file.name}`"
              >
                <i class="pi pi-times"></i>
              </button>
            </div>
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

      <!-- 成功提示 -->
      <div v-if="successMessage" class="mt-4 p-3 rounded-lg bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800">
        <div class="flex items-center">
          <i class="pi pi-check-circle text-green-500 mr-2"></i>
          <span class="text-green-700 dark:text-green-300 text-sm">{{ successMessage }}</span>
        </div>
      </div>

      <!-- 统计信息 -->
      <div v-if="files.length > 0" class="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
        <div class="flex justify-between text-sm text-gray-600 dark:text-gray-400">
          <span>文件总数: {{ files.length }}</span>
          <span>总大小: {{ formatFileSize(totalSize) }}</span>
        </div>
      </div>
    </template>

    <!-- 隐藏的文件输入（用于拖放和点击） -->
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
import { useTauriCommands } from '@/composables/useTauriCommands'
import GlassButton from './GlassButton.vue'

interface FileItem {
  id: string
  name: string
  size: number
  type: string
  path: string
  format?: string
  encrypted?: boolean
  previewUrl?: string
  file?: File // 仅用于拖放的文件
}

interface Props {
  multiple?: boolean
  accept?: string
  maxSize?: number // bytes
  maxFiles?: number
  showPreview?: boolean
  className?: string
  useTauriDialog?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  multiple: true,
  accept: '.zip,.rar,.7z,.tar,.gz,.bz2',
  maxSize: undefined,
  maxFiles: undefined,
  showPreview: true,
  className: '',
  useTauriDialog: true
})

const emit = defineEmits<{
  'files-selected': [files: FileItem[]]
  'file-removed': [fileId: string]
  'error': [error: string]
  'preview': [file: FileItem]
}>()

const dropzoneRef = ref<HTMLDivElement>()
const fileInputRef = ref<HTMLInputElement>()
const files = ref<FileItem[]>([])
const isDragging = ref(false)
const isLoading = ref(false)
const error = ref('')
const successMessage = ref('')

const tauriCommands = useTauriCommands()

const acceptedFormats = computed(() => {
  return props.accept.split(',').map(format => format.trim().replace('.', '').toUpperCase())
})

const totalSize = computed(() => {
  return files.value.reduce((sum, file) => sum + file.size, 0)
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
  if (props.useTauriDialog) {
    openTauriFileDialog()
  } else if (fileInputRef.value) {
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

  if (dropzoneRef.value && event.relatedTarget && !dropzoneRef.value.contains(event.relatedTarget as Node)) {
    isDragging.value = false
  }
}

const handleDrop = async (event: DragEvent) => {
  event.preventDefault()
  event.stopPropagation()
  isDragging.value = false

  if (!event.dataTransfer?.files.length) return

  const droppedFiles = Array.from(event.dataTransfer.files)
  await processDroppedFiles(droppedFiles)
}

const handleFileInput = async (event: Event) => {
  const input = event.target as HTMLInputElement
  if (!input.files?.length) return

  const selectedFiles = Array.from(input.files)
  await processDroppedFiles(selectedFiles)

  input.value = ''
}

const openTauriFileDialog = async () => {
  try {
    isLoading.value = true
    error.value = ''
    successMessage.value = ''

    const fileInfos = await tauriCommands.selectFiles(props.multiple, [
      {
        name: '压缩文件',
        extensions: ['zip', 'rar', '7z', 'tar', 'gz', 'bz2']
      },
      {
        name: '所有文件',
        extensions: ['*']
      }
    ])

    if (fileInfos.length === 0) {
      isLoading.value = false
      return
    }

    // 检查文件数量限制
    if (props.maxFiles && files.value.length + fileInfos.length > props.maxFiles) {
      error.value = `最多只能选择 ${props.maxFiles} 个文件`
      emit('error', error.value)
      isLoading.value = false
      return
    }

    const newFileItems: FileItem[] = []

    for (const fileInfo of fileInfos) {
      // 检查文件大小限制
      if (props.maxSize && fileInfo.size > props.maxSize) {
        error.value = `文件 "${fileInfo.name}" 超过最大大小限制 (${formatFileSize(props.maxSize)})`
        emit('error', error.value)
        continue
      }

      // 检查文件格式
      const formatCheck = await tauriCommands.checkFileFormat(fileInfo.path)

      const fileItem: FileItem = {
        id: Date.now() + Math.random().toString(36).substr(2, 9),
        name: fileInfo.name,
        size: fileInfo.size,
        type: getMimeType(fileInfo.name),
        path: fileInfo.path,
        format: formatCheck.format,
        encrypted: formatCheck.encrypted
      }

      newFileItems.push(fileItem)
    }

    if (newFileItems.length > 0) {
      files.value = [...files.value, ...newFileItems]
      emit('files-selected', newFileItems)
      successMessage.value = `成功添加 ${newFileItems.length} 个文件`
    }

  } catch (err) {
    error.value = `选择文件失败: ${err instanceof Error ? err.message : String(err)}`
    emit('error', error.value)
  } finally {
    isLoading.value = false
  }
}

const processDroppedFiles = async (fileList: File[]) => {
  isLoading.value = true
  error.value = ''
  successMessage.value = ''

  // 检查文件数量限制
  if (props.maxFiles && files.value.length + fileList.length > props.maxFiles) {
    error.value = `最多只能选择 ${props.maxFiles} 个文件`
    emit('error', error.value)
    isLoading.value = false
    return
  }

  const newFileItems: FileItem[] = []

  for (const file of fileList) {
    // 检查文件大小限制
    if (props.maxSize && file.size > props.maxSize) {
      error.value = `文件 "${file.name}" 超过最大大小限制 (${formatFileSize(props.maxSize)})`
      emit('error', error.value)
      continue
    }

    // 检查文件类型
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
      path: '', // 拖放的文件没有路径
      file // 保存File对象
    }

    newFileItems.push(fileItem)
  }

  if (newFileItems.length > 0) {
    files.value = [...files.value, ...newFileItems]
    emit('files-selected', newFileItems)
    successMessage.value = `成功添加 ${newFileItems.length} 个文件`
  }

  isLoading.value = false
}

const getMimeType = (filename: string): string => {
  const extension = filename.split('.').pop()?.toLowerCase()

  switch (extension) {
    case 'zip':
      return 'application/zip'
    case 'rar':
      return 'application/x-rar-compressed'
    case '7z':
      return 'application/x-7z-compressed'
    case 'tar':
      return 'application/x-tar'
    case 'gz':
      return 'application/gzip'
    case 'bz2':
      return 'application/x-bzip2'
    default:
      return 'application/octet-stream'
  }
}

const removeFile = (fileId: string) => {
  const fileToRemove = files.value.find(f => f.id === fileId)
  if (fileToRemove) {
    files.value = files.value.filter(f => f.id !== fileId)
    emit('file-removed', fileId)
    successMessage.value = `已删除文件: ${fileToRemove.name}`
  }
}

const clearFiles = () => {
  files.value = []
  successMessage.value = '已清空所有文件'
}

const previewFile = (file: FileItem) => {
  emit('preview', file)
}

const reset = () => {
  files.value = []
  error.value = ''
  successMessage.value = ''
  isDragging.value = false
  isLoading.value = false
}

const getFiles = () => {
  return files.value
}

// 暴露方法给父组件
defineExpose({
  clearFiles,
  reset,
  getFiles,
  openTauriFileDialog
})
</script>

<style scoped>
.enhanced-file-dropzone {
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.05) 0%,
    rgba(255, 255, 255, 0.02) 100%
  );
}

.dark .enhanced-file-dropzone {
  background: linear-gradient(
    135deg,
    rgba(0, 0, 0, 0.05) 0%,
    rgba(0, 0, 0, 0.02) 100%
  );
}
</style>