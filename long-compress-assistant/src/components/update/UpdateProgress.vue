<script setup lang="ts">
import { computed } from 'vue'
import { useUpdateStore } from '@/stores/update'
import { useAppStore } from '@/stores/app'
import { formatFileSize } from '@/utils'

const update = useUpdateStore()
const app = useAppStore()
const label = computed(() => app.t(`update.phase.${update.installPhase}`))
const percent = computed(() => update.installPhase === 'downloading' ? update.downloadPercent : null)
</script>

<template>
  <section class="update-progress" data-testid="update-progress" :data-phase="update.installPhase" aria-live="polite">
    <div class="update-progress-heading">
      <span><i class="pi" :class="update.installPhase === 'downloading' ? 'pi-cloud-download' : 'pi-spin pi-spinner'" aria-hidden="true"></i>{{ label }}</span>
      <strong v-if="percent !== null">{{ percent }}%</strong>
    </div>
    <div class="update-progress-track" role="progressbar" :aria-label="label" :aria-valuenow="percent ?? undefined" :aria-valuemin="0" :aria-valuemax="100" :aria-valuetext="percent === null ? label : `${percent}%`">
      <div :class="{ indeterminate: percent === null }" :style="percent === null ? undefined : { width: `${percent}%` }"></div>
    </div>
    <div v-if="update.installPhase === 'downloading'" class="update-progress-bytes">
      {{ formatFileSize(update.downloadedBytes) }}<template v-if="update.totalBytes"> / {{ formatFileSize(update.totalBytes) }}</template>
    </div>
    <p>{{ app.t('update.progress_hint') }}</p>
  </section>
</template>

<style scoped>
.update-progress{border:1px solid color-mix(in srgb,var(--dynamic-accent) 28%,var(--border-subtle));background:color-mix(in srgb,var(--dynamic-accent) 7%,var(--bg-card));border-radius:14px;padding:18px;color:var(--text-base)}
.update-progress-heading{display:flex;align-items:center;justify-content:space-between;gap:12px;font-size:13px;font-weight:700}
.update-progress-heading>span{display:flex;align-items:center;gap:9px;min-width:0}.update-progress-heading i,.update-progress-heading strong{color:var(--dynamic-accent)}
.update-progress-track{height:8px;margin:14px 0 8px;background:var(--bg-input);border:1px solid var(--border-subtle);border-radius:99px;overflow:hidden}
.update-progress-track>div{height:100%;border-radius:inherit;background:linear-gradient(90deg,color-mix(in srgb,var(--dynamic-accent) 65%,var(--bg-card)),var(--dynamic-accent));transition:width .2s ease}
.update-progress-track>.indeterminate{width:38%;animation:update-download 1.6s ease-in-out infinite}
.update-progress-bytes{font-size:12px;font-variant-numeric:tabular-nums;color:var(--text-secondary)}
.update-progress p{margin:8px 0 0;font-size:11px;line-height:1.6;color:var(--text-muted)}
@keyframes update-download{from{transform:translateX(-110%)}to{transform:translateX(275%)}}
@media(prefers-reduced-motion:reduce){.update-progress-track>.indeterminate{animation:none;margin-left:31%}.update-progress-track>div{transition:none}}
:global(html.reduce-motion) .update-progress-track>.indeterminate{animation:none;margin-left:31%}
</style>
