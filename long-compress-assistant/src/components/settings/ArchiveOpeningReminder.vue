<script setup lang="ts">
import { nextTick, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/tauri'
import { useAppStore } from '@/stores/app'

const app = useAppStore()
const router = useRouter()
const visible = ref(false)
let disposed = false
onUnmounted(() => { disposed = true })
watch(() => app.settingsLoaded, async ready => {
  if (!ready || !navigator.platform.toLowerCase().includes('win')) return
  try {
    const associations = await invoke<{ isDefault: boolean }[]>('archive_file_associations')
    if (!Array.isArray(associations) || disposed) return
    await router.isReady()
    await nextTick()
    if (disposed) return
    // The native, per-user receipt survives upgrades and preference resets.
    // Claim before showing, so closing the app or opening another window cannot repeat it.
    const firstTime = await invoke<boolean>('take_archive_opening_reminder')
    visible.value = !disposed && firstTime === true && !associations.some(item => item.isDefault)
  } catch { /* A setup suggestion must never interrupt startup or file opening. */ }
}, { immediate: true })

function configure() {
  visible.value = false
  void router.push({ path: '/settings', query: { section: 'behavior', focus: 'archive-opening' } })
}
</script>

<template>
  <aside v-if="visible" class="opening-reminder" data-testid="archive-opening-reminder" aria-labelledby="opening-reminder-title">
    <div class="reminder-heading"><i class="pi pi-box" aria-hidden="true"/><strong id="opening-reminder-title">用 Long解压打开压缩包</strong><button type="button" aria-label="关闭默认打开提醒" @click="visible=false"><i class="pi pi-times" aria-hidden="true"/></button></div>
    <p>在设置中选为默认应用后，双击压缩包即可按你的偏好解压或浏览。</p>
    <div class="reminder-actions"><button type="button" @click="visible=false">暂不设置</button><button type="button" class="primary" data-testid="archive-opening-reminder-configure" @click="configure">前往设置</button></div>
  </aside>
</template>

<style scoped>
.opening-reminder{position:fixed;right:20px;bottom:20px;z-index:45;width:340px;max-width:calc(100vw - 32px);padding:16px;border:1px solid var(--border-subtle);border-radius:16px;background:var(--bg-modal);box-shadow:0 12px 40px #0003;color:var(--text-base)}.reminder-heading{display:flex;gap:9px;align-items:center;font-size:13px}.reminder-heading>i{color:var(--dynamic-accent)}.reminder-heading>button{margin-left:auto;padding:5px;border-radius:6px;color:var(--text-muted)}.opening-reminder p{margin:10px 0 14px;font-size:12px;line-height:1.8;color:var(--text-muted)}.reminder-actions{display:flex;justify-content:flex-end;gap:8px}.reminder-actions button{padding:8px 12px;border:1px solid var(--border-subtle);border-radius:9px;font-size:12px;background:var(--bg-input)}.reminder-actions .primary{background:var(--dynamic-accent);color:var(--accent-ink);border-color:transparent}.opening-reminder button:hover{filter:brightness(1.08)}.opening-reminder button:focus-visible{outline:2px solid var(--dynamic-accent);outline-offset:3px}
</style>
