<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, onDeactivated, ref, useId } from 'vue'

defineProps<{ label: string }>()
const id = useId()
const trigger = ref<HTMLButtonElement>()
const panel = ref<HTMLElement>()
const open = ref(false)
let skipFocus = false
let pinned = false, closeTimer: ReturnType<typeof setTimeout> | undefined
const keepOpen = () => { if (closeTimer) clearTimeout(closeTimer); closeTimer = undefined }
const leave = () => { keepOpen(); if (!pinned) closeTimer = setTimeout(() => close(), 160) }
const position = ref({ left: '0px', top: '0px' })
function close(restore = false) {
  keepOpen(); pinned = false
  open.value = false
  if (restore) { skipFocus = true; trigger.value?.focus(); skipFocus = false }
}
function toggle() {
  if (pinned) { close(); return }
  pinned = true; void show()
}
async function show() {
  if (skipFocus) return
  keepOpen()
  open.value = true
  await nextTick()
  if (!open.value || !trigger.value || !panel.value) return
  const rect = trigger.value!.getBoundingClientRect()
  const height = panel.value!.getBoundingClientRect().height
  const width = panel.value!.getBoundingClientRect().width
  position.value = {
    left: `${Math.max(8, Math.min(rect.left, window.innerWidth - width - 8))}px`,
    top: `${Math.max(8, rect.bottom + height + 8 < window.innerHeight ? rect.bottom + 6 : rect.top - height - 6)}px`,
  }
}
function outside(event: Event) {
  if (!trigger.value?.contains(event.target as Node) && !panel.value?.contains(event.target as Node)) close()
}
function key(event: KeyboardEvent) {
  if (open.value && event.key === 'Escape') { event.preventDefault(); close(true) }
}
function dismiss(event: Event) {
  if (event.type === 'scroll' && panel.value?.contains(event.target as Node)) return
  close()
}
onDeactivated(() => close())
onMounted(() => {
  document.addEventListener('pointerdown', outside)
  document.addEventListener('keydown', key)
  document.addEventListener('focusin', outside)
  window.addEventListener('resize', dismiss)
  window.addEventListener('scroll', dismiss, true)
})
onUnmounted(() => {
  keepOpen()
  document.removeEventListener('pointerdown', outside)
  document.removeEventListener('keydown', key)
  document.removeEventListener('focusin', outside)
  window.removeEventListener('resize', dismiss)
  window.removeEventListener('scroll', dismiss, true)
})
</script>

<template>
  <button ref="trigger" type="button" class="help-trigger" :aria-label="label" :aria-expanded="open" :aria-controls="open ? id : undefined" @mouseenter="show" @mouseleave="leave" @focus="show" @blur="leave" @click.stop.prevent="toggle">?</button>
  <Teleport to="body">
    <div v-if="open" :id="id" ref="panel" role="note" :aria-label="label" class="help-panel" :style="position" @mouseenter="keepOpen" @mouseleave="leave" @click.stop><slot /></div>
  </Teleport>
</template>

<style scoped>
.help-trigger { display:inline-flex; flex-shrink:0; align-items:center; justify-content:center; width:1rem; height:1rem; border:1px solid currentColor; border-radius:50%; color:var(--text-muted); font-size:.65rem; line-height:1; }
.help-trigger:hover,.help-trigger:focus-visible { color:var(--dynamic-accent); outline:2px solid var(--dynamic-accent); outline-offset:2px; }
.help-panel { position:fixed; z-index:10000; box-sizing:border-box; width:19rem; max-width:calc(100vw - 16px); max-height:calc(100vh - 16px); overflow:auto; padding:.75rem; border:1px solid var(--border-subtle); border-radius:.75rem; background:var(--bg-card); color:var(--text-muted); box-shadow:0 8px 28px #0002; font-size:.75rem; line-height:1.65; overflow-wrap:anywhere; }
</style>
