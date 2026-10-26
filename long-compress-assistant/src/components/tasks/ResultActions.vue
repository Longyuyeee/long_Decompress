<script setup lang="ts">
import ZpaqRetrievalResult from '@/components/archive/ZpaqRetrievalResult.vue'
import { useArchiveLibraryStore } from '@/stores/archiveLibrary'
import { isDecompressArchivePath, isPotentialSplitArchivePath } from '@/utils/compressionFormat'
import { computed, nextTick, onBeforeUnmount, onDeactivated, ref, useId, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { archiveOutputLocationError } from '@/utils/archiveOutputError'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { checksumSources } from '@/utils/checksumSources'
import type { PublishedArchiveOutput } from '@/types/archiveOutput'
const props = withDefaults(defineProps<{ path: string; retrieval?: import('@/types/zpaqRetrieval').ZpaqRetrievalSelection; directory?: boolean; split?: boolean; showPath?: boolean; compact?: boolean; available?: boolean; output?: PublishedArchiveOutput | null }>(), { directory: false, split: false, showPath: false, available: true })
const router = useRouter(), app = useAppStore()
const emit = defineEmits<{ checksum: [] }>()
const busy = ref(false)
const preparingChecksum = ref(false)
const error = ref('')
const copied = ref(false)
const moreOpen = ref(false)
const menuId = useId()
const moreButton = ref<HTMLButtonElement>()
const menu = ref<HTMLElement>()
const menuPosition = ref({ left: '0px', top: '0px' })
const label = computed(() => props.directory ? '打开输出目录' : props.split ? '定位首卷' : '打开结果位置')
function closeMenu(restoreFocus = false) {
  moreOpen.value = false
  if (restoreFocus) moreButton.value?.focus({ preventScroll: true })
}
async function showMenu(last = false) {
  moreOpen.value = true
  await nextTick()
  if (!moreOpen.value || !moreButton.value || !menu.value) return
  const anchor = moreButton.value.getBoundingClientRect()
  const panel = menu.value.getBoundingClientRect()
  menuPosition.value = {
    left: `${Math.max(8, Math.min(anchor.left, window.innerWidth - panel.width - 8))}px`,
    top: `${Math.max(8, anchor.bottom + panel.height + 6 <= window.innerHeight - 8 ? anchor.bottom + 6 : anchor.top - panel.height - 6)}px`,
  }
  const items = menu.value.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')
  items[last ? items.length - 1 : 0]?.focus({ preventScroll: true })
}
function outsideMenu(event: Event) {
  if (!moreButton.value?.contains(event.target as Node) && !menu.value?.contains(event.target as Node)) closeMenu()
}
function dismissMenu(event: Event) {
  if (event.type === 'scroll' && menu.value?.contains(event.target as Node)) return
  closeMenu()
}
function menuKeydown(event: KeyboardEvent) {
  if (!props.compact) return
  if (event.key === 'Escape') { event.preventDefault(); closeMenu(true); return }
  if (event.key === 'Tab') { closeMenu(true); return }
  if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  const items = Array.from(menu.value?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') || [])
  const current = items.indexOf(document.activeElement as HTMLButtonElement)
  const index = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1
    : (current + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length
  items[index]?.focus({ preventScroll: true })
}
watch(moreOpen, open => {
  const method = open ? 'addEventListener' : 'removeEventListener'
  document[method]('pointerdown', outsideMenu)
  document[method]('focusin', outsideMenu)
  window[method]('resize', dismissMenu)
  window[method]('scroll', dismissMenu, true)
}, { flush: 'sync' })
onDeactivated(() => closeMenu())
let request = 0
onBeforeUnmount(() => { request++; closeMenu() })
watch(() => [props.path, props.directory, props.output, props.available, props.compact], () => { request++; busy.value = false; preparingChecksum.value = false; error.value = ''; copied.value = false; closeMenu() })
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
  closeMenu(true)
  const path = props.path
  const token = request
  error.value = ''
  try { await navigator.clipboard.writeText(path); if (token === request) copied.value = true }
  catch { if (token === request) error.value = '复制失败，可以选中路径手动复制。' }
}
async function addToLibrary(){
  closeMenu(true)
  if(busy.value)return
  busy.value=true;error.value=''
  try{const library=useArchiveLibraryStore();if(await library.add([props.path]))await router.push('/library');else error.value=library.error}finally{busy.value=false}
}
async function generateManifest() {
  closeMenu(true)
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
    <div class="result-toolbar flex items-center gap-2" :class="compact ? 'flex-nowrap' : 'flex-wrap'" @keydown.enter.stop>
      <template v-if="available">
      <ZpaqRetrievalResult v-if="retrieval" :path="path" :selection="retrieval"/>
      <button type="button" class="result-button bg-primary text-white hover:brightness-110" :disabled="busy || !path.trim()" @click.stop="openOutput"><i class="pi pi-folder-open" aria-hidden="true"></i>{{ busy && !preparingChecksum ? '正在打开…' : label }}</button>
      </template>
      <slot name="primary" />
      <template v-if="available">
        <Teleport to="body" :disabled="!compact">
          <div v-if="!compact || moreOpen" :id="compact ? menuId : undefined" ref="menu" :role="compact ? 'menu' : undefined" :aria-label="compact ? '更多结果操作' : undefined" :class="compact ? 'result-menu' : 'contents'" :style="compact ? menuPosition : undefined" @click.stop @keydown.stop="menuKeydown">
            <button type="button" :role="compact ? 'menuitem' : undefined" class="result-button border border-subtle text-muted hover:bg-input hover:text-content" :disabled="!path.trim()" @click.stop="copyPath"><i :class="copied ? 'pi pi-check' : 'pi pi-copy'" aria-hidden="true"></i>{{ copied ? '已复制' : '复制路径' }}</button>
            <button v-if="!directory && (isDecompressArchivePath(path) || isPotentialSplitArchivePath(path))" type="button" :role="compact ? 'menuitem' : undefined" data-testid="result-add-library" class="result-button border border-subtle text-primary" :disabled="busy || !path.trim()" @click.stop="addToLibrary"><i class="pi pi-database"/>加入 / 刷新资料库</button>
            <button v-if="!directory" type="button" :role="compact ? 'menuitem' : undefined" data-testid="result-generate-checksums" class="result-button border border-subtle text-muted hover:bg-input hover:text-content" :disabled="busy || !path.trim()" @click.stop="generateManifest"><i class="pi pi-hashtag" aria-hidden="true"></i>{{ preparingChecksum ? '正在带入…' : '生成校验清单' }}</button>
          </div>
        </Teleport>
        <button v-if="compact" ref="moreButton" type="button" class="result-button border border-subtle text-muted" aria-label="更多结果操作" title="更多结果操作" aria-haspopup="menu" :aria-controls="moreOpen ? menuId : undefined" :aria-expanded="moreOpen" @click.stop="moreOpen ? closeMenu() : showMenu()" @keydown.down.stop.prevent="showMenu()" @keydown.up.stop.prevent="showMenu(true)"><i class="pi pi-ellipsis-h"/></button>
      </template>
    </div>
    <p v-if="error" role="alert" class="break-words rounded-lg border border-subtle bg-input/50 px-3 py-2 text-xs leading-relaxed text-content">{{ error }}</p>
  </div>
</template>
<style scoped>
.result-button { display:inline-flex; flex-shrink:0; min-height:2rem; align-items:center; gap:.4rem; border-radius:.6rem; padding:.4rem .7rem; font-size:.75rem; white-space:nowrap; transition:background .15s; }
.result-button:disabled { opacity:.5; }
.result-button:focus-visible { outline:2px solid var(--dynamic-accent); outline-offset:2px; }
.result-menu { position:fixed; z-index:10000; width:15rem; max-width:calc(100vw - 16px); max-height:calc(100vh - 16px); overflow:auto; padding:.35rem; border:1px solid var(--border-subtle); border-radius:.8rem; background:var(--bg-modal); color:var(--text-base); box-shadow:0 12px 36px #0003; }
.result-menu .result-button { display:flex; width:100%; border:0; padding:.6rem .7rem; text-align:left; }
.result-menu .result-button:hover:not(:disabled),.result-menu .result-button:focus-visible { background:color-mix(in srgb,var(--dynamic-accent) 10%,var(--bg-input)); outline-offset:-2px; }
</style>
