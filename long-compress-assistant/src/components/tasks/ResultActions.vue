<script setup lang="ts">
import { useArchiveLibraryStore } from '@/stores/archiveLibrary'
import { isDecompressArchivePath, isPotentialSplitArchivePath } from '@/utils/compressionFormat'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { archiveOutputLocationError } from '@/utils/archiveOutputError'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { checksumSources } from '@/utils/checksumSources'
import type { PublishedArchiveOutput } from '@/types/archiveOutput'
const props = withDefaults(defineProps<{ path: string; directory?: boolean; split?: boolean; showPath?: boolean; output?: PublishedArchiveOutput | null }>(), { directory: false, split: false, showPath: false })
const router = useRouter(), app = useAppStore()
const emit = defineEmits<{ checksum: [] }>()
const busy = ref(false)
const preparingChecksum = ref(false)
const error = ref('')
const copied = ref(false)
const label = computed(() => props.directory ? '打开输出目录' : props.split ? '定位首卷' : '打开结果位置')
let request = 0
onBeforeUnmount(() => { request++ })
watch(() => [props.path, props.directory, props.output], () => { request++; busy.value = false; preparingChecksum.value = false; error.value = ''; copied.value = false })
async function openOutput() {
  if (busy.value || !props.path.trim()) return
  const token = ++request
  const path = props.path
  const directory = props.directory
  busy.value = true
  error.value = ''
  try {
    if (directory) {
      const state = await invoke<string>('inspect_extraction_destination', { path })
      if (token !== request) return
      if (state === 'missing' || state === 'file') {
        error.value = state === 'missing' ? '找不到输出目录，可能已被移动或删除。可以复制原路径查找，或从历史记录再次解压。' : '原输出位置现在是一个文件。可以复制路径查找，或从历史记录再次解压。'
        return
      }
      if (state !== 'empty' && state !== 'occupied') { error.value = '暂时无法访问输出目录，请检查磁盘或网络连接后重试。'; return }
    }
    if (token === request) await invoke('open_in_explorer', { path })
  } catch (caught) {
    if (token === request) error.value = directory ? '无法打开输出目录，请重试或复制路径手动打开。' : archiveOutputLocationError(caught)
  } finally { if (token === request) busy.value = false }
}
async function copyPath() {
  const path = props.path
  const token = request
  error.value = ''
  try { await navigator.clipboard.writeText(path); if (token === request) copied.value = true }
  catch { if (token === request) error.value = '复制失败，可以选中路径手动复制。' }
}
async function addToLibrary(){
  if(busy.value)return
  busy.value=true;error.value=''
  try{const library=useArchiveLibraryStore();if(await library.add([props.path]))await router.push('/library');else error.value=library.error}finally{busy.value=false}
}
async function generateManifest() {
  if (busy.value || !props.path.trim()) return
  const token = ++request
  busy.value = true; preparingChecksum.value = true; error.value = ''
  try {
    const paths = await checksumSources(props.path, props.output)
    if (token !== request) return
    if (!paths.length) throw new Error('没有可计算的结果文件。')
    app.pendingChecksumSources = { paths, label: props.path.split(/[\\/]/).pop() || props.path }
    await router.push('/integrity')
    emit('checksum')
  } catch (cause) { if (token === request) error.value = `无法带入结果：${cause instanceof Error ? cause.message : String(cause)}` }
  finally { if (token === request) { busy.value = false; preparingChecksum.value = false } }
}
</script>
<template>
  <div class="min-w-0 space-y-2" data-testid="result-actions">
    <p v-if="showPath" class="select-text break-words text-xs leading-relaxed text-muted [overflow-wrap:anywhere]" :title="path">{{ path || '未记录输出位置' }}</p>
    <div class="flex flex-wrap items-center gap-2">
      <button type="button" class="result-button bg-primary text-white hover:brightness-110" :disabled="busy || !path.trim()" @click.stop="openOutput"><i class="pi pi-folder-open" aria-hidden="true"></i>{{ busy && !preparingChecksum ? '正在打开…' : label }}</button>
      <button type="button" class="result-button border border-subtle text-muted hover:bg-input hover:text-content" :disabled="!path.trim()" @click.stop="copyPath"><i :class="copied ? 'pi pi-check' : 'pi pi-copy'" aria-hidden="true"></i>{{ copied ? '已复制' : '复制路径' }}</button>
      <button v-if="!directory && (isDecompressArchivePath(path) || isPotentialSplitArchivePath(path))" type="button" data-testid="result-add-library" class="result-button border border-subtle text-primary" :disabled="busy || !path.trim()" @click.stop="addToLibrary"><i class="pi pi-database"/>加入 / 刷新资料库</button>
      <button v-if="!directory" type="button" data-testid="result-generate-checksums" class="result-button border border-subtle text-muted hover:bg-input hover:text-content" :disabled="busy || !path.trim()" @click.stop="generateManifest"><i class="pi pi-hashtag" aria-hidden="true"></i>{{ preparingChecksum ? '正在带入…' : '生成校验清单' }}</button>
    </div>
    <p v-if="error" role="alert" class="break-words rounded-lg border border-subtle bg-input/50 px-3 py-2 text-xs leading-relaxed text-content">{{ error }}</p>
  </div>
</template>
<style scoped>
.result-button { display:inline-flex; min-height:2rem; align-items:center; gap:.4rem; border-radius:.6rem; padding:.4rem .7rem; font-size:.75rem; transition:background .15s; }
.result-button:disabled { opacity:.5; }
.result-button:focus-visible { outline:2px solid var(--dynamic-accent); outline-offset:2px; }
</style>
