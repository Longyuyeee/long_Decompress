<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { uniqueResultDirectories, uniqueResultPaths, type ResultLocation } from '@/utils/resultLocation'
const props = defineProps<{ locations: ResultLocation[] }>()
const directories = computed(() => uniqueResultDirectories(props.locations))
const paths = computed(() => uniqueResultPaths(props.locations))
const busy = ref(false)
const feedback = ref('')
const unavailable = ref<string[]>([])
let generation = 0
watch(paths, () => { generation++; busy.value = false; feedback.value = ''; unavailable.value = [] })
onBeforeUnmount(() => { generation++ })
async function openDirectories(retryUnavailable = false) {
  if (busy.value) return
  const token = ++generation
  const pending = [...(retryUnavailable ? unavailable.value : directories.value)]
  const failed: string[] = []
  busy.value = true
  feedback.value = ''
  let opened = 0
  try {
    for (const path of pending) {
      if (token !== generation) return
      try {
        const state = await invoke<string>('inspect_extraction_destination', { path })
        if (token !== generation) return
        if (state !== 'empty' && state !== 'occupied') { failed.push(path); continue }
        await invoke('open_in_explorer', { path })
        opened++
      } catch { failed.push(path) }
    }
    if (token === generation) { unavailable.value = failed; feedback.value = opened === pending.length ? `已打开 ${opened} 个目录。` : `已打开 ${opened} 个目录，另有 ${pending.length - opened} 个无法访问，可复制路径查找。` }
  } finally { if (token === generation) busy.value = false }
}
async function copyPaths() {
  const token = generation
  try { await navigator.clipboard.writeText(paths.value.join('\r\n')); if (token === generation) feedback.value = `已复制 ${paths.value.length} 条结果路径。` }
  catch { if (token === generation) feedback.value = '复制失败，请使用各条结果的复制路径按钮重试。' }
}
</script>
<template>
  <div v-if="paths.length" class="min-w-0 space-y-2 rounded-xl border border-subtle bg-input/30 p-3" data-testid="batch-result-actions">
    <div class="flex flex-wrap items-center gap-2 text-xs">
      <span class="mr-auto font-semibold text-content"><i class="pi pi-check-circle mr-1 text-emerald-500" aria-hidden="true"></i>已完成结果 · {{ locations.length }} 项</span>
      <button type="button" class="rounded-lg border border-subtle px-3 py-2 text-primary hover:bg-primary/10 disabled:opacity-50" :disabled="busy" @click.stop="openDirectories()">{{ busy ? '正在打开…' : `打开目录（${directories.length}）` }}</button>
      <button type="button" class="rounded-lg border border-subtle px-3 py-2 text-muted hover:bg-input" @click.stop="copyPaths">复制结果路径</button>
    </div>
    <div v-if="unavailable.length" class="min-w-0 rounded-lg border border-amber-500/20 bg-amber-500/5 p-2" data-testid="batch-result-unavailable">
      <div class="flex flex-wrap items-center gap-2 text-xs"><span class="mr-auto text-amber-600">以下目录暂时无法打开</span><button type="button" class="rounded-lg px-2 py-1 text-primary disabled:opacity-50" :disabled="busy" @click.stop="openDirectories(true)">重试这些目录</button></div>
      <ul class="mt-1 max-h-32 space-y-1 overflow-y-auto custom-scrollbar"><li v-for="path in unavailable" :key="path" class="select-text break-words text-xs leading-relaxed text-muted [overflow-wrap:anywhere]">{{ path }}</li></ul>
    </div>
    <p v-if="feedback" role="status" class="text-xs leading-relaxed text-muted">{{ feedback }}</p>
  </div>
</template>
