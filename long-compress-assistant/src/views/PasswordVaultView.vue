<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { usePasswordStore } from '@/stores/password'
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

onMounted(async () => {
  await passwordStore.checkUnlockStatus()
})

const stats = computed(() => {
  const total = passwordStore.entries.length
  const totalUsage = passwordStore.entries.reduce((sum, e) => sum + (e.use_count || 0), 0)
  return { total, totalUsage }
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

const showUsageHistory = (entry: any) => {
  selectedEntryForHistory.value = entry
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

const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text)
  appStore.setSuccess(appStore.t('vault.action.copy'))
}

const chartData = computed(() => {
  if (!selectedEntryForHistory.value?.usage_history) return []
  const history = selectedEntryForHistory.value.usage_history
  const days = []
  const maxCount = Math.max(...Object.values(history) as number[], 1)
  for (let i = 6; i >= 0; i--) {
    const d = new Date()
    d.setDate(d.getDate() - i)
    const dateStr = d.toISOString().split('T')[0]
    const count = history[dateStr] || 0
    days.push({ date: dateStr.slice(5), count, height: Math.round((count / maxCount) * 100) })
  }
  return days
})
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
          <thead class="sticky top-0 z-20 bg-input/80 backdrop-blur-xl border-b border-subtle">
            <tr>
              <th class="px-6 py-4 text-xs font-black text-muted uppercase tracking-[0.2em] w-[26%]">{{ appStore.t('vault.column.name') }}</th>
              <th class="px-6 py-4 text-xs font-black text-muted uppercase tracking-[0.2em] w-[26%]">
                <button @click="togglePasswordVisibility" class="flex items-center gap-1.5 hover:text-primary transition-colors">
                  {{ appStore.t('vault.column.password') }}
                  <i :class="showAllPasswords ? 'pi pi-eye-slash' : 'pi pi-eye'" class="text-sm"></i>
                </button>
              </th>
              <th class="px-6 py-4 text-xs font-black text-muted uppercase tracking-[0.2em] w-[26%]">{{ appStore.t('vault.column.notes') }}</th>
              <th class="px-6 py-4 text-xs font-black text-muted uppercase tracking-[0.2em] text-center w-[10%]">{{ appStore.t('vault.column.usage') }}</th>
              <th class="px-6 py-4 text-xs font-black text-muted uppercase tracking-[0.2em] text-right w-[12%]">{{ appStore.t('vault.column.actions') }}</th>
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
            <tbody class="divide-y divide-subtle/50">
              <tr v-for="(entry, index) in filteredAndSortedEntries" :key="entry.id" class="hover:bg-primary/[0.03] group transition-all">
                <td class="px-6 py-3.5 w-[26%]">
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
                <td class="px-6 py-3.5 w-[26%]">
                  <div class="flex items-center gap-2 overflow-hidden group/key w-full">
                    <code class="text-sm font-mono text-primary font-bold bg-primary/5 px-2 py-1 rounded-lg truncate block flex-1 select-none" :class="{ 'tracking-widest': !isPasswordVisible(entry.id) }">{{ maskPassword(entry) }}</code>
                    <button type="button" @click="toggleEntryPasswordVisibility(entry.id)" :aria-label="isPasswordVisible(entry.id) ? appStore.t('vault.action.hide') : appStore.t('vault.action.show')" :title="isPasswordVisible(entry.id) ? appStore.t('vault.action.hide') : appStore.t('vault.action.show')" class="w-7 h-7 rounded-lg flex items-center justify-center text-dim hover:text-primary hover:bg-primary/10 transition-all shrink-0">
                      <i :class="isPasswordVisible(entry.id) ? 'pi pi-eye-slash' : 'pi pi-eye'" class="text-sm"></i>
                    </button>
                    <button type="button" @click="copyToClipboard(entry.password)" :aria-label="appStore.t('vault.action.copy_password')" :title="appStore.t('vault.action.copy_password')" class="w-7 h-7 rounded-lg flex items-center justify-center text-dim hover:text-primary hover:bg-primary/10 transition-all shrink-0"><i class="pi pi-copy text-sm"></i></button>
                  </div>
                </td>
                <td class="px-6 py-3.5 w-[26%]">
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
                <td class="px-6 py-3.5 text-center w-[10%]">
                  <button @click="showUsageHistory(entry)" class="text-xs font-black text-muted hover:text-primary bg-input w-6 h-6 rounded-full flex items-center justify-center mx-auto transition-all shadow-sm border border-subtle shrink-0">
                    {{ entry.use_count || 0 }}
                  </button>
                </td>
                <td class="px-6 py-3.5 text-right w-[12%]">
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

    <!-- 数据统计悬浮区 (右下角) -->
    <div v-if="passwordStore.isInitialized && passwordStore.isUnlocked" class="fixed bottom-8 right-12 z-50 flex gap-6 bg-card/60 backdrop-blur-2xl border border-subtle px-6 py-3 rounded-2xl shadow-2xl">
      <div class="text-center">
        <div class="text-sm text-muted font-black uppercase tracking-widest mb-0.5">{{ appStore.t('vault.vault_size') }}</div>
        <div class="text-lg font-black text-primary leading-none">{{ stats.total }}</div>
      </div>
      <div class="w-px h-6 bg-subtle my-auto"></div>
      <div class="text-center">
        <div class="text-sm text-muted font-black uppercase tracking-widest mb-0.5">{{ appStore.t('vault.total_hits') }}</div>
        <div class="text-lg font-black text-content leading-none">{{ stats.totalUsage }}</div>
      </div>
    </div>

    <transition name="pop">
      <div v-if="showHistoryModal" class="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-md p-4">
        <div class="modal-no-glass rounded-[2.5rem] p-10 w-full max-w-[420px] shadow-2xl scale-in-center text-content">
          <h3 class="text-xs font-black mb-10 flex justify-between items-center text-muted uppercase tracking-[0.3em]">
            {{ appStore.t('vault.lifecycle') }}
            <button type="button" @click="showHistoryModal = false" :aria-label="appStore.t('common.close')" class="w-8 h-8 rounded-lg hover:bg-input hover:text-primary transition-colors"><i class="pi pi-times"></i></button>
          </h3>
          <div class="flex justify-between items-end mb-12">
            <div>
              <div class="text-xs text-muted font-black uppercase mb-2">{{ appStore.t('vault.total_access') }}</div>
              <div class="text-5xl font-black text-primary tracking-tighter">{{ selectedEntryForHistory?.use_count || 0 }}</div>
            </div>
            <div class="px-3 py-1 rounded-full bg-green-500/10 border border-green-500/20 text-sm font-black text-green-500">{{ appStore.t('vault.secure') }}</div>
          </div>
          <div class="h-32 flex items-end justify-between gap-3 mb-8">
            <div v-for="day in chartData" :key="day.date" class="flex-1 flex flex-col items-center gap-3">
              <div class="w-full bg-primary/20 rounded-xl relative transition-all hover:bg-primary/40" :style="{ height: day.height + '%' }"></div>
              <span class="text-xs text-muted font-bold">{{ day.date }}</span>
            </div>
          </div>
          <button @click="showHistoryModal = false" class="w-full py-4 rounded-2xl bg-input border border-subtle text-content text-sm font-black uppercase tracking-widest hover:brightness-110 transition-all">{{ appStore.t('vault.dismiss') }}</button>
        </div>
      </div>
    </transition>

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

.pop-enter-active, .pop-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.pop-enter-from, .pop-leave-to { opacity: 0; transform: scale(0.95) translateY(10px); }
</style>
