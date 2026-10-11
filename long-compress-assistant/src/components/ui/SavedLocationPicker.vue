<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, useId, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useSavedLocations } from '@/composables/useSavedLocations'

const props = defineProps<{ current?: string; disabled?: boolean }>()
const emit = defineEmits<{ select: [path: string] }>()
const locations = useSavedLocations()
const busy = ref(false)
const error = ref('')
const open = ref(false)
const trigger = ref<HTMLButtonElement>()
const menu = ref<HTMLElement>()
const menuId = useId()
const position = ref<Record<string, string>>({})
const groups = computed(() => [
  { label: '收藏目录', icon: 'pi-star', paths: locations.bookmarks.value },
  { label: '最近使用', icon: 'pi-history', paths: locations.recent.value },
])
const folderName = (path: string) => path.replace(/[\\/]+$/, '').split(/[\\/]/).pop() || path
function close(restore = false) {
  open.value = false
  if (restore) trigger.value?.focus({ preventScroll: true })
}
function placeMenu() {
  if (!trigger.value) return
  const rect = trigger.value!.getBoundingClientRect()
  const below = window.innerHeight - rect.bottom - 12
  const above = rect.top - 12
  const upward = below < 240 && above > below
  position.value = {
    left: `${Math.max(8, Math.min(rect.left, window.innerWidth - Math.min(352, window.innerWidth - 16) - 8))}px`,
    maxHeight: `${Math.min(320, Math.max(40, upward ? above : below))}px`,
    ...(upward ? { bottom: `${window.innerHeight - rect.top + 6}px` } : { top: `${rect.bottom + 6}px` }),
  }
}
async function show(last = false) {
  if (props.disabled || busy.value) return
  locations.refresh()
  placeMenu()
  open.value = true
  await nextTick()
  if (!open.value) return
  const items = menu.value?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')
  if (items?.length) items[last ? items.length - 1 : 0].focus({ preventScroll: true })
  else menu.value?.focus({ preventScroll: true })
}
function menuKey(event: KeyboardEvent) {
  if (event.key === 'Escape') { event.preventDefault(); close(true); return }
  if (event.key === 'Tab') { close(true); return }
  const items = Array.from(menu.value?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') || [])
  if (!items.length || !['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  const index = items.indexOf(document.activeElement as HTMLButtonElement)
  const next = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length
  items[next].focus()
}
function outside(event: Event) {
  if (!trigger.value?.contains(event.target as Node) && !menu.value?.contains(event.target as Node)) close()
}
function dismiss(event: Event) {
  if (event.type === 'scroll' && menu.value?.contains(event.target as Node)) return
  if (open.value) placeMenu()
}
onMounted(() => {
  document.addEventListener('pointerdown', outside)
  document.addEventListener('focusin', outside)
  window.addEventListener('resize', dismiss)
  window.addEventListener('scroll', dismiss, true)
})
let generation = 0
watch(() => [props.current, props.disabled], () => { generation++; busy.value = false; error.value = ''; close() }, { flush: 'sync' })
onUnmounted(() => {
  generation++
  document.removeEventListener('pointerdown', outside)
  document.removeEventListener('focusin', outside)
  window.removeEventListener('resize', dismiss)
  window.removeEventListener('scroll', dismiss, true)
})
async function usePath(path: string, bookmark = false) {
  if (!path || props.disabled || busy.value) return
  const request = ++generation
  busy.value = true
  error.value = ''
  try {
    const state = await invoke<string>('inspect_extraction_destination', { path })
    if (request !== generation) return
    if (!['empty', 'occupied'].includes(state)) {
      error.value = state === 'file' ? '该位置是文件，请重新选择目录。' : '该目录已不存在或暂时无法访问；保留原输出位置。'
      return
    }
    if (bookmark) locations.toggleBookmark(path)
    else { locations.remember(path); emit('select', path) }
  } catch { if (request === generation) error.value = '无法检查此目录，请确认磁盘或网络连接；保留原输出位置。' }
  finally { if (request === generation) busy.value = false }
}
function choose(path: string) {
  close(true)
  void usePath(path)
}
function toggleCurrent() {
  locations.refresh()
  if (!props.current) return
  if (locations.isBookmarked(props.current)) locations.toggleBookmark(props.current)
  else void usePath(props.current, true)
}
</script>

<template>
  <div class="saved-output-location" data-testid="saved-output-location" @click.stop @keydown.stop>
    <div class="flex min-w-0 items-center gap-1">
      <button ref="trigger" type="button" aria-label="常用目录" data-testid="saved-output-select" :disabled="disabled || busy"
        class="location-trigger" aria-haspopup="menu" :aria-expanded="open" :aria-controls="open ? menuId : undefined"
        @click="open ? close() : show()" @keydown.down.prevent="show()" @keydown.up.prevent="show(true)">
        <i class="pi pi-folder" aria-hidden="true"></i><span>{{ busy ? '正在检查目录…' : '常用目录' }}</span><i class="pi pi-chevron-down" aria-hidden="true"></i>
      </button>
      <button type="button" data-testid="bookmark-output" :disabled="disabled || busy || !current"
        :aria-label="current && locations.isBookmarked(current) ? '取消收藏当前目录' : '收藏当前目录'"
        :title="current && locations.isBookmarked(current) ? '取消收藏当前目录' : '收藏当前目录'"
        :aria-pressed="!!current && locations.isBookmarked(current)"
        class="shrink-0 rounded-lg px-2 py-1 text-primary disabled:opacity-40" @click="toggleCurrent">
        <i :class="current && locations.isBookmarked(current) ? 'pi pi-star-fill' : 'pi pi-star'"></i>
      </button>
    </div>
    <p v-if="error || locations.error.value" role="status" class="mt-1 text-xs text-amber-600 break-words">{{ error || locations.error.value }}</p>
    <Teleport to="body">
      <div v-if="open" :id="menuId" ref="menu" role="menu" aria-label="常用目录" tabindex="-1" class="location-menu" :style="position" @keydown.stop="menuKey" @click.stop>
        <section v-for="group in groups" :key="group.label" role="group" :aria-label="group.label">
          <h5><i :class="['pi', group.icon]" aria-hidden="true"></i>{{ group.label }}</h5>
          <p v-if="!group.paths.length" class="location-empty">{{ group.label === '收藏目录' ? '点击星标，收藏当前目录' : '使用过的目录会显示在这里' }}</p>
          <button v-for="path in group.paths" :key="path" type="button" role="menuitem" tabindex="-1" :title="path" :aria-label="path" @click="choose(path)">
            <i class="pi pi-folder" aria-hidden="true"></i><span><strong>{{ folderName(path) }}</strong><small>{{ path }}</small></span>
          </button>
        </section>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.saved-output-location { min-width: 0; max-width: 100%; }
.location-trigger { display:flex; align-items:center; gap:.5rem; max-width:100%; min-width:0; padding:.4rem .65rem; border:1px solid var(--border-subtle); border-radius:.6rem; background:var(--bg-input); color:var(--text-content); font-size:.75rem; font-weight:700; }
.location-trigger span { overflow:hidden; white-space:nowrap; text-overflow:ellipsis; }
.location-trigger .pi { color:var(--dynamic-accent); font-size:.7rem; }
.location-trigger:disabled { opacity:.5; }
.location-trigger:focus-visible,.location-menu button:focus-visible { outline:2px solid var(--dynamic-accent); outline-offset:-2px; }
.location-menu { position:fixed; z-index:10000; width:22rem; max-width:calc(100vw - 16px); box-sizing:border-box; overflow:auto; padding:.4rem; border:1px solid var(--border-subtle); border-radius:.8rem; background:var(--bg-card); box-shadow:0 12px 35px #0003; overscroll-behavior:contain; }
.location-menu section + section { border-top:1px solid var(--border-subtle); margin-top:.35rem; padding-top:.25rem; }
.location-menu h5 { display:flex; gap:.4rem; align-items:center; padding:.35rem .5rem; color:var(--text-muted); font-size:.7rem; font-weight:700; }
.location-menu button { display:flex; align-items:center; gap:.65rem; width:100%; padding:.5rem; border-radius:.5rem; text-align:left; color:var(--text-content); }
.location-menu button:hover,.location-menu button:focus { background:var(--bg-input); }
.location-menu button > i { color:var(--dynamic-accent); }
.location-menu button span { min-width:0; }
.location-menu strong,.location-menu small { display:block; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.location-menu strong { font-size:.8rem; }
.location-menu small,.location-empty { font-size:.7rem; color:var(--text-muted); }
.location-empty { padding:.4rem .5rem; }
</style>
