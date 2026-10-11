<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import Modal from '@/components/ui/Modal.vue'
import { formatFileSize } from '@/utils'
import type { PublishedArchiveOutput } from '@/types/archiveOutput'
import { archiveOutputLocationError } from '@/utils/archiveOutputError'
const props = defineProps<{ output: PublishedArchiveOutput }>()
const visible = ref(false)
const busy = ref(false)
const error = ref('')
const copied = ref(false)
watch(() => props.output, () => { error.value = ''; copied.value = false })
async function copyPath() {
  try { await navigator.clipboard.writeText(props.output.primaryPath); copied.value = true }
  catch { error.value = '复制失败，可选中上方路径手动复制。' }
}
async function locate() {
  if (busy.value) return
  busy.value = true
  error.value = ''
  try { await invoke('open_in_explorer', { path: props.output.primaryPath }) }
  catch (caught) { error.value = archiveOutputLocationError(caught) }
  finally { busy.value = false }
}
</script>
<template>
  <button class="mt-2 shrink-0 rounded-lg border border-subtle px-3 py-2 text-xs text-primary text-left" type="button" @click="visible = true">
    查看产物 · {{ output.files.length }} {{ output.split ? '卷' : '个文件' }} · {{ formatFileSize(output.totalBytes) }}
  </button>
  <Modal :visible="visible" @update:visible="visible = $event" title="压缩产物" cancel-text="关闭产物清单" layer="nested">
    <div class="space-y-3 min-w-0" data-testid="archive-output-summary">
      <p class="text-sm">{{ output.split ? '首卷' : '输出文件' }}</p>
      <code class="block text-xs break-all select-text">{{ output.primaryPath }}</code>
      <button type="button" :disabled="busy" class="rounded-lg bg-primary px-3 py-2 text-white text-sm" @click="locate">{{ busy ? '正在定位…' : output.split ? '定位首卷' : '定位文件' }}</button>
      <button type="button" class="ml-2 rounded-lg border border-subtle px-3 py-2 text-sm" @click="copyPath">{{ copied ? '已复制' : '复制路径' }}</button>
      <p v-if="error" role="alert" class="text-sm text-red-500">{{ error }}</p>
      <p class="text-xs text-muted">共 {{ output.files.length }} {{ output.split ? '卷' : '个文件' }} · {{ formatFileSize(output.totalBytes) }} · 完成时记录</p>
      <ol class="space-y-2 text-xs">
        <li v-for="file in output.files" :key="file.path" class="border-t border-subtle pt-2">
          <code class="block break-all select-text">{{ file.path }}</code>
          <span class="text-muted">{{ formatFileSize(file.bytes) }}</span>
        </li>
      </ol>
    </div>
  </Modal>
</template>
