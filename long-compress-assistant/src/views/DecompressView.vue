<template>
  <div class="decompress-view p-6 max-w-4xl mx-auto">
    <h1 class="text-2xl font-bold mb-6">解压文件</h1>
    <EnhancedFileDropzone @files-selected="onFilesSelected" />
    
    <div v-if="selectedFiles.length > 0" class="mt-8 space-y-4">
      <div v-for="file in selectedFiles" :key="(file as any).path" class="flex items-center justify-between p-3 glass-card">
        <span>{{ (file as any).name }}</span>
        <button @click="removeFile((file as any).path)" class="text-red-500">移除</button>
      </div>
      
      <button @click="startDecompress" class="glass-button-primary w-full py-3 mt-4">开始解压</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import { useDecompressionStore } from '@/stores'

const selectedFiles = ref<any[]>([])
const decompressionStore = useDecompressionStore()

const onFilesSelected = (files: any[]) => {
  selectedFiles.value = files
}

const removeFile = (path: string) => {
  selectedFiles.value = selectedFiles.value.filter(f => f.path !== path)
}

const startDecompress = () => {
  alert('开始解压逻辑已触发')
}
</script>
