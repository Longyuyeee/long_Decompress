<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import { formatProgressPercent, normalizeProgressPercent } from '@/utils/progress'
import type { TaskStatus } from '@/stores/task'

withDefaults(defineProps<{
  stage: string
  status?: TaskStatus
  progress?: number
  speed?: string
  currentFile?: string
}>(), { progress: 0 })

const appStore = useAppStore()
</script>

<template>
  <section class="execution-overview" :class="`is-${status || 'pending'}`" data-testid="task-execution-overview">
    <div class="execution-state">
      <div class="execution-stage">
        <span>{{ appStore.t('progress.stage') }}</span>
        <strong :title="stage">{{ stage }}</strong>
      </div>
      <div class="execution-progress">
        <span>{{ appStore.t('progress.percent') }}</span>
        <strong><slot name="progress">{{ formatProgressPercent(progress) }}%</slot></strong>
        <small v-if="speed">{{ speed }}</small>
      </div>
    </div>
    <div class="execution-progress-track" aria-hidden="true">
      <div :style="{ width: `${normalizeProgressPercent(progress / 100)}%` }"></div>
    </div>
    <div v-if="$slots.default" class="execution-statistics"><slot /></div>
    <div v-if="currentFile" class="execution-current-file" :title="currentFile">
      <i class="pi pi-file" aria-hidden="true"></i>
      <p class="current-file-path task-current-file">{{ currentFile }}</p>
    </div>
    <div v-if="$slots.extra" class="execution-extra"><slot name="extra" /></div>
  </section>
</template>

<style scoped>
.execution-overview {
  min-width: 0;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 60%, transparent);
  border-radius: .875rem;
  padding: .75rem;
  background: color-mix(in srgb, var(--bg-input) 42%, transparent);
  color: var(--text-content);
}
.execution-state { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; }
.execution-stage { min-width: 0; display: flex; flex-wrap: wrap; align-items: baseline; gap: .35rem; }
.execution-stage > span, .execution-progress > span { color: var(--text-muted); font-size: .6875rem; }
.execution-stage strong { display: block; color: var(--text-content); font-size: .8125rem; overflow-wrap: anywhere; }
.execution-progress { flex: 0 0 auto; text-align: right; }
.execution-progress > span { margin-right: .35rem; }
.execution-progress > strong { color: var(--dynamic-accent); font-family: monospace; font-size: .9375rem; }
.execution-progress > small { display: block; margin-top: .2rem; color: var(--text-muted); font-size: .6875rem; }
.execution-progress-track { height: .25rem; margin-top: .625rem; overflow: hidden; border-radius: 999px; background: color-mix(in srgb, var(--dynamic-accent) 12%, var(--bg-input)); }
.execution-progress-track > div { height: 100%; border-radius: inherit; background: var(--dynamic-accent); transition: width .3s ease; }
.is-completed .execution-stage strong { color: #22c55e; }
.is-failed .execution-stage strong { color: #f87171; }
.is-cancelled .execution-stage strong { color: var(--text-muted); }
.execution-statistics { margin-top: .75rem; }
.execution-statistics:empty { display: none; }
.execution-statistics :deep(.execution-metrics) { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: .625rem 1rem; margin: 0; }
.execution-statistics :deep(.execution-metric) { min-width: 0; display: flex; flex-wrap: wrap; align-items: baseline; justify-content: space-between; gap: .125rem .5rem; }
.execution-statistics :deep(dt) { color: var(--text-muted); font-size: .6875rem; line-height: 1.5; overflow-wrap: anywhere; }
.execution-statistics :deep(dd) { margin: .125rem 0 0; font-family: monospace; font-size: .8125rem; font-weight: 800; overflow-wrap: anywhere; }
.execution-current-file { display: flex; align-items: flex-start; gap: .5rem; margin-top: .625rem; }
.execution-current-file > i { flex: none; margin-top: .2rem; color: var(--text-dim); font-size: .6875rem; }
.execution-current-file > p { min-width: 0; margin: 0; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; overflow: hidden; overflow-wrap: anywhere; color: var(--text-muted); font-family: monospace; font-size: .6875rem; line-height: 1.6; }
.execution-extra { margin-top: .75rem; }
.execution-extra:empty { display: none; }
@media (prefers-reduced-motion: reduce) { .execution-progress-track > div { transition: none; } }
</style>
