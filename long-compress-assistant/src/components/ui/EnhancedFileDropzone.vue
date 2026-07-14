<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { open } from '@tauri-apps/api/dialog'
import { listen } from '@tauri-apps/api/event'

const props = defineProps({
  compact: {
    type: Boolean,
    default: false
  },
  mode: {
    type: String,
    default: 'file', // 'file' | 'folder'
    validator: (value: string) => ['file', 'folder'].includes(value)
  },
  accept: {
    type: String,
    default: '*'
  },
  hint: {
    type: String,
    default: ''
  },
  subHint: {
    type: String,
    default: ''
  }
})

const appStore = useAppStore()
const isDragging = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)
const emit = defineEmits(['files-selected'])

// Tauri 原生拖放监听
let unlistenHover: any = null
let unlistenDrop: any = null
let unlistenCancel: any = null

onMounted(async () => {
  // 当文件悬停在窗口上时
  unlistenHover = await listen('tauri://file-drop-hover', () => {
    isDragging.value = true
  })

  // 当文件真正在窗口放下时
  unlistenDrop = await listen<string[]>('tauri://file-drop', (event) => {
    isDragging.value = false
    const paths = event.payload
    if (paths && paths.length > 0) {
      handleRawPaths(paths)
    }
  })

  // 当拖放取消或离开窗口时
  unlistenCancel = await listen('tauri://file-drop-cancelled', () => {
    isDragging.value = false
  })
})

onUnmounted(() => {
  if (unlistenHover) unlistenHover()
  if (unlistenDrop) unlistenDrop()
  if (unlistenCancel) unlistenCancel()
})

const displayHint = computed(() => {
  if (props.hint) return props.hint
  return props.mode === 'folder' 
    ? appStore.t('compress.drop_folder_hint') 
    : appStore.t('decompress.drop_hint')
})

const displaySubHint = computed(() => {
  if (props.subHint) return props.subHint
  return props.mode === 'folder'
    ? appStore.t('compress.drop_subhint')
    : 'ZIP, 7Z, RAR, TAR, GZ, BZ2, XZ, Zstd, ISO + 30 more'
})

const displayAddLabel = computed(() => {
  return props.mode === 'folder'
    ? appStore.t('compress.add_folders')
    : appStore.t('compress.add_files')
})

// 兼容标准的 Web 拖放（作为兜底）
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

const triggerFileInput = async () => {
  if (props.mode === 'folder') {
    try {
      const selected = await open({
        directory: true,
        multiple: true,
        title: appStore.t('compress.add_folders')
      })
      if (selected) handleRawPaths(Array.isArray(selected) ? selected : [selected])
    } catch (err) {
      console.error('Failed to select folders:', err)
    }
  } else {
    fileInput.value?.click()
  }
}

// 处理来自 Tauri 原生路径的数据
const handleRawPaths = (paths: string[]) => {
  const results = paths.map(path => {
    const name = path.split(/[\\/]/).filter(Boolean).pop() || path
    return {
      name,
      path,
      size: 0,
      type: props.mode === 'folder' ? 'directory' : 'file',
      isDirectory: props.mode === 'folder'
    }
  })
  emit('files-selected', results)
}

const handleFiles = (files: File[]) => {
  const missingPaths: string[] = []
  const fileData = files.map(file => {
    const hasPath = !!(file as any).path
    if (!hasPath) missingPaths.push(file.name)
    return {
      name: file.name,
      path: (file as any).path || '',
      size: file.size,
      type: file.type || 'file',
      isDirectory: false
    }
  }).filter(f => f.path) // 过滤掉没有路径的文件

  if (fileData.length > 0) {
    emit('files-selected', fileData)
  }
  if (missingPaths.length > 0) {
    appStore.setError(`部分文件无法添加 (缺少路径): ${missingPaths.slice(0, 3).join(', ')}${missingPaths.length > 3 ? '...' : ''}。请使用浏览按钮选择文件。`)
  }
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
    role="button"
    :aria-label="appStore.t('dropzone.hint')"
    tabindex="0"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="onDrop"
    @click="triggerFileInput"
    @keydown.enter="triggerFileInput"
    @keydown.space.prevent="triggerFileInput"
  >
    <input
      type="file"
      ref="fileInput"
      class="hidden"
      aria-hidden="true"
      multiple
      :accept="props.accept"
      @change="onFileChange"
    >
    
    <div v-if="!compact" class="flex flex-col items-center justify-center space-y-6">
      <div class="relative pointer-events-none">
        <div class="w-24 h-24 rounded-3xl bg-gradient-to-br from-primary/15 to-primary/5 border border-primary/20 flex items-center justify-center text-primary group-hover:scale-110 group-hover:shadow-xl group-hover:shadow-primary/25 transition-all duration-400">
          <i :class="props.mode === 'folder' ? 'pi pi-folder-open' : 'pi pi-cloud-upload'" class="text-4xl"></i>
        </div>
        <div class="absolute -top-2 -right-2 w-8 h-8 rounded-full bg-gradient-to-br from-primary to-primary/80 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-all duration-300 shadow-lg animate-pulse">
          <i class="pi pi-plus text-white text-sm"></i>
        </div>
      </div>

      <div class="text-center space-y-2 pointer-events-none">
        <p class="text-lg font-bold text-content tracking-tight">{{ displayHint }}</p>
        <p class="text-sm text-muted/90 font-medium tracking-wide leading-relaxed">{{ displaySubHint }}</p>
      </div>

      <div class="pt-3 pointer-events-none">
        <span class="inline-flex items-center gap-2.5 px-6 py-2.5 rounded-xl bg-gradient-to-r from-primary/15 to-primary/10 border border-primary/30 text-primary text-sm font-bold tracking-wide group-hover:from-primary group-hover:to-primary/90 group-hover:text-white group-hover:shadow-lg group-hover:scale-105 transition-all duration-300">
          <i class="pi pi-folder-open text-base"></i>
          {{ appStore.t('dropzone.browse') }}
        </span>
      </div>
    </div>

    <div v-else class="flex items-center justify-center gap-2.5">
      <i class="pi pi-plus text-primary text-sm pointer-events-none"></i>
      <span class="text-sm font-bold text-muted uppercase tracking-wider group-hover:text-content transition-colors duration-300 pointer-events-none">
        {{ displayAddLabel }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.drop-area {
  @apply relative border-2 border-dashed border-subtle cursor-pointer;
  background-color: transparent;
  transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}

.drop-area:hover {
  border-color: var(--dynamic-accent);
  background: radial-gradient(circle at center, color-mix(in srgb, var(--dynamic-accent) 5%, transparent) 0%, transparent 70%);
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.08);
}

.is-dragging {
  border-color: var(--dynamic-accent);
  background: radial-gradient(circle at center, color-mix(in srgb, var(--dynamic-accent) 12%, transparent) 0%, transparent 70%);
  transform: scale(1.02);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.12), 0 0 0 4px color-mix(in srgb, var(--dynamic-accent) 10%, transparent);
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}
</style>
