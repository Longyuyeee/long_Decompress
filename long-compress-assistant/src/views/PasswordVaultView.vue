<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import {
  usePasswordStore,
  type PasswordEntry,
} from '@/stores/password'
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'
import PasswordEntryModal from '@/components/passwords/PasswordEntryModal.vue'

const passwordStore = usePasswordStore()
const appStore = useAppStore()
const tauriCommands = useTauriCommands()

const showAddModal = ref(false)
const showHistoryModal = ref(false)
const showClearConfirm = ref(false)
const showDeleteConfirm = ref(false)
const deleteTargetId = ref<string | null>(null)
const selectedEntryForHistory = ref<any>(null)
const searchQuery = ref('')
const showAllPasswords = ref(false)
const visiblePasswordIds = ref<Set<string>>(new Set())
type AnalyticsRange = '7d' | '30d' | '90d' | 'all'
const analyticsRange = ref<AnalyticsRange>('30d')
const analyticsRanges: Array<{ id: AnalyticsRange; labelKey: string }> = [
  { id: '7d', labelKey: 'vault.analytics.range_7d' },
  { id: '30d', labelKey: 'vault.analytics.range_30d' },
  { id: '90d', labelKey: 'vault.analytics.range_90d' },
  { id: 'all', labelKey: 'vault.analytics.range_all' },
]

type VaultEntryWithHistory = Omit<PasswordEntry, 'usage_history'> & {
  usage_history?: Record<string, number>
}

const DAY_MS = 86_400_000
const safeDate = (value?: string | null) => {
  if (!value) return null
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? null : date
}

const daysSince = (value?: string | null) => {
  const date = safeDate(value)
  return date ? Math.max(0, Math.floor((Date.now() - date.getTime()) / DAY_MS)) : null
}

const formatDate = (value?: string | null) => {
  const date = safeDate(value)
  return date
    ? new Intl.DateTimeFormat(appStore.language || 'zh-CN', { year: 'numeric', month: 'short', day: 'numeric' }).format(date)
    : appStore.t('vault.analytics.never')
}

onMounted(async () => {
  await passwordStore.checkUnlockStatus()
})

const stats = computed(() => {
  const entries = passwordStore.entries
  const total = entries.length
  const totalUsage = entries.reduce((sum, entry) => sum + (entry.use_count || 0), 0)
  const active30 = entries.filter(entry => {
    const age = daysSince(entry.last_used)
    return age !== null && age <= 30
  }).length
  const neverUsed = entries.filter(entry => !entry.last_used && !entry.use_count).length
  const favorites = entries.filter(entry => entry.favorite).length
  const documented = entries.filter(entry => Boolean(entry.notes?.trim()) || Boolean(entry.tags?.length) || Boolean(entry.category)).length

  return {
    total,
    totalUsage,
    active30,
    neverUsed,
    favorites,
    documented,
  }
})

const filteredAndSortedEntries = computed(() => {
  let result = [...passwordStore.entries]
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(e => 
      (e.name?.toLowerCase() || '').includes(q) || 
      (e.notes?.toLowerCase() || '').includes(q) ||
      (e.password?.toLowerCase() || '').includes(q)
    )
  }
  return result.sort((a, b) => (b.use_count || 0) - (a.use_count || 0))
})

const editingEntry = ref<any>(null)

const handleEdit = (entry: any) => {
  editingEntry.value = JSON.parse(JSON.stringify(entry))
  showAddModal.value = true
}

const handleAddNew = () => {
  editingEntry.value = null
  showAddModal.value = true
}

const handleDelete = (id: string) => {
  deleteTargetId.value = id
  showDeleteConfirm.value = true
}

const confirmDelete = async () => {
  showDeleteConfirm.value = false
  if (!deleteTargetId.value) return
  try {
    await passwordStore.deleteEntry(deleteTargetId.value)
    appStore.setSuccess(appStore.t('common.success'))
  } catch (e) {
    appStore.setError(appStore.t('common.error'))
  }
  deleteTargetId.value = null
}

const isExporting = ref(false)
const isImporting = ref(false)

const handleExport = async () => {
  if (passwordStore.entries.length === 0) {
    appStore.setError(appStore.t('vault.export.empty'))
    return
  }

  try {
    isExporting.value = true
    await tauriCommands.exportPasswords()
  } catch (error) {
    appStore.setError(appStore.t('vault.export.failed'))
  } finally {
    isExporting.value = false
  }
}

const handleImport = async () => {
  try {
    isImporting.value = true
    await tauriCommands.importPasswords()
    await passwordStore.fetchAllData()
    appStore.setSuccess(appStore.t('vault.import.success'))
  } catch (error) {
    appStore.setError(appStore.t('vault.import.failed'))
  } finally {
    isImporting.value = false
  }
}

const confirmClearAll = async () => {
  showClearConfirm.value = false
  try {
    await passwordStore.clearAll()
    appStore.setSuccess(appStore.t('common.success'))
  } catch (e) {
    appStore.setError(appStore.t('common.error'))
  }
}

const showUsageHistory = async (entry: PasswordEntry) => {
  await passwordStore.fetchAllData()
  selectedEntryForHistory.value = passwordStore.entries.find(item => item.id === entry.id) || entry
  showHistoryModal.value = true
}

const showVaultAnalytics = async () => {
  // 自动解压可能刚刚在后端命中密码；打开统计前重新读取真实文件数据。
  await passwordStore.fetchAllData()
  selectedEntryForHistory.value = null
  showHistoryModal.value = true
}

const isPasswordVisible = (id: string) => {
  return showAllPasswords.value || visiblePasswordIds.value.has(id)
}

const maskPassword = (entry: { id: string; password: string }) => {
  if (isPasswordVisible(entry.id)) return entry.password
  if (!entry.password) return '——'
  return '•'.repeat(Math.min(entry.password.length, 16))
}

const togglePasswordVisibility = () => {
  showAllPasswords.value = !showAllPasswords.value
  if (showAllPasswords.value) visiblePasswordIds.value = new Set()
}

const toggleEntryPasswordVisibility = (id: string) => {
  const next = new Set(visiblePasswordIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  visiblePasswordIds.value = next
}

const copyPasswordToClipboard = async (entry: PasswordEntry) => {
  const password = await passwordStore.usePassword(entry.id)
  await navigator.clipboard.writeText(password)
  appStore.setSuccess(appStore.t('vault.action.copy'))
}

const usageEvents = computed(() => {
  const events: Array<{ date: Date; count: number }> = []
  ;(passwordStore.entries as VaultEntryWithHistory[]).forEach(entry => {
    const history = entry.usage_history
    const historyItems = Object.entries(history || {})
    if (historyItems.length) {
      historyItems.forEach(([date, count]) => {
        const parsed = safeDate(`${date}T00:00:00`)
        if (parsed && count > 0) events.push({ date: parsed, count })
      })
      return
    }
    const fallback = safeDate(entry.last_used)
    if (fallback) events.push({ date: fallback, count: 1 })
  })
  return events
})

const buildFixedRangeBuckets = (days: number, bucketDays: number) => {
  const today = new Date()
  const endDate = new Date(today.getFullYear(), today.getMonth(), today.getDate() + 1)
  const startDate = new Date(endDate)
  startDate.setDate(startDate.getDate() - days)
  const bucketCount = Math.ceil(days / bucketDays)
  return Array.from({ length: bucketCount }, (_, index) => {
    const bucketStartDate = new Date(startDate)
    bucketStartDate.setDate(bucketStartDate.getDate() + index * bucketDays)
    const bucketEndDate = new Date(bucketStartDate)
    bucketEndDate.setDate(bucketEndDate.getDate() + bucketDays)
    const bucketStart = bucketStartDate.getTime()
    const bucketEnd = Math.min(endDate.getTime(), bucketEndDate.getTime())
    const count = usageEvents.value.reduce(
      (sum, event) => sum + (event.date.getTime() >= bucketStart && event.date.getTime() < bucketEnd ? event.count : 0),
      0,
    )
    return {
      date: `${String(bucketStartDate.getMonth() + 1).padStart(2, '0')}-${String(bucketStartDate.getDate()).padStart(2, '0')}`,
      count,
    }
  })
}

const buildLifetimeBuckets = () => {
  const dateCandidates = [
    ...usageEvents.value.map(event => event.date),
    ...passwordStore.entries.map(entry => safeDate(entry.created_at)).filter((date): date is Date => Boolean(date)),
  ]
  const now = new Date()
  const earliest = dateCandidates.length
    ? new Date(Math.min(...dateCandidates.map(date => date.getTime())))
    : now
  const firstMonth = new Date(earliest.getFullYear(), earliest.getMonth(), 1)
  const nextMonth = new Date(now.getFullYear(), now.getMonth() + 1, 1)
  const totalMonths = Math.max(
    1,
    (now.getFullYear() - earliest.getFullYear()) * 12 + now.getMonth() - earliest.getMonth() + 1,
  )
  const monthsPerBucket = Math.max(1, Math.ceil(totalMonths / 10))
  const bucketCount = Math.ceil(totalMonths / monthsPerBucket)

  return Array.from({ length: bucketCount }, (_, index) => {
    const startDate = new Date(firstMonth)
    startDate.setMonth(startDate.getMonth() + index * monthsPerBucket)
    const endDate = new Date(firstMonth)
    endDate.setMonth(endDate.getMonth() + Math.min(totalMonths, (index + 1) * monthsPerBucket))
    const bucketEnd = index === bucketCount - 1
      ? nextMonth.getTime()
      : endDate.getTime()
    const count = usageEvents.value.reduce(
      (sum, event) => sum + (event.date.getTime() >= startDate.getTime() && event.date.getTime() < bucketEnd ? event.count : 0),
      0,
    )
    const year = String(startDate.getFullYear()).slice(-2)
    const month = String(startDate.getMonth() + 1).padStart(2, '0')
    return { date: `${year}-${month}`, count }
  })
}

const usageTrend = computed(() => {
  const buckets = analyticsRange.value === '7d'
    ? buildFixedRangeBuckets(7, 1)
    : analyticsRange.value === '30d'
      ? buildFixedRangeBuckets(30, 5)
      : analyticsRange.value === '90d'
        ? buildFixedRangeBuckets(90, 15)
        : buildLifetimeBuckets()
  const maxCount = Math.max(...buckets.map(bucket => bucket.count), 1)
  return buckets.map(bucket => ({
    ...bucket,
    height: Math.round((bucket.count / maxCount) * 100),
  }))
})

const usageTrendCoordinates = computed(() =>
  usageTrend.value.map((day, index) => ({
    ...day,
    x: index * (100 / Math.max(usageTrend.value.length - 1, 1)),
    y: 36 - (day.height / 100) * 28,
  }))
)

const usageTrendPoints = computed(() =>
  usageTrendCoordinates.value.map(point => `${point.x},${point.y}`).join(' ')
)

const rangeUsageTotal = computed(() =>
  usageTrend.value.reduce((sum, bucket) => sum + bucket.count, 0)
)

const lifetimeStats = computed(() => {
  const createdDates = passwordStore.entries
    .map(entry => safeDate(entry.created_at))
    .filter((date): date is Date => Boolean(date))
  const now = new Date()
  const earliest = createdDates.length
    ? new Date(Math.min(...createdDates.map(date => date.getTime())))
    : null
  const vaultAgeDays = earliest ? Math.max(0, Math.floor((now.getTime() - earliest.getTime()) / DAY_MS)) : 0
  const activeMonths = earliest
    ? Math.max(1, (now.getFullYear() - earliest.getFullYear()) * 12 + now.getMonth() - earliest.getMonth() + 1)
    : 1
  const monthlyAverage = Math.round((stats.value.totalUsage / activeMonths) * 10) / 10
  const monthlyCounts = usageEvents.value.reduce<Record<string, number>>((result, event) => {
    const key = `${event.date.getFullYear()}-${String(event.date.getMonth() + 1).padStart(2, '0')}`
    result[key] = (result[key] || 0) + event.count
    return result
  }, {})
  const peakMonth = Object.entries(monthlyCounts).sort(([, a], [, b]) => b - a)[0]

  return {
    vaultAgeDays,
    firstCreated: earliest ? formatDate(earliest.toISOString()) : appStore.t('vault.analytics.never'),
    monthlyAverage,
    peakMonth: peakMonth?.[0] || appStore.t('vault.analytics.never'),
    peakMonthCount: peakMonth?.[1] || 0,
  }
})

const usageTierBreakdown = computed(() => {
  const total = Math.max(passwordStore.entries.length, 1)
  const tiers = [
    { key: 'never', labelKey: 'vault.analytics.tier_never', color: '#94a3b8', matches: (count: number) => count === 0 },
    { key: 'occasional', labelKey: 'vault.analytics.tier_occasional', color: '#38bdf8', matches: (count: number) => count >= 1 && count <= 2 },
    { key: 'regular', labelKey: 'vault.analytics.tier_regular', color: '#8b5cf6', matches: (count: number) => count >= 3 && count <= 9 },
    { key: 'frequent', labelKey: 'vault.analytics.tier_frequent', color: '#22c55e', matches: (count: number) => count >= 10 },
  ]
  return tiers.map(item => {
    const count = passwordStore.entries.filter(entry => item.matches(entry.use_count || 0)).length
    return { ...item, count, ratio: count / total, percent: Math.round((count / total) * 100) }
  })
})

const usageTierGradient = computed(() => {
  if (!passwordStore.entries.length) return 'conic-gradient(#334155 0deg 360deg)'
  let cursor = 0
  const stops = usageTierBreakdown.value.map((item, index) => {
    const start = cursor
    cursor = index === usageTierBreakdown.value.length - 1 ? 360 : cursor + item.ratio * 360
    return `${item.color} ${start}deg ${cursor}deg`
  })
  return `conic-gradient(${stops.join(', ')})`
})

const categoryBreakdown = computed(() => {
  const counts = passwordStore.entries.reduce<Record<string, number>>((result, entry) => {
    const key = entry.category || 'Other'
    result[key] = (result[key] || 0) + 1
    return result
  }, {})
  const max = Math.max(...Object.values(counts), 1)
  return Object.entries(counts)
    .sort(([, countA], [, countB]) => countB - countA)
    .slice(0, 6)
    .map(([label, count]) => ({ label, count, percent: Math.round((count / max) * 100) }))
})

const mostUsedEntries = computed(() =>
  [...passwordStore.entries]
    .sort((a, b) => (b.use_count || 0) - (a.use_count || 0))
    .slice(0, 5)
)

const hitRecencyBreakdown = computed(() => {
  const buckets = [
    { key: 'week', labelKey: 'vault.analytics.hit_week', min: 0, max: 7, color: '#22c55e' },
    { key: 'month', labelKey: 'vault.analytics.hit_month', min: 8, max: 30, color: '#38bdf8' },
    { key: 'older', labelKey: 'vault.analytics.hit_older', min: 31, max: Number.POSITIVE_INFINITY, color: '#8b5cf6' },
    { key: 'never', labelKey: 'vault.analytics.hit_never', min: -1, max: -1, color: '#94a3b8' },
  ]
  const counts = buckets.map(bucket => ({
    ...bucket,
    count: passwordStore.entries.filter(entry => {
      const age = daysSince(entry.last_used)
      if (bucket.key === 'never') return age === null
      return age !== null && age >= bucket.min && age <= bucket.max
    }).length,
  }))
  const max = Math.max(...counts.map(bucket => bucket.count), 1)
  return counts.map(bucket => ({ ...bucket, percent: Math.round((bucket.count / max) * 100) }))
})

const activityHeatmap = computed(() => {
  const today = new Date()
  const todayStart = new Date(today.getFullYear(), today.getMonth(), today.getDate())
  const localDateKey = (date: Date) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
  const dailyCounts = usageEvents.value.reduce<Record<string, number>>((result, event) => {
    const key = localDateKey(event.date)
    result[key] = (result[key] || 0) + event.count
    return result
  }, {})
  const cells = Array.from({ length: 35 }, (_, index) => {
    const date = new Date(todayStart)
    date.setDate(date.getDate() - (34 - index))
    const key = localDateKey(date)
    return { key, label: formatDate(date.toISOString()), count: dailyCounts[key] || 0 }
  })
  const max = Math.max(...cells.map(cell => cell.count), 1)
  return cells.map(cell => ({ ...cell, level: cell.count ? Math.max(1, Math.ceil((cell.count / max) * 4)) : 0 }))
})

const recentlyActiveEntries = computed(() => [...passwordStore.entries]
  .filter(entry => safeDate(entry.last_used))
  .sort((a, b) => (safeDate(b.last_used)?.getTime() || 0) - (safeDate(a.last_used)?.getTime() || 0))
  .slice(0, 4))

const insightCards = computed(() => {
  const insights: Array<{ icon: string; tone: string; titleKey: string; detailKey: string; count: number }> = []
  const missingContext = passwordStore.entries.filter(entry => !entry.notes?.trim() && !entry.tags?.length && !entry.category).length
  if (stats.value.neverUsed) insights.push({ icon: 'pi pi-question-circle', tone: 'info', titleKey: 'vault.analytics.insight_unused_title', detailKey: 'vault.analytics.insight_unused_detail', count: stats.value.neverUsed })
  if (missingContext) insights.push({ icon: 'pi pi-tags', tone: 'warning', titleKey: 'vault.analytics.insight_context_title', detailKey: 'vault.analytics.insight_context_detail', count: missingContext })
  if (stats.value.favorites) insights.push({ icon: 'pi pi-star', tone: 'success', titleKey: 'vault.analytics.insight_favorite_title', detailKey: 'vault.analytics.insight_favorite_detail', count: stats.value.favorites })
  if (stats.value.active30) insights.push({ icon: 'pi pi-bolt', tone: 'success', titleKey: 'vault.analytics.insight_active_title', detailKey: 'vault.analytics.insight_active_detail', count: stats.value.active30 })
  if (!insights.length) insights.push({ icon: 'pi pi-inbox', tone: 'info', titleKey: 'vault.analytics.insight_empty_title', detailKey: 'vault.analytics.insight_empty_detail', count: stats.value.total })
  return insights.slice(0, 4)
})

const selectedProfile = computed(() => {
  const entry = selectedEntryForHistory.value as VaultEntryWithHistory | null
  if (!entry) return null
  return {
    created: formatDate(entry.created_at),
    updated: formatDate(entry.updated_at),
    lastUsed: formatDate(entry.last_used),
    daysStored: daysSince(entry.created_at) || 0,
    useCount: entry.use_count || 0,
    category: entry.category || appStore.t('vault.analytics.uncategorized'),
    notes: entry.notes?.trim() || appStore.t('vault.analytics.no_notes'),
    tags: entry.tags || [],
    favorite: Boolean(entry.favorite),
  }
})

const selectedUsageEvents = computed(() => {
  const entry = selectedEntryForHistory.value as VaultEntryWithHistory | null
  if (!entry) return []
  const history = Object.entries(entry.usage_history || {})
    .map(([date, count]) => ({ date: safeDate(`${date}T00:00:00`), count }))
    .filter((item): item is { date: Date; count: number } => Boolean(item.date) && item.count > 0)
  if (history.length) return history
  const fallback = safeDate(entry.last_used)
  // 旧条目只有最后命中时间而没有逐日历史，最多确认当天发生过一次，不把累计次数伪造到同一天。
  return fallback ? [{ date: fallback, count: 1 }] : []
})

const selectedUsageDays = computed(() => {
  const today = new Date()
  const localKey = (date: Date) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
  const counts = selectedUsageEvents.value.reduce<Record<string, number>>((result, item) => {
    result[localKey(item.date)] = (result[localKey(item.date)] || 0) + item.count
    return result
  }, {})
  return Array.from({ length: 14 }, (_, index) => {
    const date = new Date(today.getFullYear(), today.getMonth(), today.getDate() - (13 - index))
    const key = localKey(date)
    return { key, label: `${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`, count: counts[key] || 0 }
  })
})

const selectedUsageMax = computed(() => Math.max(...selectedUsageDays.value.map(day => day.count), 1))
</script>

<template>
  <div class="password-vault p-responsive p-8 h-screen flex flex-col gap-6 transition-colors duration-700 relative overflow-hidden">
    <header class="flex justify-between items-center gap-6 shrink-0">
      <div class="flex items-center gap-6 shrink-0">
        <div>
          <h1 class="text-3xl font-black text-content tracking-tighter mb-0.5">{{ appStore.t('nav.vault') }}</h1>
          <p class="text-muted text-xs font-bold uppercase tracking-[0.2em] ml-0.5">{{ appStore.t('vault.usage_stats') }}</p>
        </div>
        
        <div class="flex gap-2">
          <button v-if="passwordStore.isUnlocked" @click="handleAddNew" :aria-label="appStore.t('common.add')" class="w-9 h-9 rounded-xl bg-primary text-white flex items-center justify-center hover:scale-105 transition-all shadow-lg hover:shadow-primary/40">
            <i class="pi pi-plus text-sm"></i>
          </button>
          <div class="w-px h-5 bg-subtle my-auto mx-1"></div>
          <button v-if="passwordStore.isUnlocked" @click="handleExport" :disabled="isExporting || passwordStore.entries.length === 0" class="w-8 h-8 rounded-lg bg-input border border-subtle text-muted flex items-center justify-center hover:text-primary hover:bg-primary/5 transition-all disabled:opacity-85 disabled:cursor-not-allowed" :title="appStore.t('vault.export')" aria-label="Export passwords">
            <i v-if="!isExporting" class="pi pi-download text-sm"></i>
            <i v-else class="pi pi-spin pi-spinner text-sm"></i>
          </button>
          <button v-if="passwordStore.isUnlocked" @click="handleImport" :disabled="isImporting" class="w-8 h-8 rounded-lg bg-input border border-subtle text-muted flex items-center justify-center hover:text-primary hover:bg-primary/5 transition-all disabled:opacity-85 disabled:cursor-not-allowed" :title="appStore.t('vault.import')" aria-label="Import passwords">
            <i v-if="!isImporting" class="pi pi-upload text-sm"></i>
            <i v-else class="pi pi-spin pi-spinner text-sm"></i>
          </button>
          <button v-if="passwordStore.isUnlocked" @click="showClearConfirm = true" class="w-8 h-8 rounded-lg bg-input border border-subtle text-muted flex items-center justify-center hover:text-red-500 transition-all" :title="appStore.t('vault.clear_all')" aria-label="Clear all passwords">
            <i class="pi pi-trash text-sm"></i>
          </button>
        </div>
      </div>

      <div class="flex-1 flex justify-end">
        <div class="relative w-full max-w-[280px] group">
          <i class="pi pi-search absolute left-4 top-1/2 -translate-y-1/2 text-dim text-sm group-hover:text-primary transition-colors"></i>
          <input v-model="searchQuery" type="text" :disabled="!passwordStore.isUnlocked" :placeholder="appStore.t('common.search')" class="w-full bg-input border border-subtle rounded-xl pl-10 pr-4 py-2.5 text-sm text-content focus:outline-none focus:border-primary transition-all shadow-sm placeholder:text-dim disabled:opacity-60 disabled:cursor-not-allowed">
        </div>
      </div>
    </header>

    <div class="relative flex-1 min-h-0 aero-card overflow-hidden flex flex-col mb-12">
      <div v-if="passwordStore.isInitialized && passwordStore.isLoading" class="absolute inset-0 z-50 bg-card/80 backdrop-blur-sm flex items-center justify-center">
        <i class="pi pi-spin pi-spinner text-primary text-2xl"></i>
      </div>

      <div v-if="!passwordStore.isInitialized" class="flex-1 p-6" aria-busy="true" aria-label="Loading password vault">
        <div class="grid grid-cols-[26%_26%_26%_10%_12%] gap-4 border-b border-subtle pb-4">
          <div v-for="index in 5" :key="`vault-head-${index}`" class="h-3 rounded-full bg-input animate-pulse"></div>
        </div>
        <div v-for="row in 5" :key="`vault-row-${row}`" class="grid grid-cols-[26%_26%_26%_10%_12%] gap-4 border-b border-subtle/50 py-5">
          <div v-for="column in 5" :key="`vault-cell-${row}-${column}`" class="h-4 rounded-lg bg-input/80 animate-pulse" :style="{ opacity: 1 - row * 0.12 }"></div>
        </div>
      </div>

      <!-- 自动初始化失败状态：保留本机加密，但不再要求用户设置或输入主密码 -->
      <div v-else-if="!passwordStore.isUnlocked" class="flex-1 flex items-center justify-center">
        <div class="text-center space-y-5">
          <div class="w-16 h-16 mx-auto rounded-full bg-red-500/10 border border-red-500/20 flex items-center justify-center">
            <i class="pi pi-exclamation-circle text-2xl text-red-400"></i>
          </div>
          <div>
            <p class="text-sm font-black text-content">{{ appStore.t('vault.unavailable') }}</p>
            <p class="text-xs text-muted mt-1">{{ passwordStore.errorMessage || appStore.t('vault.init_failed') }}</p>
          </div>
          <button @click="passwordStore.retryInitialization()" class="px-6 py-2.5 rounded-xl bg-primary text-white text-sm font-black uppercase tracking-widest hover:brightness-110 transition-all shadow-lg">
            <i class="pi pi-refresh mr-2 text-xs"></i>{{ appStore.t('vault.retry_init') }}
          </button>
        </div>
      </div>

      <div v-else class="flex-1 overflow-hidden flex flex-col relative">
        <table class="w-full text-left border-collapse table-fixed">
          <colgroup>
            <col class="vault-col-name">
            <col class="vault-col-password">
            <col class="vault-col-notes">
            <col class="vault-col-usage">
            <col class="vault-col-actions">
          </colgroup>
          <thead class="sticky top-0 z-20 bg-input/80 backdrop-blur-xl border-b border-subtle">
            <tr>
              <th data-testid="vault-name-header" class="px-4 py-4 text-xs font-black text-muted uppercase tracking-[0.16em] whitespace-nowrap">{{ appStore.t('vault.column.name') }}</th>
              <th data-testid="vault-password-header" class="px-4 py-4 text-xs font-black text-muted uppercase tracking-[0.16em]">
                <button @click="togglePasswordVisibility" class="flex items-center gap-1.5 hover:text-primary transition-colors">
                  {{ appStore.t('vault.column.password') }}
                  <i :class="showAllPasswords ? 'pi pi-eye-slash' : 'pi pi-eye'" class="text-sm"></i>
                </button>
              </th>
              <th data-testid="vault-notes-header" class="px-4 py-4 text-xs font-black text-muted uppercase tracking-[0.16em] whitespace-nowrap">{{ appStore.t('vault.column.notes') }}</th>
              <th data-testid="vault-usage-header" class="px-2 py-4 text-xs font-black text-muted uppercase tracking-[0.08em] text-center whitespace-nowrap">{{ appStore.t('vault.column.usage') }}</th>
              <th class="px-3 py-4 text-xs font-black text-muted uppercase tracking-[0.12em] text-right whitespace-nowrap">{{ appStore.t('vault.column.actions') }}</th>
            </tr>
          </thead>
        </table>
        
        <div class="flex-1 overflow-y-auto custom-scrollbar">
          <!-- 空状态提示 -->
          <div v-if="filteredAndSortedEntries.length === 0 && !passwordStore.isLoading" class="flex flex-col items-center justify-center h-full text-center gap-4">
            <i class="pi pi-shield text-4xl text-muted/30"></i>
            <p class="text-muted text-xs font-bold">{{ appStore.t('vault.empty') }}</p>
          </div>
          <table v-else class="w-full text-left border-collapse table-fixed">
            <colgroup>
              <col class="vault-col-name">
              <col class="vault-col-password">
              <col class="vault-col-notes">
              <col class="vault-col-usage">
              <col class="vault-col-actions">
            </colgroup>
            <tbody class="divide-y divide-subtle/50">
              <tr v-for="(entry, index) in filteredAndSortedEntries" :key="entry.id" class="hover:bg-primary/[0.03] group transition-all">
                <td class="px-4 py-3.5">
                  <div class="flex items-center gap-2 relative group/tooltip">
                    <div class="w-1.5 h-1.5 rounded-full bg-primary/40 group-hover:bg-primary transition-colors shrink-0"></div>
                    <span class="text-sm font-bold text-content truncate block w-full">{{ entry.name }}</span>
                    <!-- 自定义悬浮窗 (Aero Tooltip) - 修复遮挡问题：前两行向下弹出，其余向上弹出 -->
                    <div :class="[
                      'absolute left-0 px-3 py-2 rounded-xl bg-card/90 backdrop-blur-3xl border border-subtle shadow-2xl text-sm text-content whitespace-normal break-all max-w-[200px] z-[100] opacity-0 transition-all pointer-events-none font-bold',
                      index < 2 ? 'top-[110%] mt-1 translate-y-[-8px] group-hover/tooltip:translate-y-0' : 'bottom-[110%] mb-1 translate-y-2 group-hover/tooltip:translate-y-0'
                    ]" class="group-hover/tooltip:opacity-100">
                      {{ entry.name }}
                    </div>
                  </div>
                </td>
                <td class="px-4 py-3.5">
                  <div class="flex items-center gap-2 overflow-hidden group/key w-full">
                    <code class="text-sm font-mono text-primary font-bold bg-primary/5 px-2 py-1 rounded-lg truncate block flex-1 select-none" :class="{ 'tracking-widest': !isPasswordVisible(entry.id) }">{{ maskPassword(entry) }}</code>
                    <button type="button" @click="toggleEntryPasswordVisibility(entry.id)" :aria-label="isPasswordVisible(entry.id) ? appStore.t('vault.action.hide') : appStore.t('vault.action.show')" :title="isPasswordVisible(entry.id) ? appStore.t('vault.action.hide') : appStore.t('vault.action.show')" class="w-7 h-7 rounded-lg flex items-center justify-center text-dim hover:text-primary hover:bg-primary/10 transition-all shrink-0">
                      <i :class="isPasswordVisible(entry.id) ? 'pi pi-eye-slash' : 'pi pi-eye'" class="text-sm"></i>
                    </button>
                    <button type="button" @click="copyPasswordToClipboard(entry)" :aria-label="appStore.t('vault.action.copy_password')" :title="appStore.t('vault.action.copy_password')" class="w-7 h-7 rounded-lg flex items-center justify-center text-dim hover:text-primary hover:bg-primary/10 transition-all shrink-0"><i class="pi pi-copy text-sm"></i></button>
                  </div>
                </td>
                <td class="px-4 py-3.5">
                  <div class="relative group/tooltip w-full">
                    <span class="text-sm text-muted italic truncate block w-full">{{ entry.notes || '—' }}</span>
                    <!-- 自定义悬浮窗 (Aero Tooltip) - 修复遮挡问题：前两行向下弹出，其余向上弹出 -->
                    <div v-if="entry.notes" :class="[
                      'absolute left-0 px-3 py-2 rounded-xl bg-card/90 backdrop-blur-3xl border border-subtle shadow-2xl text-sm text-muted whitespace-normal break-all max-w-[240px] z-[100] opacity-0 transition-all pointer-events-none italic',
                      index < 2 ? 'top-[110%] mt-1 translate-y-[-8px] group-hover/tooltip:translate-y-0' : 'bottom-[110%] mb-1 translate-y-2 group-hover/tooltip:translate-y-0'
                    ]" class="group-hover/tooltip:opacity-100">
                      {{ entry.notes }}
                    </div>
                  </div>
                </td>
                <td class="px-2 py-3.5 text-center">
                  <button data-testid="vault-entry-usage" @click="showUsageHistory(entry)" class="text-xs font-black text-muted hover:text-primary bg-input w-6 h-6 rounded-full flex items-center justify-center mx-auto transition-all shadow-sm border border-subtle shrink-0">
                    {{ entry.use_count || 0 }}
                  </button>
                </td>
                <td class="px-3 py-3.5 text-right">
                  <div class="flex justify-end gap-3 sm:opacity-0 sm:group-hover:opacity-100 transition-all shrink-0">
                    <button @click="handleEdit(entry)" :aria-label="`${appStore.t('common.edit')}: ${entry.name}`" class="w-7 h-7 rounded-lg text-primary/60 hover:text-primary hover:bg-primary/10 transition-colors"><i class="pi pi-pencil text-sm"></i></button>
                    <button @click="handleDelete(entry.id)" :aria-label="`${appStore.t('common.delete')}: ${entry.name}`" class="w-7 h-7 rounded-lg text-red-400/60 hover:text-red-500 hover:bg-red-500/10 transition-colors"><i class="pi pi-trash text-sm"></i></button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- 数据统计入口 -->
    <button
      v-if="passwordStore.isInitialized && passwordStore.isUnlocked"
      type="button"
      data-testid="vault-analytics-trigger"
      class="fixed bottom-8 right-12 z-50 flex items-center gap-6 bg-card/90 border border-subtle px-6 py-3 rounded-2xl shadow-2xl hover:border-primary/40 hover:-translate-y-0.5 transition-all group"
      :aria-label="appStore.t('vault.analytics.open')"
      @click="showVaultAnalytics"
    >
      <div class="text-center">
        <div class="text-sm text-muted font-black uppercase tracking-widest mb-0.5">{{ appStore.t('vault.vault_size') }}</div>
        <div class="text-lg font-black text-primary leading-none">{{ stats.total }}</div>
      </div>
      <div class="w-px h-6 bg-subtle my-auto"></div>
      <div class="text-center">
        <div class="text-sm text-muted font-black uppercase tracking-widest mb-0.5">{{ appStore.t('vault.total_hits') }}</div>
        <div class="text-lg font-black text-content leading-none">{{ stats.totalUsage }}</div>
      </div>
      <div class="w-9 h-9 rounded-xl bg-primary/10 text-primary flex items-center justify-center group-hover:bg-primary group-hover:text-white transition-all">
        <i class="pi pi-chart-pie text-sm"></i>
      </div>
    </button>

    <Teleport to="body">
      <transition name="analytics-pop">
        <div
          v-if="showHistoryModal"
          data-testid="vault-analytics-modal"
          class="vault-analytics-backdrop fixed inset-0 z-[350] flex items-center justify-center p-4 sm:p-6"
          role="dialog"
          aria-modal="true"
          :aria-label="appStore.t(selectedEntryForHistory ? 'vault.analytics.entry_title' : 'vault.analytics.title')"
          @click.self="showHistoryModal = false"
        >
          <section class="vault-analytics-panel relative w-full max-w-[1120px] max-h-[90vh] rounded-[2rem] overflow-hidden text-content flex flex-col">
            <div class="analytics-orb analytics-orb-primary"></div>
            <div class="analytics-orb analytics-orb-secondary"></div>

            <header class="relative z-10 shrink-0 px-6 sm:px-8 pt-6 pb-5 border-b border-subtle/70 flex items-center justify-between gap-5">
              <div class="flex items-center gap-4 min-w-0">
                <div class="w-11 h-11 rounded-2xl bg-primary/10 border border-primary/20 text-primary flex items-center justify-center shadow-lg shadow-primary/10">
                  <i class="pi pi-chart-bar"></i>
                </div>
                <div class="min-w-0">
                  <p class="text-xs font-black text-primary uppercase tracking-[0.24em]">{{ appStore.t('vault.analytics.eyebrow') }}</p>
                  <h2 class="text-xl sm:text-2xl font-black tracking-tight truncate">
                    {{ appStore.t(selectedEntryForHistory ? 'vault.analytics.entry_title' : 'vault.analytics.title') }}
                  </h2>
                  <p class="text-xs text-muted mt-1">
                    {{ selectedEntryForHistory ? appStore.t('vault.analytics.entry_subtitle') : appStore.t('vault.analytics.subtitle') }}
                  </p>
                </div>
              </div>
              <button
                type="button"
                @click="showHistoryModal = false"
                :aria-label="appStore.t('common.close')"
                class="w-10 h-10 shrink-0 rounded-xl bg-input/70 border border-subtle text-muted hover:text-primary hover:border-primary/40 transition-all"
              >
                <i class="pi pi-times"></i>
              </button>
            </header>

            <div class="relative z-10 flex-1 min-h-0 overflow-y-auto custom-scrollbar p-5 sm:p-8">
              <div v-if="selectedProfile" data-testid="vault-entry-profile" class="space-y-5">
                <article class="analytics-card entry-profile-hero">
                  <div class="flex flex-col lg:flex-row lg:items-start justify-between gap-5">
                    <div class="min-w-0">
                      <p class="analytics-label">{{ appStore.t('vault.analytics.entry_profile') }}</p>
                      <h3 class="text-2xl font-black mt-1 truncate">{{ selectedEntryForHistory.name }}</h3>
                      <p class="text-sm text-muted mt-2">{{ appStore.t('vault.analytics.entry_profile_hint') }}</p>
                    </div>
                    <span class="entry-hit-badge">
                      <i class="pi pi-key"></i>
                      {{ selectedProfile.useCount }} {{ appStore.t('vault.analytics.unlock_hits') }}
                    </span>
                  </div>
                  <div class="grid grid-cols-2 lg:grid-cols-4 gap-3 mt-6">
                    <div class="lifecycle-metric"><span>{{ appStore.t('vault.analytics.total_unlock_hits') }}</span><strong>{{ selectedProfile.useCount }}</strong></div>
                    <div class="lifecycle-metric"><span>{{ appStore.t('vault.analytics.last_hit') }}</span><strong>{{ selectedProfile.lastUsed }}</strong></div>
                    <div class="lifecycle-metric"><span>{{ appStore.t('vault.analytics.created') }}</span><strong>{{ selectedProfile.created }}</strong></div>
                    <div class="lifecycle-metric"><span>{{ appStore.t('vault.analytics.updated') }}</span><strong>{{ selectedProfile.updated }}</strong></div>
                  </div>
                </article>

                <div class="grid grid-cols-1 lg:grid-cols-12 gap-5">
                  <article class="analytics-card lg:col-span-8" data-testid="vault-entry-hit-timeline">
                    <div class="flex items-center justify-between gap-4 mb-5">
                      <div>
                        <p class="analytics-label">{{ appStore.t('vault.analytics.entry_activity') }}</p>
                        <h3 class="analytics-title">{{ appStore.t('vault.analytics.entry_hit_timeline') }}</h3>
                      </div>
                      <span class="text-xs text-muted">{{ appStore.t('vault.analytics.last_14_days') }}</span>
                    </div>
                    <div class="entry-hit-chart">
                      <div v-for="day in selectedUsageDays" :key="day.key" class="entry-hit-column">
                        <strong>{{ day.count }}</strong>
                        <div class="entry-hit-track"><span :style="{ height: `${Math.max(day.count ? 12 : 3, (day.count / selectedUsageMax) * 100)}%` }"></span></div>
                        <small>{{ day.label }}</small>
                      </div>
                    </div>
                  </article>

                  <article class="analytics-card lg:col-span-4">
                    <p class="analytics-label">{{ appStore.t('vault.analytics.entry_status') }}</p>
                    <h3 class="analytics-title mb-5">{{ appStore.t('vault.analytics.archive_context') }}</h3>
                    <div class="space-y-3">
                      <div class="profile-context-row"><span>{{ appStore.t('vault.analytics.category') }}</span><strong>{{ selectedProfile.category }}</strong></div>
                      <div class="profile-context-row"><span>{{ appStore.t('vault.analytics.days_stored') }}</span><strong>{{ selectedProfile.daysStored }} {{ appStore.t('vault.analytics.days') }}</strong></div>
                      <div class="profile-context-row"><span>{{ appStore.t('vault.analytics.favorite') }}</span><strong>{{ selectedProfile.favorite ? appStore.t('common.yes') : appStore.t('common.no') }}</strong></div>
                      <div class="profile-context-row"><span>{{ appStore.t('vault.analytics.tag_count') }}</span><strong>{{ selectedProfile.tags.length }}</strong></div>
                    </div>
                  </article>

                  <article class="analytics-card lg:col-span-12">
                    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                      <div>
                        <p class="analytics-label">{{ appStore.t('vault.analytics.archive_clues') }}</p>
                        <h3 class="analytics-title mb-4">{{ appStore.t('vault.analytics.tags') }}</h3>
                        <div v-if="selectedProfile.tags.length" class="flex flex-wrap gap-2">
                          <span v-for="tag in selectedProfile.tags" :key="tag" class="entry-context-tag">{{ tag }}</span>
                        </div>
                        <p v-else class="text-sm text-muted">{{ appStore.t('vault.analytics.no_tags') }}</p>
                      </div>
                      <div class="lg:border-l border-subtle lg:pl-6">
                        <p class="analytics-label">{{ appStore.t('vault.column.notes') }}</p>
                        <h3 class="analytics-title mb-4">{{ appStore.t('vault.analytics.notes_context') }}</h3>
                        <p class="text-sm text-muted leading-7 whitespace-pre-wrap break-words">{{ selectedProfile.notes }}</p>
                      </div>
                    </div>
                  </article>
                </div>
              </div>

              <template v-else>
              <div class="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-5">
                <article class="analytics-kpi">
                  <div class="flex items-center justify-between">
                    <span>{{ appStore.t('vault.analytics.saved_entries') }}</span>
                    <i class="pi pi-inbox text-emerald-400"></i>
                  </div>
                  <strong>{{ stats.total }}</strong>
                  <p>{{ appStore.t('vault.analytics.saved_entries_hint') }}</p>
                </article>
                <article class="analytics-kpi">
                  <div class="flex items-center justify-between">
                    <span>{{ appStore.t('vault.analytics.active_30') }}</span>
                    <i class="pi pi-bolt text-sky-400"></i>
                  </div>
                  <strong>{{ stats.active30 }}<small>/{{ stats.total }}</small></strong>
                  <p>{{ appStore.t('vault.analytics.active_hint') }}</p>
                </article>
                <article class="analytics-kpi">
                  <div class="flex items-center justify-between">
                    <span>{{ appStore.t('vault.analytics.total_usage') }}</span>
                    <i class="pi pi-chart-line text-violet-400"></i>
                  </div>
                  <strong>{{ stats.totalUsage }}</strong>
                  <p>{{ appStore.t('vault.analytics.total_usage_hint') }}</p>
                </article>
                <article class="analytics-kpi">
                  <div class="flex items-center justify-between">
                    <span>{{ appStore.t('vault.analytics.never_used') }}</span>
                    <i class="pi pi-question-circle text-orange-400"></i>
                  </div>
                  <strong data-testid="vault-unverified-count">{{ stats.neverUsed }}</strong>
                  <p>{{ appStore.t('vault.analytics.never_used_hint') }}</p>
                </article>
              </div>

              <article data-testid="vault-lifetime-overview" class="analytics-card mb-5">
                <div class="flex items-center justify-between gap-4 mb-4">
                  <div>
                    <p class="analytics-label">{{ appStore.t('vault.analytics.lifetime') }}</p>
                    <h3 class="analytics-title">{{ appStore.t('vault.analytics.lifetime_profile') }}</h3>
                  </div>
                  <i class="pi pi-history text-primary"></i>
                </div>
                <div class="grid grid-cols-2 lg:grid-cols-4 gap-3">
                  <div class="lifetime-metric">
                    <span>{{ appStore.t('vault.analytics.vault_age') }}</span>
                    <strong>{{ lifetimeStats.vaultAgeDays }}<small>{{ appStore.t('vault.analytics.days') }}</small></strong>
                  </div>
                  <div class="lifetime-metric">
                    <span>{{ appStore.t('vault.analytics.first_created') }}</span>
                    <strong>{{ lifetimeStats.firstCreated }}</strong>
                  </div>
                  <div class="lifetime-metric">
                    <span>{{ appStore.t('vault.analytics.monthly_average') }}</span>
                    <strong>{{ lifetimeStats.monthlyAverage }}<small>{{ appStore.t('vault.analytics.times_per_month') }}</small></strong>
                  </div>
                  <div class="lifetime-metric">
                    <span>{{ appStore.t('vault.analytics.peak_month') }}</span>
                    <strong>{{ lifetimeStats.peakMonth }}<small>{{ lifetimeStats.peakMonthCount }} {{ appStore.t('vault.analytics.times') }}</small></strong>
                  </div>
                </div>
              </article>

              <div class="grid grid-cols-1 lg:grid-cols-12 gap-5">
                <article class="analytics-card lg:col-span-4">
                  <div class="flex items-center justify-between mb-5">
                    <div>
                      <p class="analytics-label">{{ appStore.t('vault.analytics.hit_spectrum') }}</p>
                      <h3 class="analytics-title">{{ appStore.t('vault.analytics.hit_tier_distribution') }}</h3>
                    </div>
                    <span class="text-xs text-muted">{{ stats.totalUsage }} {{ appStore.t('vault.analytics.unlock_hits') }}</span>
                  </div>
                  <div class="flex items-center gap-6">
                    <div class="usage-donut shrink-0" :style="{ background: usageTierGradient }">
                      <div>
                        <strong>{{ stats.total }}</strong>
                        <span>{{ appStore.t('vault.analytics.items') }}</span>
                      </div>
                    </div>
                    <div class="flex-1 space-y-2.5 min-w-0">
                      <div v-for="item in usageTierBreakdown" :key="item.key" class="flex items-center gap-2 text-xs">
                        <span class="w-2 h-2 rounded-full shrink-0" :style="{ backgroundColor: item.color }"></span>
                        <span class="text-muted flex-1 truncate">{{ appStore.t(item.labelKey) }}</span>
                        <strong>{{ item.count }}</strong>
                      </div>
                    </div>
                  </div>
                </article>

                <article class="analytics-card analytics-trend-card lg:col-span-8">
                  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-2">
                    <div>
                      <p class="analytics-label">{{ appStore.t('vault.analytics.activity') }}</p>
                      <h3 class="analytics-title">{{ appStore.t('vault.analytics.usage_trend') }}</h3>
                    </div>
                    <div class="analytics-range-switch flex items-center gap-1 p-1 rounded-xl bg-input/80 border border-subtle">
                      <button
                        v-for="range in analyticsRanges"
                        :key="range.id"
                        type="button"
                        :data-testid="`vault-range-${range.id}`"
                        class="analytics-range-button px-2.5 py-1.5 rounded-lg text-[10px] font-black"
                        :class="{ 'is-active': analyticsRange === range.id }"
                        @click="analyticsRange = range.id"
                      >
                        {{ appStore.t(range.labelKey) }}
                      </button>
                    </div>
                  </div>
                  <div class="usage-chart relative h-40 mt-4">
                    <div class="chart-grid absolute inset-x-0 top-2 bottom-7 flex flex-col justify-between pointer-events-none">
                      <span v-for="line in 4" :key="line"></span>
                    </div>
                    <svg
                      :key="analyticsRange"
                      class="usage-chart-svg absolute inset-x-0 top-0 w-full h-[125px] overflow-visible"
                      viewBox="0 0 100 40"
                      preserveAspectRatio="none"
                      aria-hidden="true"
                    >
                      <defs>
                        <linearGradient id="usageAreaGradient" x1="0" y1="0" x2="0" y2="1">
                          <stop offset="0%" stop-color="var(--dynamic-accent)" stop-opacity="0.52" />
                          <stop offset="55%" stop-color="var(--dynamic-accent)" stop-opacity="0.13" />
                          <stop offset="100%" stop-color="var(--dynamic-accent)" stop-opacity="0" />
                        </linearGradient>
                        <filter id="usageLineGlow" x="-20%" y="-50%" width="140%" height="200%">
                          <feGaussianBlur stdDeviation="1.5" result="blur" />
                          <feMerge>
                            <feMergeNode in="blur" />
                            <feMergeNode in="SourceGraphic" />
                          </feMerge>
                        </filter>
                      </defs>
                      <polygon class="usage-chart-area" :points="`0,40 ${usageTrendPoints} 100,40`" fill="url(#usageAreaGradient)" />
                      <polyline class="usage-chart-glow" :points="usageTrendPoints" fill="none" stroke="var(--dynamic-accent)" stroke-width="4.5" stroke-opacity="0.2" stroke-linecap="round" stroke-linejoin="round" vector-effect="non-scaling-stroke" />
                      <polyline class="usage-chart-line" :points="usageTrendPoints" fill="none" stroke="var(--dynamic-accent)" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" vector-effect="non-scaling-stroke" pathLength="1" />
                      <g
                        v-for="(point, index) in usageTrendCoordinates"
                        :key="point.date"
                        class="usage-chart-point"
                        :style="{ '--point-delay': `${0.24 + index * 0.055}s` }"
                      >
                        <ellipse :cx="point.x" :cy="point.y" rx="1.2" ry="1.6" fill="var(--dynamic-accent)" fill-opacity="0.16" />
                        <ellipse :cx="point.x" :cy="point.y" rx="0.56" ry="0.72" fill="var(--bg-modal)" stroke="var(--dynamic-accent)" stroke-width="0.42" />
                      </g>
                    </svg>
                    <div
                      class="chart-labels absolute inset-x-0 bottom-0 grid"
                      :style="{ gridTemplateColumns: `repeat(${usageTrend.length}, minmax(0, 1fr))` }"
                    >
                      <div v-for="(day, index) in usageTrend" :key="day.date" class="chart-label text-center" :style="{ '--label-delay': `${0.3 + index * 0.04}s` }">
                        <strong data-testid="vault-usage-day-count" class="block text-xs">{{ day.count }}</strong>
                        <span class="text-[10px] text-muted font-mono">{{ day.date }}</span>
                      </div>
                    </div>
                  </div>
                  <div class="flex items-center justify-between text-[10px] text-muted mt-2">
                    <span>{{ appStore.t('vault.analytics.recorded_activity') }}</span>
                    <strong data-testid="vault-range-usage-total" class="text-primary">{{ rangeUsageTotal }} {{ appStore.t('vault.analytics.times') }}</strong>
                  </div>
                </article>

                <article class="analytics-card lg:col-span-6">
                  <p class="analytics-label">{{ appStore.t('vault.analytics.categories') }}</p>
                  <h3 class="analytics-title mb-5">{{ appStore.t('vault.analytics.category_distribution') }}</h3>
                  <div v-if="categoryBreakdown.length" class="space-y-3">
                    <div v-for="category in categoryBreakdown" :key="category.label" class="grid grid-cols-[90px_1fr_28px] gap-3 items-center">
                      <span class="text-xs text-muted truncate">{{ category.label }}</span>
                      <div class="category-track h-2 bg-input rounded-full overflow-hidden">
                        <div class="category-fill h-full rounded-full bg-gradient-to-r from-primary/60 to-primary" :style="{ width: `${category.percent}%` }"></div>
                      </div>
                      <strong class="text-xs text-right">{{ category.count }}</strong>
                    </div>
                  </div>
                  <p v-else class="text-sm text-muted">{{ appStore.t('vault.analytics.no_data') }}</p>
                </article>

                <article class="analytics-card lg:col-span-6">
                  <div class="grid grid-cols-2 gap-5">
                    <div>
                      <p class="analytics-label">{{ appStore.t('vault.analytics.coverage') }}</p>
                      <h3 class="analytics-title mb-4">{{ appStore.t('vault.analytics.context_coverage') }}</h3>
                      <div class="space-y-2">
                        <div class="risk-row"><span class="risk-dot bg-emerald-500"></span><span>{{ appStore.t('vault.analytics.ever_hit') }}</span><strong>{{ stats.total - stats.neverUsed }}</strong></div>
                        <div class="risk-row"><span class="risk-dot bg-sky-500"></span><span>{{ appStore.t('vault.analytics.active_30') }}</span><strong>{{ stats.active30 }}</strong></div>
                        <div class="risk-row"><span class="risk-dot bg-violet-500"></span><span>{{ appStore.t('vault.analytics.documented') }}</span><strong>{{ stats.documented }}</strong></div>
                        <div class="risk-row"><span class="risk-dot bg-amber-500"></span><span>{{ appStore.t('vault.analytics.favorites') }}</span><strong>{{ stats.favorites }}</strong></div>
                      </div>
                    </div>
                    <div class="border-l border-subtle pl-5">
                      <p class="analytics-label">{{ appStore.t('vault.analytics.most_used') }}</p>
                      <div class="space-y-3 mt-4">
                        <div v-for="(entry, index) in mostUsedEntries" :key="entry.id" class="flex items-center gap-2 min-w-0">
                          <span class="text-[10px] font-mono text-dim w-4">{{ String(index + 1).padStart(2, '0') }}</span>
                          <span class="text-xs font-bold truncate flex-1">{{ entry.name }}</span>
                          <span class="text-xs text-primary font-black">{{ entry.use_count || 0 }}</span>
                        </div>
                        <p v-if="!mostUsedEntries.length" class="text-sm text-muted">{{ appStore.t('vault.analytics.no_data') }}</p>
                      </div>
                    </div>
                  </div>
                </article>

                <article class="analytics-card lg:col-span-7" data-testid="vault-activity-heatmap">
                  <div class="flex items-center justify-between gap-4 mb-5">
                    <div>
                      <p class="analytics-label">{{ appStore.t('vault.analytics.rhythm') }}</p>
                      <h3 class="analytics-title">{{ appStore.t('vault.analytics.activity_heatmap') }}</h3>
                    </div>
                    <span class="text-xs text-muted">{{ appStore.t('vault.analytics.last_35_days') }}</span>
                  </div>
                  <div class="heatmap-grid">
                    <div
                      v-for="cell in activityHeatmap"
                      :key="cell.key"
                      class="heatmap-cell"
                      :class="`heatmap-level-${cell.level}`"
                      :title="`${cell.label} · ${cell.count} ${appStore.t('vault.analytics.times')}`"
                    ></div>
                  </div>
                  <div class="flex items-center justify-between mt-4 text-[10px] text-dim">
                    <span>{{ activityHeatmap[0]?.label }}</span>
                    <span>{{ appStore.t('vault.analytics.less') }} ··· {{ appStore.t('vault.analytics.more') }}</span>
                    <span>{{ activityHeatmap[activityHeatmap.length - 1]?.label }}</span>
                  </div>
                </article>

                <article class="analytics-card lg:col-span-5" data-testid="vault-hit-recency-breakdown">
                  <p class="analytics-label">{{ appStore.t('vault.analytics.recency') }}</p>
                  <h3 class="analytics-title mb-5">{{ appStore.t('vault.analytics.hit_recency') }}</h3>
                  <div class="space-y-3.5">
                    <div v-for="bucket in hitRecencyBreakdown" :key="bucket.key" class="grid grid-cols-[92px_1fr_28px] gap-3 items-center">
                      <span class="text-xs text-muted truncate">{{ appStore.t(bucket.labelKey) }}</span>
                      <div class="h-2.5 bg-input rounded-full overflow-hidden">
                        <div class="h-full rounded-full age-fill" :style="{ width: `${bucket.percent}%`, backgroundColor: bucket.color }"></div>
                      </div>
                      <strong class="text-xs text-right">{{ bucket.count }}</strong>
                    </div>
                  </div>
                </article>

                <article class="analytics-card lg:col-span-12" data-testid="vault-action-insights">
                  <div class="flex items-center justify-between gap-4 mb-5">
                    <div>
                      <p class="analytics-label">{{ appStore.t('vault.analytics.intelligence') }}</p>
                      <h3 class="analytics-title">{{ appStore.t('vault.analytics.archive_insights') }}</h3>
                    </div>
                    <span class="px-3 py-1.5 rounded-full bg-primary/10 text-primary text-xs font-black">{{ insightCards.length }}</span>
                  </div>
                  <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
                    <div v-for="insight in insightCards" :key="insight.titleKey" class="insight-card" :class="`insight-${insight.tone}`">
                      <div class="flex items-center justify-between gap-3"><i :class="insight.icon"></i><strong>{{ insight.count }}</strong></div>
                      <h4>{{ appStore.t(insight.titleKey) }}</h4>
                      <p>{{ appStore.t(insight.detailKey) }}</p>
                    </div>
                  </div>
                </article>

                <article class="analytics-card lg:col-span-12">
                  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                    <div>
                      <p class="analytics-label">{{ appStore.t('vault.analytics.frequency') }}</p>
                      <h3 class="analytics-title mb-4">{{ appStore.t('vault.analytics.top_unlock_hits') }}</h3>
                      <div class="space-y-2">
                        <div v-for="(entry, index) in mostUsedEntries" :key="entry.id" class="analytics-ranking-row">
                          <span class="ranking-index">{{ String(index + 1).padStart(2, '0') }}</span><span class="truncate flex-1 font-bold">{{ entry.name }}</span><strong>{{ entry.use_count || 0 }}</strong>
                        </div>
                        <p v-if="!mostUsedEntries.length" class="text-sm text-muted">{{ appStore.t('vault.analytics.no_data') }}</p>
                      </div>
                    </div>
                    <div class="lg:border-l border-subtle lg:pl-6">
                      <p class="analytics-label">{{ appStore.t('vault.analytics.recency') }}</p>
                      <h3 class="analytics-title mb-4">{{ appStore.t('vault.analytics.recent_unlock_hits') }}</h3>
                      <div class="space-y-2">
                        <div v-for="entry in recentlyActiveEntries" :key="entry.id" class="analytics-ranking-row"><span class="w-8 h-8 rounded-xl bg-primary/10 text-primary flex items-center justify-center"><i class="pi pi-bolt text-xs"></i></span><span class="truncate flex-1 font-bold">{{ entry.name }}</span><time class="text-xs text-muted">{{ formatDate(entry.last_used) }}</time></div>
                        <p v-if="!recentlyActiveEntries.length" class="text-sm text-muted">{{ appStore.t('vault.analytics.no_data') }}</p>
                      </div>
                    </div>
                  </div>
                </article>
              </div>
              </template>
            </div>
          </section>
        </div>
      </transition>
    </Teleport>

    <transition name="pop">
      <div v-if="showClearConfirm" class="fixed inset-0 z-[150] flex items-center justify-center bg-black/60 backdrop-blur-xl p-4">
        <div class="modal-no-glass rounded-3xl p-10 w-full max-w-xs text-center shadow-2xl text-content">
          <h3 class="text-sm font-black mb-2 uppercase tracking-widest">{{ appStore.t('vault.confirm.clear_title') }}</h3>
          <p class="text-sm text-muted mb-8">{{ appStore.t('vault.confirm.clear_desc') }}</p>
          <div class="flex flex-col gap-2">
            <button @click="confirmClearAll" class="w-full py-3 bg-red-500 text-white rounded-xl text-xs font-black">{{ appStore.t('vault.confirm.clear_btn') }}</button>
            <button @click="showClearConfirm = false" class="w-full py-3 bg-input text-muted rounded-xl text-xs font-bold border border-subtle hover:text-content transition-colors">{{ appStore.t('vault.confirm.cancel') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 删除单条目确认弹窗 -->
    <transition name="pop">
      <div v-if="showDeleteConfirm" class="fixed inset-0 z-[150] flex items-center justify-center bg-black/60 backdrop-blur-xl p-4">
        <div class="modal-no-glass rounded-3xl p-10 w-full max-w-xs text-center shadow-2xl text-content">
          <h3 class="text-sm font-black mb-2 uppercase tracking-widest">{{ appStore.t('vault.confirm.delete_title') }}</h3>
          <p class="text-sm text-muted mb-8">{{ appStore.t('vault.confirm.delete_desc') }}</p>
          <div class="flex flex-col gap-2">
            <button @click="confirmDelete" class="w-full py-3 bg-red-500 text-white rounded-xl text-xs font-black">{{ appStore.t('vault.confirm.delete_btn') }}</button>
            <button @click="showDeleteConfirm = false" class="w-full py-3 bg-input text-muted rounded-xl text-xs font-bold border border-subtle hover:text-content transition-colors">{{ appStore.t('vault.confirm.cancel') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <PasswordEntryModal v-model:visible="showAddModal" :entry="editingEntry" @saved="passwordStore.fetchAllData" />
  </div>
</template>

<style scoped>
.password-vault {
  background: radial-gradient(circle at 100% 0%, color-mix(in srgb, var(--dynamic-accent) 5%, transparent) 0%, transparent 40%);
}

.vault-col-name { width: 19%; }
.vault-col-password { width: 37%; }
.vault-col-notes { width: 18%; }
.vault-col-usage { width: 13%; }
.vault-col-actions { width: 13%; }

.vault-analytics-backdrop {
  background:
    radial-gradient(circle at 50% 15%, color-mix(in srgb, var(--dynamic-accent) 10%, transparent), transparent 38%),
    rgba(5, 8, 18, 0.82);
  backdrop-filter: blur(3px) saturate(0.72);
}

.vault-analytics-panel {
  background:
    linear-gradient(145deg, color-mix(in srgb, var(--bg-modal) 96%, var(--dynamic-accent) 4%), var(--bg-modal));
  border: 1px solid color-mix(in srgb, var(--dynamic-accent) 24%, var(--border-subtle));
  box-shadow:
    0 30px 90px rgba(0, 0, 0, 0.46),
    0 0 0 1px color-mix(in srgb, var(--dynamic-accent) 8%, transparent);
}

.analytics-orb {
  position: absolute;
  border-radius: 999px;
  pointer-events: none;
  filter: blur(1px);
  opacity: 0.28;
  animation: analytics-orb-drift 9s ease-in-out infinite alternate;
}

.analytics-orb-primary {
  width: 320px;
  height: 320px;
  right: -150px;
  top: -190px;
  background: radial-gradient(circle, var(--dynamic-accent), transparent 68%);
}

.analytics-orb-secondary {
  width: 260px;
  height: 260px;
  left: -150px;
  bottom: -170px;
  background: radial-gradient(circle, #8b5cf6, transparent 68%);
  opacity: 0.14;
  animation-delay: -4s;
  animation-duration: 12s;
}

.analytics-kpi,
.analytics-card {
  position: relative;
  background: color-mix(in srgb, var(--bg-input) 76%, transparent);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 78%, transparent);
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 4%, transparent);
  transition:
    transform 0.35s cubic-bezier(0.22, 1, 0.36, 1),
    border-color 0.35s ease,
    box-shadow 0.35s ease;
  animation: analytics-card-rise 0.65s both cubic-bezier(0.22, 1, 0.36, 1);
}

.analytics-kpi::after,
.analytics-card::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  opacity: 0;
  background: linear-gradient(120deg, transparent 25%, color-mix(in srgb, var(--dynamic-accent) 12%, transparent) 48%, transparent 70%);
  background-size: 220% 100%;
  transition: opacity 0.35s ease;
}

.analytics-kpi:hover,
.analytics-card:hover {
  transform: translateY(-2px);
  border-color: color-mix(in srgb, var(--dynamic-accent) 28%, var(--border-subtle));
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 7%, transparent),
    0 14px 38px color-mix(in srgb, var(--dynamic-accent) 8%, transparent);
}

.analytics-kpi:hover::after,
.analytics-card:hover::after {
  opacity: 1;
  animation: analytics-sheen 1.4s ease both;
}

.analytics-kpi:nth-child(2) { animation-delay: 0.06s; }
.analytics-kpi:nth-child(3) { animation-delay: 0.12s; }
.analytics-kpi:nth-child(4) { animation-delay: 0.18s; }

.analytics-card:nth-child(2) { animation-delay: 0.08s; }
.analytics-card:nth-child(3) { animation-delay: 0.14s; }
.analytics-card:nth-child(4) { animation-delay: 0.2s; }
.analytics-card:nth-child(5) { animation-delay: 0.26s; }

.heatmap-grid {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 0.45rem;
}

.heatmap-cell {
  aspect-ratio: 1.8;
  min-height: 0.7rem;
  border-radius: 0.35rem;
  background: color-mix(in srgb, var(--bg-input) 82%, transparent);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 70%, transparent);
  transition: transform 0.2s ease, filter 0.2s ease;
  animation: analytics-card-rise 0.45s both;
}
.heatmap-cell:hover { transform: scale(1.12); filter: brightness(1.15); }
.heatmap-level-1 { background: color-mix(in srgb, var(--dynamic-accent) 25%, var(--bg-input)); }
.heatmap-level-2 { background: color-mix(in srgb, var(--dynamic-accent) 45%, var(--bg-input)); }
.heatmap-level-3 { background: color-mix(in srgb, var(--dynamic-accent) 68%, var(--bg-input)); }
.heatmap-level-4 { background: var(--dynamic-accent); box-shadow: 0 0 12px color-mix(in srgb, var(--dynamic-accent) 35%, transparent); }

.age-fill { animation: analytics-bar-grow 0.75s both cubic-bezier(0.22, 1, 0.36, 1); }
.insight-card { border: 1px solid var(--border-subtle); background: color-mix(in srgb, var(--bg-input) 74%, transparent); border-radius: 1rem; padding: 1rem; min-width: 0; }
.insight-card > div { color: var(--dynamic-accent); }
.insight-card > div strong { font-size: 1.4rem; }
.insight-card h4 { font-size: 0.82rem; font-weight: 900; margin-top: 0.8rem; }
.insight-card p { font-size: 0.68rem; color: var(--text-muted); line-height: 1.55; margin-top: 0.3rem; }
.insight-danger > div { color: #ef4444; }.insight-warning > div { color: #f59e0b; }.insight-info > div { color: #0ea5e9; }.insight-success > div { color: #22c55e; }
.analytics-ranking-row { display: flex; align-items: center; gap: 0.7rem; min-width: 0; padding: 0.65rem 0.75rem; border-radius: 0.8rem; background: color-mix(in srgb, var(--bg-input) 70%, transparent); font-size: 0.75rem; }
.ranking-index { width: 1.5rem; color: var(--text-dim); font-family: ui-monospace, monospace; font-size: 0.65rem; }

@keyframes analytics-bar-grow { from { width: 0; opacity: 0.3; } }

.analytics-kpi {
  border-radius: 1.15rem;
  padding: 1rem;
}

.analytics-kpi > div:first-child {
  color: var(--text-muted);
  font-size: 0.68rem;
  font-weight: 900;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.analytics-kpi strong {
  display: block;
  color: var(--text-base);
  font-size: 1.8rem;
  font-weight: 900;
  line-height: 1;
  margin-top: 0.65rem;
  letter-spacing: -0.04em;
}

.analytics-kpi strong small {
  color: var(--text-muted);
  font-size: 0.72rem;
  margin-left: 0.2rem;
  letter-spacing: 0;
}

.analytics-kpi p {
  color: var(--text-muted);
  font-size: 0.68rem;
  font-weight: 700;
  margin-top: 0.65rem;
}

.analytics-card {
  border-radius: 1.4rem;
  padding: 1.25rem;
}

.analytics-trend-card {
  overflow: hidden;
  background:
    radial-gradient(circle at 72% 18%, color-mix(in srgb, var(--dynamic-accent) 11%, transparent), transparent 38%),
    color-mix(in srgb, var(--bg-input) 76%, transparent);
}

.analytics-label {
  color: var(--text-muted);
  font-size: 0.65rem;
  font-weight: 900;
  letter-spacing: 0.17em;
  text-transform: uppercase;
}

.analytics-title {
  color: var(--text-base);
  font-size: 0.95rem;
  font-weight: 900;
  margin-top: 0.25rem;
}

.usage-donut {
  position: relative;
  isolation: isolate;
  width: 118px;
  height: 118px;
  border-radius: 999px;
  padding: 13px;
  box-shadow:
    0 12px 30px color-mix(in srgb, var(--dynamic-accent) 16%, transparent),
    inset 0 0 0 1px color-mix(in srgb, white 8%, transparent);
  animation: analytics-donut-arrive 0.85s 0.18s both cubic-bezier(0.22, 1, 0.36, 1);
}

.usage-donut::before {
  content: '';
  position: absolute;
  inset: -8px;
  z-index: -1;
  border-radius: inherit;
  border: 1px solid color-mix(in srgb, var(--dynamic-accent) 18%, transparent);
  border-left-color: color-mix(in srgb, var(--dynamic-accent) 62%, transparent);
  animation: analytics-orbit 8s linear infinite;
}

.usage-donut::after {
  content: '';
  position: absolute;
  width: 6px;
  height: 6px;
  top: -10px;
  left: calc(50% - 3px);
  border-radius: 999px;
  background: var(--dynamic-accent);
  box-shadow: 0 0 14px 3px color-mix(in srgb, var(--dynamic-accent) 52%, transparent);
}

.usage-donut > div {
  width: 100%;
  height: 100%;
  border-radius: inherit;
  background: var(--bg-modal);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.usage-donut strong {
  font-size: 1.7rem;
  font-weight: 900;
  line-height: 1;
}

.usage-donut span {
  color: var(--text-muted);
  font-size: 0.62rem;
  font-weight: 800;
  margin-top: 0.35rem;
}

.lifecycle-metric {
  min-width: 0;
  padding: 0.75rem;
  border-radius: 0.9rem;
  background: color-mix(in srgb, var(--bg-card) 65%, transparent);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 72%, transparent);
}

.lifecycle-metric span {
  display: block;
  color: var(--text-muted);
  font-size: 0.62rem;
  font-weight: 800;
  margin-bottom: 0.35rem;
}

.lifecycle-metric strong {
  display: block;
  font-size: 0.78rem;
  font-weight: 900;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lifetime-metric {
  min-width: 0;
  padding: 0.9rem 1rem;
  border-radius: 1rem;
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--dynamic-accent) 7%, transparent), transparent 65%),
    color-mix(in srgb, var(--bg-card) 72%, transparent);
  border: 1px solid color-mix(in srgb, var(--dynamic-accent) 13%, var(--border-subtle));
  transition: transform 0.3s ease, border-color 0.3s ease, background 0.3s ease;
}

.lifetime-metric:hover {
  transform: translateY(-2px);
  border-color: color-mix(in srgb, var(--dynamic-accent) 34%, var(--border-subtle));
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--dynamic-accent) 13%, transparent), transparent 70%),
    color-mix(in srgb, var(--bg-card) 78%, transparent);
}

.lifetime-metric span {
  display: block;
  color: var(--text-muted);
  font-size: 0.64rem;
  font-weight: 800;
  margin-bottom: 0.45rem;
}

.lifetime-metric strong {
  display: block;
  color: var(--text-base);
  font-size: 1rem;
  font-weight: 900;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lifetime-metric small {
  color: var(--text-muted);
  font-size: 0.62rem;
  font-weight: 700;
  margin-left: 0.35rem;
}

.risk-row {
  display: grid;
  grid-template-columns: 8px 1fr auto;
  align-items: center;
  gap: 0.5rem;
  color: var(--text-muted);
  font-size: 0.72rem;
}

.risk-row strong {
  color: var(--text-base);
}

.risk-dot {
  width: 7px;
  height: 7px;
  border-radius: 999px;
}

.analytics-range-switch {
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 4%, transparent);
}

.analytics-range-button {
  position: relative;
  color: var(--text-muted);
  transition: color 0.25s ease, background 0.25s ease, transform 0.25s ease, box-shadow 0.25s ease;
}

.analytics-range-button:hover {
  color: var(--text-base);
  transform: translateY(-1px);
}

.analytics-range-button.is-active {
  color: white;
  background: linear-gradient(135deg, color-mix(in srgb, var(--dynamic-accent) 82%, white), var(--dynamic-accent));
  box-shadow:
    0 6px 16px color-mix(in srgb, var(--dynamic-accent) 30%, transparent),
    inset 0 1px 0 rgba(255, 255, 255, 0.25);
}

.usage-chart::before {
  content: '';
  position: absolute;
  inset: 0 0 1.75rem;
  pointer-events: none;
  background: radial-gradient(ellipse at 70% 20%, color-mix(in srgb, var(--dynamic-accent) 9%, transparent), transparent 54%);
}

.chart-grid span {
  border-top: 1px dashed color-mix(in srgb, var(--border-subtle) 65%, transparent);
}

.usage-chart-area {
  transform-origin: bottom;
  animation: analytics-area-reveal 0.8s 0.12s both cubic-bezier(0.22, 1, 0.36, 1);
}

.usage-chart-glow {
  filter: url(#usageLineGlow);
  opacity: 0;
  animation: analytics-glow-in 0.65s 0.55s forwards ease;
}

.usage-chart-line {
  stroke-dasharray: 1;
  stroke-dashoffset: 1;
  filter: url(#usageLineGlow);
  animation: analytics-line-draw 1.05s 0.15s forwards cubic-bezier(0.65, 0, 0.35, 1);
}

.usage-chart-point {
  opacity: 0;
  transform-box: fill-box;
  transform-origin: center;
  animation: analytics-point-pop 0.45s var(--point-delay) both cubic-bezier(0.34, 1.56, 0.64, 1);
}

.chart-label {
  opacity: 0;
  animation: analytics-label-rise 0.4s var(--label-delay) both ease-out;
}

.category-fill {
  position: relative;
  transform-origin: left;
  animation: analytics-bar-grow 0.75s 0.25s both cubic-bezier(0.22, 1, 0.36, 1);
  box-shadow: 0 0 12px color-mix(in srgb, var(--dynamic-accent) 18%, transparent);
}

.category-fill::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.48), transparent);
  transform: translateX(-120%);
  animation: analytics-bar-shine 2.8s 1s infinite ease-in-out;
}

.entry-profile-hero {
  overflow: hidden;
  background:
    radial-gradient(circle at 92% 18%, color-mix(in srgb, var(--dynamic-accent) 18%, transparent), transparent 32%),
    color-mix(in srgb, var(--bg-input) 82%, transparent);
}

.entry-hit-badge {
  display: inline-flex;
  align-self: flex-start;
  align-items: center;
  gap: 0.55rem;
  padding: 0.7rem 1rem;
  border-radius: 999px;
  color: var(--dynamic-accent);
  background: color-mix(in srgb, var(--dynamic-accent) 11%, var(--bg-input));
  border: 1px solid color-mix(in srgb, var(--dynamic-accent) 30%, var(--border-subtle));
  font-size: 0.75rem;
  font-weight: 900;
  white-space: nowrap;
}

.entry-hit-chart {
  display: grid;
  grid-template-columns: repeat(14, minmax(0, 1fr));
  align-items: end;
  gap: 0.45rem;
  min-height: 190px;
}

.entry-hit-column {
  display: grid;
  grid-template-rows: 1.25rem 130px 1.2rem;
  gap: 0.45rem;
  min-width: 0;
  text-align: center;
}

.entry-hit-column strong { font-size: 0.68rem; }
.entry-hit-column small { color: var(--text-muted); font-size: 0.55rem; font-family: monospace; }
.entry-hit-track {
  display: flex;
  align-items: flex-end;
  justify-content: center;
  border-radius: 0.65rem;
  background: color-mix(in srgb, var(--bg-input) 72%, transparent);
  overflow: hidden;
}
.entry-hit-track span {
  width: 60%;
  min-height: 3px;
  border-radius: 999px 999px 0.25rem 0.25rem;
  background: linear-gradient(to top, var(--dynamic-accent), color-mix(in srgb, var(--dynamic-accent) 45%, white));
  box-shadow: 0 0 16px color-mix(in srgb, var(--dynamic-accent) 34%, transparent);
  animation: analytics-area-reveal 0.75s both cubic-bezier(0.22, 1, 0.36, 1);
  transform-origin: bottom;
}

.profile-context-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.7rem 0.8rem;
  border-radius: 0.8rem;
  background: color-mix(in srgb, var(--bg-input) 76%, transparent);
  font-size: 0.75rem;
}
.profile-context-row span { color: var(--text-muted); }
.profile-context-row strong { text-align: right; overflow-wrap: anywhere; }
.entry-context-tag {
  padding: 0.55rem 0.8rem;
  border-radius: 999px;
  color: var(--dynamic-accent);
  background: color-mix(in srgb, var(--dynamic-accent) 10%, var(--bg-input));
  border: 1px solid color-mix(in srgb, var(--dynamic-accent) 22%, var(--border-subtle));
  font-size: 0.72rem;
  font-weight: 800;
}

@media (max-width: 720px) {
  .entry-hit-chart { gap: 0.22rem; }
  .entry-hit-column { grid-template-rows: 1rem 105px 1rem; }
  .entry-hit-column small { font-size: 0; }
  .entry-hit-column:nth-child(odd) small { font-size: 0.48rem; }
}

@keyframes analytics-card-rise {
  from { opacity: 0; transform: translateY(14px) scale(0.985); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}

@keyframes analytics-orb-drift {
  from { transform: translate3d(-10px, -6px, 0) scale(0.94); opacity: 0.18; }
  to { transform: translate3d(14px, 10px, 0) scale(1.08); opacity: 0.34; }
}

@keyframes analytics-sheen {
  from { background-position: 180% 0; }
  to { background-position: -40% 0; }
}

@keyframes analytics-donut-arrive {
  from { opacity: 0; transform: scale(0.72) rotate(-34deg); }
  to { opacity: 1; transform: scale(1) rotate(0); }
}

@keyframes analytics-orbit {
  to { transform: rotate(360deg); }
}

@keyframes analytics-area-reveal {
  from { opacity: 0; transform: scaleY(0); }
  to { opacity: 1; transform: scaleY(1); }
}

@keyframes analytics-line-draw {
  to { stroke-dashoffset: 0; }
}

@keyframes analytics-glow-in {
  to { opacity: 1; }
}

@keyframes analytics-point-pop {
  from { opacity: 0; transform: scale(0); }
  to { opacity: 1; transform: scale(1); }
}

@keyframes analytics-label-rise {
  from { opacity: 0; transform: translateY(5px); }
  to { opacity: 1; transform: translateY(0); }
}

@keyframes analytics-bar-grow {
  from { transform: scaleX(0); }
  to { transform: scaleX(1); }
}

@keyframes analytics-bar-shine {
  0%, 48% { transform: translateX(-120%); }
  78%, 100% { transform: translateX(120%); }
}

@media (prefers-reduced-motion: reduce) {
  .analytics-orb,
  .analytics-kpi,
  .analytics-card,
  .usage-donut,
  .usage-donut::before,
  .usage-chart-area,
  .usage-chart-glow,
  .usage-chart-line,
  .usage-chart-point,
  .chart-label,
  .category-fill,
  .category-fill::after {
    animation: none !important;
  }

  .usage-chart-glow,
  .usage-chart-point,
  .chart-label {
    opacity: 1;
  }

  .usage-chart-line {
    stroke-dashoffset: 0;
  }
}

.analytics-pop-enter-active,
.analytics-pop-leave-active {
  transition: opacity 0.24s ease;
}

.analytics-pop-enter-active .vault-analytics-panel,
.analytics-pop-leave-active .vault-analytics-panel {
  transition: transform 0.34s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.24s ease;
}

.analytics-pop-enter-from,
.analytics-pop-leave-to {
  opacity: 0;
}

.analytics-pop-enter-from .vault-analytics-panel,
.analytics-pop-leave-to .vault-analytics-panel {
  opacity: 0;
  transform: translateY(18px) scale(0.975);
}

.pop-enter-active, .pop-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.pop-enter-from, .pop-leave-to { opacity: 0; transform: scale(0.95) translateY(10px); }
</style>
