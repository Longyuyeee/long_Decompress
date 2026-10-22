<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import Modal from '@/components/ui/Modal.vue'
import type { LocalFileEntry } from './DualPaneFileManager.vue'
import type { ArchiveImagePreview, ArchiveTextPreview } from '@/composables/useTauriCommands'
import { localPreviewKind } from '@/utils/filePreview'

const props = defineProps<{ entry: LocalFileEntry }>()
const emit = defineEmits<{ close: [] }>()
const image = ref<ArchiveImagePreview>(), text = ref<ArchiveTextPreview>(), error = ref(''), loading = ref(true)
let disposed = false
onMounted(async () => {
  try {
    if (localPreviewKind(props.entry.name) === 'image') {
      const result = await invoke<ArchiveImagePreview>('file_manager_preview_image', { path: props.entry.path })
      if (!disposed) image.value = result
    } else {
      const result = await invoke<ArchiveTextPreview>('file_manager_preview_text', { path: props.entry.path })
      if (!disposed) text.value = result
    }
  } catch (cause) { if (!disposed) error.value = String(cause).replace(/^Error:\s*/, '') }
  finally { if (!disposed) loading.value = false }
})
onUnmounted(() => { disposed = true })
const locate = () => invoke('open_in_explorer', { path: props.entry.path }).catch(() => { error.value = '无法定位此文件，它可能已被移动。' })
const bytes = (value: number) => value < 1024 ? `${value} B` : value < 1048576 ? `${(value / 1024).toFixed(1)} KB` : `${(value / 1048576).toFixed(1)} MB`
</script>

<template>
  <Modal :visible="true" :title="entry.name" :icon="localPreviewKind(entry.name)==='image'?'pi pi-image':'pi pi-file'" size="xl" @update:visible="emit('close')">
    <div class="local-preview" data-testid="local-file-preview">
      <p class="preview-location" :title="entry.path">{{ entry.path }}</p>
      <div v-if="loading" class="preview-placeholder"><i class="pi pi-spin pi-spinner"/><p>正在读取预览…</p></div>
      <div v-else-if="error" class="preview-placeholder" role="status"><i class="pi pi-info-circle"/><p>{{ error }}</p></div>
      <img v-else-if="image" data-testid="local-image-preview" :src="image.dataUrl" :alt="entry.name">
      <pre v-else-if="text" class="preview-text custom-scrollbar" data-testid="local-text-preview">{{ text.content || '（空文本文件）' }}</pre>
      <div v-if="text || image" class="preview-meta"><template v-if="text"><span>{{ text.encoding }}</span><span>{{ text.lineCount }} 行</span><span>{{ text.truncated ? '显示前 1 MB' : '完整显示' }} · {{ bytes(text.totalSize) }}</span></template><template v-else-if="image"><span>{{ image.width }} × {{ image.height }}</span><span>{{ bytes(image.byteSize) }}</span></template><span>只读预览</span></div>
    </div>
    <template #footer><button class="preview-button" @click="locate"><i class="pi pi-external-link"/>在资源管理器中定位</button><button class="preview-button primary" data-testid="local-preview-close" @click="emit('close')">关闭</button></template>
  </Modal>
</template>

<style scoped>
.local-preview{min-width:0}.preview-location{font-size:11px;color:var(--text-muted);overflow-wrap:anywhere;line-height:1.7;margin-bottom:14px}.local-preview>img{display:block;max-width:100%;max-height:48vh;object-fit:contain;margin:auto;border-radius:12px}.preview-text{white-space:pre-wrap;overflow-wrap:anywhere;tab-size:2;min-height:180px;max-height:46vh;overflow:auto;font-size:12px;line-height:1.8;padding:16px;border:1px solid var(--border-subtle);border-radius:12px;background:var(--bg-input);font-family:ui-monospace,Consolas,monospace}.preview-placeholder{display:grid;place-content:center;justify-items:center;gap:14px;min-height:180px;text-align:center;color:var(--text-muted);font-size:12px;line-height:1.8;overflow-wrap:anywhere}.preview-placeholder>i{font-size:26px;color:var(--dynamic-accent)}.preview-meta{display:flex;gap:8px;flex-wrap:wrap;margin-top:14px;font-size:10px;color:var(--text-muted)}.preview-meta>span{padding:4px 8px;border-radius:6px;background:var(--bg-input)}.preview-button{display:inline-flex;align-items:center;justify-content:center;gap:7px;font-size:11px;padding:9px 12px;border-radius:10px;border:1px solid var(--border-subtle);background:var(--bg-input);color:var(--text-content)}.preview-button.primary{background:var(--dynamic-accent);color:white}@media(max-height:600px){.preview-text{min-height:100px}.preview-placeholder{min-height:100px}}
</style>
