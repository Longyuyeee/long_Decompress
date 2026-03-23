<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { open } from '@tauri-apps/api/dialog'

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
    : 'ZIP, 7Z, RAR, TAR'
})

const displayAddLabel = computed(() => {
  return props.mode === 'folder'
    ? appStore.t('compress.add_folders')
    : appStore.t('compress.add_files')
})

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
      
      if (Array.isArray(selected)) {
        const folderData = selected.map(path => ({
          name: path.split(/[\\/]/).filter(Boolean).pop() || path,
          path: path,
          size: 0,
          type: 'directory',
          isDirectory: true
        }))
        emit('files-selected', folderData)
      } else if (typeof selected === 'string') {
        emit('files-selected', [{
          name: selected.split(/[\\/]/).filter(Boolean).pop() || selected,
          path: selected,
          size: 0,
          type: 'directory',
          isDirectory: true
        }])
      }
    } catch (err) {
      console.error('Failed to select folders:', err)
    }
  } else {
    fileInput.value?.click()
  }
}

const handleFiles = (files: File[]) => {
  if (props.mode === 'folder') {
    // 文件夹模式：必须确保只返回顶级文件夹作为单个对象
    const folderMap = new Map<string, any>()
    
    files.forEach(file => {
      const fullPath = (file as any).path || file.name
      const relativePath = (file as any).webkitRelativePath || ""
      
      let folderPath = fullPath
      let folderName = file.name

      // 如果是通过 webkitdirectory 选择的，relativePath 会包含 "Folder/file.txt"
      if (relativePath) {
        const rootFolderName = relativePath.split('/')[0]
        folderName = rootFolderName
        // 尝试回推根路径
        folderPath = fullPath.substring(0, fullPath.indexOf(relativePath) + rootFolderName.length)
      }

      if (!folderMap.has(folderPath)) {
        folderMap.set(folderPath, {
          name: folderName,
          path: folderPath,
          size: 0,
          type: 'directory',
          isDirectory: true
        })
      }
    })
    emit('files-selected', Array.from(folderMap.values()))
  } else {
    // 文件模式：正常处理文件实体
    const fileData = files.map(file => ({
      name: file.name,
      path: (file as any).path || file.name,
      size: file.size,
      type: file.type || 'file',
      isDirectory: false
    }))
    emit('files-selected', fileData)
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
      :accept="props.accept"
      :webkitdirectory="props.mode === 'folder'"
      :directory="props.mode === 'folder'"
      @change="onFileChange"
    >
    
    <div v-if="!compact" class="flex flex-col items-center justify-center space-y-4 pointer-events-none">
      <div class="w-16 h-16 rounded-[1.5rem] bg-input border border-subtle flex items-center justify-center text-dim group-hover:text-primary group-hover:scale-110 transition-all duration-500 shadow-sm">
        <i :class="props.mode === 'folder' ? 'pi pi-folder-open' : 'pi pi-cloud-upload'" class="text-2xl"></i>
      </div>
      
      <div class="text-center">
        <p class="text-xs font-black text-content tracking-tight mb-0.5">{{ displayHint }}</p>
        <p class="text-[8px] text-muted font-bold uppercase tracking-widest opacity-40">{{ displaySubHint }}</p>
      </div>

      <div class="pt-1">
        <span class="px-4 py-1.5 rounded-full bg-primary/10 border border-primary/20 text-primary text-[8px] font-black uppercase tracking-widest group-hover:bg-primary group-hover:text-white transition-all">
          Browse Files
        </span>
      </div>
    </div>

    <div v-else class="flex items-center justify-center gap-3 pointer-events-none">
      <i class="pi pi-plus-circle text-primary text-xs"></i>
      <span class="text-[9px] font-black text-muted uppercase tracking-[0.2em] group-hover:text-content transition-colors">
        {{ displayAddLabel }}
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
