<script setup lang="ts">
import { onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useSavedLocations } from '@/composables/useSavedLocations'

const props = defineProps<{ current?: string; disabled?: boolean }>()
const emit = defineEmits<{ select: [path: string] }>()
const locations = useSavedLocations()
const busy = ref(false)
const error = ref('')
let generation = 0
watch(() => [props.current, props.disabled], () => { generation++; busy.value = false; error.value = '' }, { flush: 'sync' })
onUnmounted(() => { generation++ })
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
function choose(event: Event) {
  const target = event.target as HTMLSelectElement
  const path = target.value
  target.value = ''
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
      <select aria-label="选择书签或最近输出目录" data-testid="saved-output-select" :disabled="disabled || busy"
        class="min-w-0 max-w-full rounded-lg border border-subtle bg-input px-2 py-1 text-xs text-content"
        @focus="locations.refresh" @pointerdown="locations.refresh" @change="choose">
        <option value="">{{ busy ? '正在检查目录…' : '书签 / 最近目录' }}</option>
        <optgroup label="书签"><option v-for="path in locations.bookmarks.value" :key="path" :value="path">{{ path }}</option></optgroup>
        <optgroup label="最近目录"><option v-for="path in locations.recent.value" :key="path" :value="path">{{ path }}</option></optgroup>
      </select>
      <button type="button" data-testid="bookmark-output" :disabled="disabled || busy || !current"
        :aria-label="current && locations.isBookmarked(current) ? '取消输出目录书签' : '收藏输出目录'"
        class="shrink-0 rounded-lg px-2 py-1 text-primary disabled:opacity-40" @click="toggleCurrent">
        <i :class="current && locations.isBookmarked(current) ? 'pi pi-star-fill' : 'pi pi-star'"></i>
      </button>
    </div>
    <p v-if="error || locations.error.value" role="status" class="mt-1 text-xs text-amber-600 break-words">{{ error || locations.error.value }}</p>
  </div>
</template>

<style scoped>
.saved-output-location { min-width: 0; max-width: 100%; }
select { width: 12rem; }
</style>
