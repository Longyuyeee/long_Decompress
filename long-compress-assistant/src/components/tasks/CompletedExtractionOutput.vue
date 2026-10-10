<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'

const props = withDefaults(defineProps<{ path: string; title?: string }>(), { title: '解压结果' })
const busy = ref(false)
const error = ref('')
const copied = ref(false)
let request = 0
onBeforeUnmount(() => { request++ })
watch(() => props.path, () => { request++; busy.value = false; error.value = ''; copied.value = false })

async function openOutput() {
  if (busy.value) return
  const token = ++request
  const path = props.path
  busy.value = true
  error.value = ''
  try {
    if (!path.trim()) { error.value = '没有记录输出目录。'; return }
    const state = await invoke<string>('inspect_extraction_destination', { path })
    if (token !== request) return
    if (state === 'missing' || state === 'file') {
      error.value = state === 'missing'
        ? '找不到输出目录，可能已被移动或删除。可以复制原路径查找，或从历史记录再次解压。'
        : '原输出位置现在是一个文件。可以复制路径查找，或从历史记录再次解压。'
      return
    }
    if (state !== 'empty' && state !== 'occupied') {
      error.value = '暂时无法访问输出目录，请检查磁盘或网络连接后重试。'
      return
    }
    await invoke('open_in_explorer', { path })
  } catch {
    if (token === request) error.value = '无法打开输出目录，请重试或复制路径手动打开。'
  } finally { if (token === request) busy.value = false }
}
async function copyPath() {
  const path = props.path
  try { await navigator.clipboard.writeText(path); if (path === props.path) copied.value = true }
  catch { if (path === props.path) error.value = '复制失败，可以选中上方路径手动复制。' }
}
</script>

<template>
  <section class="min-w-0 space-y-3 rounded-xl border border-subtle bg-input/30 p-4" data-testid="completed-extraction-output">
    <div class="flex min-w-0 items-start gap-3">
      <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary"><i class="pi pi-folder-open" aria-hidden="true"></i></span>
      <div class="min-w-0 flex-1">
        <p class="break-words text-sm font-bold text-content [overflow-wrap:anywhere]">{{ title }}</p>
        <p class="mt-1 select-text break-words text-xs leading-relaxed text-muted [overflow-wrap:anywhere]">{{ path || '未记录输出目录' }}</p>
      </div>
    </div>
    <div class="flex flex-wrap gap-2">
      <button type="button" class="rounded-lg bg-primary px-3 py-2 text-sm font-medium text-white transition-colors hover:bg-primary/90 focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary disabled:opacity-50" :disabled="busy || !path.trim()" @click="openOutput">{{ busy ? '正在打开…' : '打开输出目录' }}</button>
      <button type="button" class="rounded-lg border border-subtle px-3 py-2 text-sm text-muted transition-colors hover:bg-input hover:text-content disabled:opacity-50" :disabled="!path.trim()" @click="copyPath">{{ copied ? '已复制' : '复制路径' }}</button>
    </div>
    <p v-if="error" role="alert" class="rounded-lg border border-subtle bg-input/50 px-3 py-2 text-sm leading-relaxed text-content">{{ error }}</p>
  </section>
</template>
