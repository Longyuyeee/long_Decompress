<template>
  <div class="p-6">
    <EnhancedFileDropzone @files-selected="handleFiles" />
    <div v-if="files.length > 0" class="mt-4">
      <p>已选择 {{ files.length }} 个文件</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import EnhancedFileDropzone from './EnhancedFileDropzone.vue'

const files = ref<any[]>([])
const handleFiles = (selected: any[]) => {
  files.value = selected
}
</script>
