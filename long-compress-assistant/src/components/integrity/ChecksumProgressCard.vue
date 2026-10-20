<script setup lang="ts">
import { computed } from 'vue'
import type { ChecksumProgress } from '@/types/checksum'
const props = defineProps<{ progress: ChecksumProgress | null; stopping: boolean }>()
defineEmits<{ stop: [] }>()
const percent = computed(() => props.progress?.totalBytes ? Math.min(100, props.progress.completedBytes / props.progress.totalBytes * 100) : 0)
const bytes = (value: number) => value >= 1024 ** 3 ? `${(value / 1024 ** 3).toFixed(2)} GB` : value >= 1024 ** 2 ? `${(value / 1024 ** 2).toFixed(1)} MB` : value >= 1024 ? `${(value / 1024).toFixed(1)} KB` : `${value} B`
</script>

<template>
  <div class="read-progress" :data-read-bytes="progress?.completedBytes || 0" :data-file-bytes="progress?.fileBytes || 0" :data-total-bytes="progress?.totalBytes || 0" data-testid="checksum-progress" role="status" aria-live="polite">
    <div class="read-heading"><span><i class="pi pi-spinner pi-spin"></i>{{ stopping ? '正在停止读取…' : '正在校验' }}</span><button :disabled="!progress || stopping" data-testid="checksum-stop" @click="$emit('stop')"><i class="pi pi-stop-circle"></i>{{ stopping ? '停止中…' : '停止' }}</button></div>
    <p class="read-file" data-testid="checksum-current-file" :title="progress?.currentFile">{{ progress?.currentFile || '正在准备本次文件列表…' }}</p>
    <div class="read-track" role="progressbar" aria-label="本次文件读取进度" :aria-valuenow="percent" aria-valuemin="0" aria-valuemax="100"><span :style="{ width: `${percent}%` }"></span></div>
    <div v-if="progress" class="read-stats"><span data-testid="checksum-byte-count">已读取 {{ bytes(progress.completedBytes) }} / {{ bytes(progress.totalBytes) }}</span><span>{{ progress.completedFiles }} / {{ progress.totalFiles }} 项已处理</span></div>
    <p v-if="progress?.currentFile" class="read-hint">当前文件 {{ bytes(progress.fileBytes) }} / {{ bytes(progress.fileSize) }} · 停止后可继续未完成项</p>
  </div>
</template>

<style scoped>
.read-progress { min-width:0; margin-top:.7rem; padding:.8rem; border:1px solid color-mix(in srgb,var(--dynamic-accent) 28%,var(--border-subtle)); border-radius:.8rem; background:color-mix(in srgb,var(--dynamic-accent) 5%,var(--bg-input)); }
.read-heading,.read-stats { display:flex; flex-wrap:wrap; align-items:center; justify-content:space-between; gap:.5rem; }
.read-heading { color:var(--dynamic-accent); font-size:.75rem; font-weight:800; }
.read-heading span,.read-heading button { display:inline-flex; align-items:center; gap:.4rem; }
.read-heading button { padding:.4rem .7rem; border:1px solid var(--border-subtle); border-radius:.55rem; background:var(--bg-card); color:var(--text-content); }
.read-heading button:disabled { opacity:.5; cursor:wait; }
.read-file { margin:.5rem 0; overflow-wrap:anywhere; color:var(--text-content); font-size:.68rem; line-height:1.6; }
.read-track { height:.35rem; overflow:hidden; border-radius:999px; background:var(--border-subtle); }
.read-track span { display:block; height:100%; background:var(--dynamic-accent); transition:width .12s ease; }
.read-stats,.read-hint { margin-top:.45rem; color:var(--text-muted); font-size:.62rem; line-height:1.6; }
@media(prefers-reduced-motion:reduce) { .read-track span { transition:none; } }
</style>
