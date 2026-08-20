<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useHistoryStore } from '@/stores/history'
import { useAppStore } from '@/stores/app'
import type { TaskHistoryRecord, TaskHistoryStatus } from '@/types/taskHistory'

type TypeFilter = 'all' | 'compression' | 'decompression'
type StatusFilter = 'all' | TaskHistoryStatus
type RangeFilter = '7d' | '30d' | '90d' | 'all'

const historyStore = useHistoryStore()
const appStore = useAppStore()
const query = ref('')
const typeFilter = ref<TypeFilter>('all')
const statusFilter = ref<StatusFilter>('all')
const rangeFilter = ref<RangeFilter>('30d')
const selectedRecord = ref<TaskHistoryRecord | null>(null)
const showClearConfirm = ref(false)

const DAY_MS = 86_400_000
const terminalColor: Record<TaskHistoryStatus, string> = {
  completed: 'text-emerald-500 bg-emerald-500/10 border-emerald-500/20',
  failed: 'text-red-500 bg-red-500/10 border-red-500/20',
  cancelled: 'text-amber-500 bg-amber-500/10 border-amber-500/20',
}

const safeTime = (value?: string | null) => {
  const time = value ? new Date(value).getTime() : Number.NaN
  return Number.isNaN(time) ? 0 : time
}

const filteredRecords = computed(() => {
  const normalized = query.value.trim().toLowerCase()
  const cutoff = rangeFilter.value === 'all'
    ? 0
    : Date.now() - Number.parseInt(rangeFilter.value, 10) * DAY_MS
  return historyStore.sortedRecords.filter(record => {
    const matchesQuery = !normalized || [record.name, record.outputPath, record.format, ...record.sourcePaths]
      .some(value => value?.toLowerCase().includes(normalized))
    return matchesQuery
      && (typeFilter.value === 'all' || record.taskType === typeFilter.value)
      && (statusFilter.value === 'all' || record.status === statusFilter.value)
      && (!cutoff || safeTime(record.completedAt) >= cutoff)
  })
})

const stats = computed(() => {
  const records = historyStore.records
  const success = records.filter(record => record.status === 'completed').length
  const durations = records.filter(record => record.durationMs > 0).map(record => record.durationMs)
  return {
    total: records.length,
    successRate: records.length ? Math.round((success / records.length) * 100) : 0,
    processed: records.reduce((sum, record) => sum + Math.max(record.processedBytes, record.totalBytes), 0),
    averageDuration: durations.length ? durations.reduce((sum, value) => sum + value, 0) / durations.length : 0,
  }
})

const trend = computed(() => {
  const today = new Date()
  const dayStart = new Date(today.getFullYear(), today.getMonth(), today.getDate()).getTime()
  const buckets = Array.from({ length: 14 }, (_, index) => {
    const start = dayStart - (13 - index) * DAY_MS
    const end = start + DAY_MS
    const dayRecords = historyStore.records.filter(record => {
      const time = safeTime(record.completedAt)
      return time >= start && time < end
    })
    return {
      label: new Intl.DateTimeFormat(appStore.language || 'zh-CN', { month: 'numeric', day: 'numeric' }).format(start),
      total: dayRecords.length,
      success: dayRecords.filter(record => record.status === 'completed').length,
      failed: dayRecords.filter(record => record.status === 'failed').length,
    }
  })
  const max = Math.max(...buckets.map(bucket => bucket.total), 1)
  return buckets.map(bucket => ({ ...bucket, height: Math.max(4, (bucket.total / max) * 100) }))
})

const groupedRecords = computed(() => {
  const groups = new Map<string, TaskHistoryRecord[]>()
  filteredRecords.value.forEach(record => {
    const time = safeTime(record.completedAt)
    const key = time
      ? new Intl.DateTimeFormat(appStore.language || 'zh-CN', { year: 'numeric', month: 'long', day: 'numeric' }).format(time)
      : appStore.t('history.unknown_date')
    groups.set(key, [...(groups.get(key) || []), record])
  })
  return Array.from(groups.entries()).map(([label, records]) => ({ label, records }))
})

const formatBytes = (bytes: number) => {
  if (!bytes) return '—'
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB']
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  return `${(bytes / 1024 ** index).toFixed(index > 1 ? 1 : 0)} ${units[index]}`
}

const formatDuration = (ms: number) => {
  if (!ms) return '—'
  if (ms < 1_000) return `${ms} ms`
  if (ms < 60_000) return `${(ms / 1_000).toFixed(1)} s`
  const minutes = Math.floor(ms / 60_000)
  return `${minutes}m ${Math.round((ms % 60_000) / 1_000)}s`
}

const formatDateTime = (value?: string | null) => {
  const time = safeTime(value)
  return time ? new Intl.DateTimeFormat(appStore.language || 'zh-CN', {
    month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit',
  }).format(time) : '—'
}

const typeLabel = (record: TaskHistoryRecord) => appStore.t(`history.type.${record.taskType}`)
const statusLabel = (record: TaskHistoryRecord) => appStore.t(`history.status.${record.status}`)

const refresh = async () => {
  try { await historyStore.fetchHistory() } catch { /* rendered by error state */ }
}

const confirmClear = async () => {
  try {
    await historyStore.clearHistory()
    selectedRecord.value = null
    showClearConfirm.value = false
  } catch (error) {
    appStore.setError(`${appStore.t('common.error')}: ${String(error)}`)
  }
}

const removeSelectedRecord = async () => {
  if (!selectedRecord.value) return
  try {
    await historyStore.deleteRecord(selectedRecord.value.id)
    selectedRecord.value = null
  } catch (error) {
    appStore.setError(`${appStore.t('common.error')}: ${String(error)}`)
  }
}

watch(filteredRecords, records => {
  if (selectedRecord.value && !records.some(record => record.id === selectedRecord.value?.id)) {
    selectedRecord.value = null
  }
})

onMounted(refresh)
</script>

<template>
  <div class="history-view h-full min-w-0 overflow-y-auto overflow-x-hidden custom-scrollbar p-responsive p-6 lg:p-8">
    <div class="mx-auto w-full max-w-[1500px] space-y-5 pb-8">
      <header class="flex flex-col sm:flex-row sm:items-end justify-between gap-4">
        <div>
          <p class="text-xs font-black uppercase tracking-[0.24em] text-primary mb-2">{{ appStore.t('history.eyebrow') }}</p>
          <h1 class="text-3xl lg:text-4xl font-black text-content tracking-tight">{{ appStore.t('nav.history') }}</h1>
          <p class="text-sm text-muted mt-2">{{ appStore.t('history.subtitle') }}</p>
        </div>
        <div class="flex gap-2">
          <button type="button" class="history-ghost-button" :disabled="historyStore.isLoading" @click="refresh">
            <i class="pi pi-refresh" :class="{ 'pi-spin': historyStore.isLoading }"></i>
            {{ appStore.t('history.refresh') }}
          </button>
          <button type="button" class="history-ghost-button text-red-500" :disabled="!historyStore.records.length" @click="showClearConfirm = true">
            <i class="pi pi-trash"></i>{{ appStore.t('history.clear') }}
          </button>
        </div>
      </header>

      <section class="grid grid-cols-2 xl:grid-cols-4 gap-3" data-testid="history-kpis">
        <article class="history-kpi history-kpi-primary">
          <div><span>{{ appStore.t('history.total_tasks') }}</span><i class="pi pi-inbox"></i></div>
          <strong>{{ stats.total }}</strong><small>{{ appStore.t('history.persisted_hint') }}</small>
        </article>
        <article class="history-kpi">
          <div><span>{{ appStore.t('history.success_rate') }}</span><i class="pi pi-check-circle text-emerald-500"></i></div>
          <strong>{{ stats.successRate }}<em>%</em></strong><small>{{ appStore.t('history.all_time') }}</small>
        </article>
        <article class="history-kpi">
          <div><span>{{ appStore.t('history.processed_volume') }}</span><i class="pi pi-database text-sky-500"></i></div>
          <strong class="text-[clamp(1.2rem,2vw,2rem)]">{{ formatBytes(stats.processed) }}</strong><small>{{ appStore.t('history.real_bytes_hint') }}</small>
        </article>
        <article class="history-kpi">
          <div><span>{{ appStore.t('history.average_duration') }}</span><i class="pi pi-stopwatch text-amber-500"></i></div>
          <strong class="text-[clamp(1.2rem,2vw,2rem)]">{{ formatDuration(stats.averageDuration) }}</strong><small>{{ appStore.t('history.completed_samples') }}</small>
        </article>
      </section>

      <section class="history-panel p-4 sm:p-5">
        <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-4 mb-5">
          <div>
            <p class="text-xs font-black uppercase tracking-[0.18em] text-primary">{{ appStore.t('history.trend_eyebrow') }}</p>
            <h2 class="font-black text-content mt-1">{{ appStore.t('history.trend_title') }}</h2>
          </div>
          <div class="flex flex-wrap gap-3 text-xs font-bold text-muted">
            <span class="flex items-center gap-1.5"><i class="w-2 h-2 rounded-full bg-primary"></i>{{ appStore.t('history.legend.total') }}</span>
            <span class="flex items-center gap-1.5"><i class="w-2 h-2 rounded-full bg-emerald-500"></i>{{ appStore.t('history.legend.completed') }}</span>
            <span class="flex items-center gap-1.5"><i class="w-2 h-2 rounded-full bg-red-500"></i>{{ appStore.t('history.legend.failed') }}</span>
          </div>
        </div>
        <div class="history-chart h-32 flex items-end gap-1.5 sm:gap-2" data-testid="history-trend">
          <div v-for="bucket in trend" :key="bucket.label" class="group flex-1 h-full flex flex-col justify-end items-center min-w-0">
            <div class="relative w-full max-w-8 flex-1 flex items-end">
              <div class="w-full rounded-t-lg bg-gradient-to-t from-primary to-primary/50 transition-all duration-700 group-hover:brightness-110" :style="{ height: `${bucket.height}%` }">
                <div v-if="bucket.success" class="absolute bottom-0 left-0 w-1/2 bg-emerald-400/80 rounded-tl-lg" :style="{ height: `${Math.max(8, bucket.success / Math.max(bucket.total, 1) * bucket.height)}%` }"></div>
                <div v-if="bucket.failed" class="absolute bottom-0 right-0 w-1/2 bg-red-400/80 rounded-tr-lg" :style="{ height: `${Math.max(8, bucket.failed / Math.max(bucket.total, 1) * bucket.height)}%` }"></div>
              </div>
            </div>
            <span class="mt-2 text-[9px] text-dim truncate w-full text-center">{{ bucket.label }}</span>
          </div>
        </div>
      </section>

      <section class="history-panel p-3 sm:p-4 flex flex-col xl:flex-row gap-3" aria-label="History filters">
        <label class="relative flex-1 min-w-0">
          <i class="pi pi-search absolute left-4 top-1/2 -translate-y-1/2 text-dim"></i>
          <input v-model="query" data-testid="history-search" class="history-control w-full pl-11" :placeholder="appStore.t('history.search_placeholder')">
        </label>
        <div class="grid grid-cols-3 gap-2 xl:flex xl:shrink-0">
          <select v-model="typeFilter" class="history-control min-w-0" data-testid="history-type-filter">
            <option value="all">{{ appStore.t('history.type.all') }}</option><option value="compression">{{ appStore.t('history.type.compression') }}</option><option value="decompression">{{ appStore.t('history.type.decompression') }}</option>
          </select>
          <select v-model="statusFilter" class="history-control min-w-0" data-testid="history-status-filter">
            <option value="all">{{ appStore.t('history.status.all') }}</option><option value="completed">{{ appStore.t('history.status.completed') }}</option><option value="failed">{{ appStore.t('history.status.failed') }}</option><option value="cancelled">{{ appStore.t('history.status.cancelled') }}</option>
          </select>
          <select v-model="rangeFilter" class="history-control min-w-0" data-testid="history-range-filter">
            <option value="7d">{{ appStore.t('history.range.7d') }}</option><option value="30d">{{ appStore.t('history.range.30d') }}</option><option value="90d">{{ appStore.t('history.range.90d') }}</option><option value="all">{{ appStore.t('history.range.all') }}</option>
          </select>
        </div>
      </section>

      <div v-if="historyStore.error" class="history-panel p-8 text-center">
        <i class="pi pi-exclamation-triangle text-red-500 text-2xl"></i><p class="font-black mt-3">{{ appStore.t('history.load_failed') }}</p><p class="text-xs text-muted mt-1 break-all">{{ historyStore.error }}</p>
      </div>
      <div v-else-if="historyStore.isLoading && !historyStore.isInitialized" class="history-panel p-5 space-y-3" aria-busy="true">
        <div v-for="index in 5" :key="index" class="h-16 rounded-2xl bg-input/70 animate-pulse"></div>
      </div>
      <div v-else-if="!filteredRecords.length" class="history-panel min-h-72 flex flex-col items-center justify-center text-center p-8" data-testid="history-empty">
        <div class="w-16 h-16 rounded-3xl bg-primary/10 border border-primary/20 flex items-center justify-center text-primary"><i class="pi pi-history text-2xl"></i></div>
        <h2 class="font-black text-lg mt-5">{{ historyStore.records.length ? appStore.t('history.no_matches') : appStore.t('history.empty_title') }}</h2>
        <p class="text-sm text-muted mt-2 max-w-md">{{ historyStore.records.length ? appStore.t('history.no_matches_hint') : appStore.t('history.empty_hint') }}</p>
      </div>

      <section v-else class="space-y-5" data-testid="history-list">
        <div v-for="group in groupedRecords" :key="group.label">
          <div class="flex items-center gap-3 mb-2 px-1"><h2 class="text-xs font-black uppercase tracking-[0.16em] text-muted">{{ group.label }}</h2><span class="h-px bg-subtle flex-1"></span><span class="text-[10px] text-dim">{{ group.records.length }}</span></div>
          <div class="history-panel divide-y divide-subtle/60 overflow-hidden">
            <article v-for="record in group.records" :key="record.id" class="history-row grid gap-3 p-4 cursor-pointer hover:bg-primary/5 transition-colors" tabindex="0" @click="selectedRecord = record" @keydown.enter="selectedRecord = record">
              <div class="flex items-center gap-3 min-w-0">
                <div class="w-10 h-10 rounded-2xl bg-primary/10 text-primary flex items-center justify-center shrink-0"><i :class="record.taskType === 'compression' ? 'pi pi-box' : 'pi pi-folder-open'"></i></div>
                <div class="min-w-0"><h3 class="font-black text-content truncate">{{ record.name }}</h3><p class="text-xs text-muted truncate mt-0.5">{{ record.outputPath || record.sourcePaths[0] }}</p></div>
              </div>
              <div class="hidden md:flex items-center gap-2 text-xs text-muted"><span class="history-chip">{{ typeLabel(record) }}</span><span v-if="record.format" class="history-chip uppercase">{{ record.format }}</span></div>
              <div class="text-xs text-muted"><strong class="block text-content">{{ formatBytes(Math.max(record.processedBytes, record.totalBytes)) }}</strong><span>{{ formatDuration(record.durationMs) }}</span></div>
              <div class="flex items-center justify-end gap-3"><span class="px-2.5 py-1 rounded-full border text-xs font-black" :class="terminalColor[record.status]">{{ statusLabel(record) }}</span><time class="text-xs text-dim hidden sm:block">{{ formatDateTime(record.completedAt) }}</time><i class="pi pi-chevron-right text-dim"></i></div>
            </article>
          </div>
        </div>
      </section>
    </div>

    <Teleport to="body">
      <transition name="history-drawer">
        <div v-if="selectedRecord" class="fixed inset-0 z-[360] bg-slate-950/35 backdrop-blur-sm flex justify-end" @click.self="selectedRecord = null">
          <aside class="history-detail h-full w-full max-w-[520px] overflow-y-auto overflow-x-hidden custom-scrollbar p-5 sm:p-7" data-testid="history-detail">
            <header class="flex items-start justify-between gap-4 mb-6"><div class="min-w-0"><p class="text-xs font-black text-primary uppercase tracking-[0.2em]">{{ appStore.t('history.detail_eyebrow') }}</p><h2 class="text-2xl font-black text-content truncate mt-2">{{ selectedRecord.name }}</h2><p class="text-sm text-muted mt-1">{{ typeLabel(selectedRecord) }} · {{ selectedRecord.format?.toUpperCase() || '—' }}</p></div><button class="history-icon-button" :aria-label="appStore.t('common.close')" @click="selectedRecord = null"><i class="pi pi-times"></i></button></header>
            <div class="grid grid-cols-2 gap-3 mb-5"><div class="detail-metric"><span>{{ appStore.t('history.detail.status') }}</span><strong :class="terminalColor[selectedRecord.status].split(' ')[0]">{{ statusLabel(selectedRecord) }}</strong></div><div class="detail-metric"><span>{{ appStore.t('history.detail.duration') }}</span><strong>{{ formatDuration(selectedRecord.durationMs) }}</strong></div><div class="detail-metric"><span>{{ appStore.t('history.detail.volume') }}</span><strong>{{ formatBytes(Math.max(selectedRecord.processedBytes, selectedRecord.totalBytes)) }}</strong></div><div class="detail-metric"><span>{{ appStore.t('history.detail.completed_at') }}</span><strong>{{ formatDateTime(selectedRecord.completedAt) }}</strong></div></div>
            <section class="detail-section"><h3><i class="pi pi-sign-in"></i>{{ appStore.t('history.detail.sources') }}</h3><div class="space-y-2 mt-3"><code v-for="source in selectedRecord.sourcePaths" :key="source" class="detail-path">{{ source }}</code><p v-if="!selectedRecord.sourcePaths.length" class="text-sm text-dim">—</p></div></section>
            <section class="detail-section"><h3><i class="pi pi-sign-out"></i>{{ appStore.t('history.detail.output') }}</h3><code class="detail-path mt-3">{{ selectedRecord.outputPath || '—' }}</code></section>
            <section v-if="selectedRecord.errorMessage" class="detail-section border-red-500/20 bg-red-500/5"><h3 class="text-red-500"><i class="pi pi-exclamation-circle"></i>{{ appStore.t('history.detail.error') }}</h3><p class="text-sm text-red-500/90 break-words mt-3">{{ selectedRecord.errorMessage }}</p></section>
            <section class="detail-section"><div class="flex items-center justify-between"><h3><i class="pi pi-list"></i>{{ appStore.t('history.detail.logs') }}</h3><span class="text-xs text-dim">{{ selectedRecord.logs.length }}</span></div><div v-if="selectedRecord.logs.length" class="space-y-2 mt-3"><div v-for="(log, index) in selectedRecord.logs" :key="`${log.timestamp}-${index}`" class="detail-log"><time>{{ formatDateTime(log.timestamp) }}</time><span :class="log.severity === 'error' ? 'text-red-500' : log.severity === 'success' ? 'text-emerald-500' : 'text-content'">{{ log.message }}</span></div></div><p v-else class="text-sm text-dim mt-3">{{ appStore.t('history.detail.no_logs') }}</p></section>
            <button type="button" class="w-full mt-5 py-3 rounded-xl border border-red-500/20 text-red-500 font-black hover:bg-red-500/10 transition-colors" @click="removeSelectedRecord"><i class="pi pi-trash mr-2"></i>{{ appStore.t('history.delete_record') }}</button>
          </aside>
        </div>
      </transition>
      <div v-if="showClearConfirm" class="fixed inset-0 z-[370] bg-slate-950/35 backdrop-blur-sm flex items-center justify-center p-5" @click.self="showClearConfirm = false"><div class="history-detail w-full max-w-md rounded-3xl p-6"><div class="w-12 h-12 rounded-2xl bg-red-500/10 text-red-500 flex items-center justify-center"><i class="pi pi-trash"></i></div><h2 class="text-xl font-black mt-4">{{ appStore.t('history.clear_confirm_title') }}</h2><p class="text-sm text-muted mt-2">{{ appStore.t('history.clear_confirm_hint') }}</p><div class="grid grid-cols-2 gap-3 mt-6"><button class="history-ghost-button justify-center" @click="showClearConfirm = false">{{ appStore.t('common.cancel') }}</button><button class="rounded-xl bg-red-500 text-white font-black" @click="confirmClear">{{ appStore.t('history.clear') }}</button></div></div></div>
    </Teleport>
  </div>
</template>

<style scoped>
.history-panel,.history-detail{background:color-mix(in srgb,var(--card-bg) 92%,transparent);border:1px solid var(--border-color);box-shadow:0 18px 45px rgba(22,30,55,.08);backdrop-filter:blur(24px)}
.history-panel{border-radius:1.5rem}.history-detail{background:var(--card-bg)}
.history-kpi{min-width:0;padding:1.05rem;border-radius:1.35rem;border:1px solid var(--border-color);background:linear-gradient(145deg,color-mix(in srgb,var(--card-bg) 94%,transparent),color-mix(in srgb,var(--dynamic-accent) 4%,var(--card-bg)));box-shadow:0 12px 28px rgba(20,30,60,.06)}
.history-kpi-primary{background:linear-gradient(145deg,color-mix(in srgb,var(--dynamic-accent) 16%,var(--card-bg)),var(--card-bg))}.history-kpi>div{display:flex;justify-content:space-between;gap:.5rem;color:var(--text-muted);font-size:.7rem;font-weight:900;text-transform:uppercase;letter-spacing:.1em}.history-kpi strong{display:block;font-size:2rem;line-height:1.05;margin-top:.75rem;color:var(--text-content);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.history-kpi em{font-size:.85rem;font-style:normal;color:var(--text-muted);margin-left:.15rem}.history-kpi small{display:block;color:var(--text-dim);font-size:.65rem;margin-top:.4rem;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.history-ghost-button{min-height:2.6rem;padding:.65rem .9rem;border:1px solid var(--border-color);border-radius:.85rem;background:var(--input-bg);display:flex;align-items:center;gap:.5rem;font-size:.75rem;font-weight:900;transition:.2s}.history-ghost-button:hover:not(:disabled){border-color:color-mix(in srgb,var(--dynamic-accent) 45%,var(--border-color));color:var(--dynamic-accent)}.history-ghost-button:disabled{opacity:.45}
.history-control{height:2.75rem;min-width:0;border:1px solid var(--border-color);border-radius:.9rem;background:var(--input-bg);color:var(--text-content);font-size:.75rem;font-weight:800;padding:.65rem .85rem;outline:none}.history-control:focus{border-color:var(--dynamic-accent);box-shadow:0 0 0 3px color-mix(in srgb,var(--dynamic-accent) 12%,transparent)}
.history-chart{background:repeating-linear-gradient(to bottom,transparent 0,transparent calc(33.33% - 1px),var(--border-color) 33.33%);border-radius:1rem;padding:.5rem .2rem 0}.history-row{grid-template-columns:minmax(0,1.7fr) minmax(10rem,.8fr) minmax(6rem,.5fr) minmax(12rem,.9fr)}.history-chip{padding:.3rem .55rem;border-radius:.55rem;background:var(--input-bg);border:1px solid var(--border-color);font-weight:800}
.history-icon-button{width:2.6rem;height:2.6rem;flex:none;border-radius:.9rem;border:1px solid var(--border-color);background:var(--input-bg);color:var(--text-muted)}.detail-metric,.detail-section{border:1px solid var(--border-color);background:var(--input-bg);border-radius:1.1rem;padding:1rem;min-width:0}.detail-metric span{display:block;color:var(--text-muted);font-size:.68rem;font-weight:900;text-transform:uppercase;letter-spacing:.08em}.detail-metric strong{display:block;margin-top:.45rem;font-size:.9rem;overflow:hidden;text-overflow:ellipsis}.detail-section{margin-top:.75rem}.detail-section h3{font-size:.75rem;font-weight:900;display:flex;align-items:center;gap:.55rem}.detail-path{display:block;width:100%;font-size:.72rem;color:var(--text-muted);white-space:pre-wrap;overflow-wrap:anywhere;background:color-mix(in srgb,var(--card-bg) 70%,transparent);border-radius:.65rem;padding:.65rem}.detail-log{display:grid;grid-template-columns:5.5rem minmax(0,1fr);gap:.7rem;font-size:.7rem}.detail-log time{color:var(--text-dim)}.detail-log span{overflow-wrap:anywhere}
.history-drawer-enter-active,.history-drawer-leave-active{transition:opacity .22s}.history-drawer-enter-active .history-detail,.history-drawer-leave-active .history-detail{transition:transform .3s cubic-bezier(.2,.8,.2,1)}.history-drawer-enter-from,.history-drawer-leave-to{opacity:0}.history-drawer-enter-from .history-detail,.history-drawer-leave-to .history-detail{transform:translateX(100%)}
@media(max-width:1050px){.history-row{grid-template-columns:minmax(0,1.5fr) minmax(5rem,.5fr) minmax(11rem,.8fr)}.history-row>div:nth-child(2){display:none}}
@media(max-width:700px){.history-row{grid-template-columns:minmax(0,1fr) auto}.history-row>div:nth-child(3){display:none}.history-kpi{padding:.85rem}.history-kpi strong{font-size:1.45rem}.history-kpi small{display:none}}
@media(prefers-reduced-motion:reduce){.history-chart *,.history-drawer-enter-active .history-detail,.history-drawer-leave-active .history-detail{transition:none!important}}
</style>
