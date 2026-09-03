<script setup lang="ts">
import { computed } from 'vue'
import type { ResourcePreflightReport } from '@/types/resourcePreflight'
import { formatResourceBytes } from '@/utils/resourcePreflight'

const props = withDefaults(defineProps<{ report?: ResourcePreflightReport, compact?: boolean }>(), {
  compact: false,
})

const statusLabel = computed(() => ({
  ready: '已通过',
  warning: '需留意',
  blocked: '已阻止',
}[props.report?.status || 'warning']))

const locationLabel = computed(() => ({
  local: '本地磁盘',
  network: '网络位置',
  removable: '移动设备',
  unknown: '位置未知',
}[props.report?.location || 'unknown']))

const compactLocationLabel = computed(() => ({
  local: '本地',
  network: '网络',
  removable: '移动设备',
  unknown: '未知',
}[props.report?.location || 'unknown']))

const mediumLabel = computed(() => ({
  ssd: 'SSD',
  hdd: 'HDD',
  unknown: '介质未知',
}[props.report?.medium || 'unknown']))
</script>

<template>
  <section
    v-if="report"
    data-testid="resource-preflight-card"
    class="resource-preflight-card min-w-0 rounded-xl border text-xs"
    :class="[`is-${report.status}`, { 'is-compact': compact }]"
  >
    <div v-if="compact" data-testid="resource-preflight-metrics" class="compact-preflight min-w-0 px-3 py-2.5">
      <div class="compact-preflight-heading flex min-w-0 items-center gap-2">
        <i class="pi pi-database shrink-0 text-primary"></i>
        <span class="shrink-0 font-black text-content">存储预检</span>
        <span class="status-badge shrink-0 rounded-full px-2 py-0.5 font-black">{{ statusLabel }}</span>
        <span class="min-w-0 flex-1 truncate text-right text-dim" :title="`${locationLabel} · ${mediumLabel}`">{{ compactLocationLabel }} · {{ mediumLabel }}</span>
      </div>
      <dl class="compact-preflight-grid mt-2 grid min-w-0 grid-cols-2 gap-1.5">
        <div class="metric"><dt>剩余可用</dt><dd>{{ formatResourceBytes(report.availableBytes) }}</dd></div>
        <div class="metric"><dt>预计占用</dt><dd>{{ formatResourceBytes(report.estimatedOutputBytes) }}</dd></div>
      </dl>
      <div class="mt-2 min-w-0 truncate whitespace-nowrap font-mono text-dim" :title="`${report.summary}；${report.mountPoint || report.probePath}；${report.fileSystem || '文件系统未知'}；预留 ${formatResourceBytes(report.reserveBytes)}`">
        {{ report.mountPoint || report.probePath }}<span v-if="report.fileSystem"> · {{ report.fileSystem }}</span> · 预留 {{ formatResourceBytes(report.reserveBytes) }}
      </div>
    </div>
    <template v-else>
    <div class="flex min-w-0 items-start justify-between gap-3">
      <div class="min-w-0">
        <div class="flex items-center gap-2 font-black text-content">
          <i class="pi pi-database text-primary"></i>
          <span>目标存储预检</span>
        </div>
        <p class="mt-1 break-words [overflow-wrap:anywhere] leading-relaxed text-muted">{{ report.summary }}</p>
      </div>
      <span class="status-badge shrink-0 rounded-full px-2 py-1 font-black">{{ statusLabel }}</span>
    </div>

    <dl data-testid="resource-preflight-metrics" class="resource-preflight-metrics mt-3 grid min-w-0 grid-cols-2 gap-2">
      <div data-testid="resource-preflight-location" class="metric"><dt>目标位置</dt><dd :title="locationLabel">{{ locationLabel }}</dd></div>
      <div data-testid="resource-preflight-medium" class="metric"><dt>存储介质</dt><dd :title="mediumLabel">{{ mediumLabel }}</dd></div>
      <div data-testid="resource-preflight-available" class="metric"><dt>剩余可用</dt><dd>{{ formatResourceBytes(report.availableBytes) }}</dd></div>
      <div data-testid="resource-preflight-estimated" class="metric"><dt>预计占用</dt><dd>{{ formatResourceBytes(report.estimatedOutputBytes) }}</dd></div>
    </dl>

    <div class="mt-2 min-w-0 break-words [overflow-wrap:anywhere] font-mono text-dim">
      {{ report.mountPoint || report.probePath }}<span v-if="report.fileSystem"> · {{ report.fileSystem }}</span>
      · 预留 {{ formatResourceBytes(report.reserveBytes) }}
    </div>
    <ul v-if="report.warnings.length" class="mt-2 space-y-1 text-amber-500">
      <li v-for="warning in report.warnings" :key="warning" class="flex min-w-0 gap-2">
        <i class="pi pi-exclamation-triangle mt-0.5 shrink-0"></i>
        <span class="min-w-0 break-words [overflow-wrap:anywhere]">{{ warning }}</span>
      </li>
    </ul>
    </template>
  </section>
</template>

<style scoped>
.resource-preflight-card {
  container-type: inline-size;
  max-width: 100%;
  overflow-x: hidden;
  background: color-mix(in srgb, var(--bg-input) 42%, transparent);
  border-color: color-mix(in srgb, var(--border-subtle) 74%, transparent);
}
.resource-preflight-card:not(.is-compact) { padding: 0.75rem; }
.resource-preflight-card.is-compact { min-height: 7.25rem; overflow: hidden; }

.resource-preflight-card.is-ready { border-color: color-mix(in srgb, #22c55e 38%, var(--border-subtle)); }
.resource-preflight-card.is-warning { border-color: color-mix(in srgb, #f59e0b 44%, var(--border-subtle)); }
.resource-preflight-card.is-blocked { border-color: color-mix(in srgb, #ef4444 48%, var(--border-subtle)); }
.is-ready .status-badge { color: #22c55e; background: color-mix(in srgb, #22c55e 12%, transparent); }
.is-warning .status-badge { color: #f59e0b; background: color-mix(in srgb, #f59e0b 12%, transparent); }
.is-blocked .status-badge { color: #ef4444; background: color-mix(in srgb, #ef4444 12%, transparent); }

.metric {
  min-width: 0;
  padding: 0.5rem;
  border-radius: 0.5rem;
  background: color-mix(in srgb, var(--bg-card) 48%, transparent);
}
.metric dt,
.metric dd {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.metric dt { color: var(--text-muted); }
.metric dd { margin-top: 0.125rem; color: var(--text-content); font-weight: 800; }

@container (max-width: 17rem) {
  .resource-preflight-metrics { grid-template-columns: minmax(0, 1fr); }
}
</style>
