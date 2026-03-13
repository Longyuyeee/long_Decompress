<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { usePasswordStore } from '@/stores/password'
import { useAppStore } from '@/stores/app'
import PasswordEntryModal from '@/components/passwords/PasswordEntryModal.vue'

const passwordStore = usePasswordStore()
const appStore = useAppStore()

const showAddModal = ref(false)
const showHistoryModal = ref(false)
const showClearConfirm = ref(false)
const selectedEntryForHistory = ref<any>(null)
const searchQuery = ref('')

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

const handleDelete = async (id: string) => {
  try {
    await passwordStore.deleteEntry(id)
    appStore.successMessage = appStore.t('common.success')
  } catch (e) {
    appStore.errorMessage = appStore.t('common.error')
  }
}

const handleExport = async () => {
  console.log('Exporting passwords...')
}

const handleImport = async () => {
  console.log('Importing passwords...')
}

const confirmClearAll = async () => {
  showClearConfirm.value = false
  try {
    await passwordStore.clearAll()
    appStore.successMessage = appStore.t('common.success')
  } catch (e) {
    appStore.errorMessage = appStore.t('common.error')
  }
}

const showUsageHistory = (entry: any) => {
  selectedEntryForHistory.value = entry
  showHistoryModal.value = true
}

const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text)
  appStore.successMessage = appStore.language === 'zh-CN' ? '已安全复制' : 'Copied'
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
          <p class="text-muted text-[9px] font-bold uppercase tracking-[0.2em] ml-0.5">{{ appStore.t('vault.usage_stats') }}</p>
        </div>
        
        <div class="flex gap-2">
          <button @click="handleAddNew" class="w-9 h-9 rounded-xl bg-primary text-white flex items-center justify-center hover:scale-105 transition-all shadow-lg hover:shadow-primary/40">
            <i class="pi pi-plus text-sm"></i>
          </button>
          <div class="w-px h-5 bg-subtle my-auto mx-1"></div>
          <button @click="handleExport" class="w-8 h-8 rounded-lg bg-input border border-subtle text-muted flex items-center justify-center hover:text-primary hover:bg-primary/5 transition-all" title="Export">
            <i class="pi pi-download text-[10px]"></i>
          </button>
          <button @click="handleImport" class="w-8 h-8 rounded-lg bg-input border border-subtle text-muted flex items-center justify-center hover:text-primary hover:bg-primary/5 transition-all" title="Import">
            <i class="pi pi-upload text-[10px]"></i>
          </button>
          <button @click="showClearConfirm = true" class="w-8 h-8 rounded-lg bg-input border border-subtle text-muted flex items-center justify-center hover:text-red-500 transition-all" title="Clear All">
            <i class="pi pi-trash text-[10px]"></i>
          </button>
        </div>
      </div>

      <div class="flex-1 flex justify-end">
        <div class="relative w-full max-w-[280px] group">
          <i class="pi pi-search absolute left-4 top-1/2 -translate-y-1/2 text-dim text-[10px] group-hover:text-primary transition-colors"></i>
          <input v-model="searchQuery" type="text" :placeholder="appStore.t('common.search')" class="w-full bg-input border border-subtle rounded-xl pl-10 pr-4 py-2.5 text-[11px] text-content focus:outline-none focus:border-primary transition-all shadow-sm placeholder:text-dim">
        </div>
      </div>
    </header>

    <div class="flex-1 min-h-0 aero-card overflow-hidden flex flex-col mb-12">
      <div v-if="passwordStore.isLoading" class="absolute inset-0 z-50 bg-card/80 backdrop-blur-sm flex items-center justify-center">
        <i class="pi pi-spin pi-spinner text-primary text-2xl"></i>
      </div>

      <div class="flex-1 overflow-hidden flex flex-col relative">
        <table class="w-full text-left border-collapse table-fixed">
          <thead class="sticky top-0 z-20 bg-input/80 backdrop-blur-xl border-b border-subtle">
            <tr>
              <th class="px-6 py-4 text-[9px] font-black text-muted uppercase tracking-[0.2em] w-[18%]">{{ appStore.t('vault.column.name') }}</th>
              <th class="px-6 py-4 text-[9px] font-black text-muted uppercase tracking-[0.2em] w-[27%]">{{ appStore.t('vault.column.password') }}</th>
              <th class="px-6 py-4 text-[9px] font-black text-muted uppercase tracking-[0.2em] w-[31%]">{{ appStore.t('vault.column.notes') }}</th>
              <th class="px-6 py-4 text-[9px] font-black text-muted uppercase tracking-[0.2em] text-center w-[12%]">{{ appStore.t('vault.column.usage') }}</th>
              <th class="px-6 py-4 text-[9px] font-black text-muted uppercase tracking-[0.2em] text-right w-[12%]">{{ appStore.t('vault.column.actions') }}</th>
            </tr>
          </thead>
        </table>
        
        <div class="flex-1 overflow-y-auto custom-scrollbar">
          <table class="w-full text-left border-collapse table-fixed">
            <tbody class="divide-y divide-subtle/50">
              <tr v-for="entry in filteredAndSortedEntries" :key="entry.id" class="hover:bg-primary/[0.03] group transition-all">
                <td class="px-6 py-3.5 w-[18%]">
                  <div class="flex items-center gap-2 relative group/tooltip">
                    <div class="w-1.5 h-1.5 rounded-full bg-primary/40 group-hover:bg-primary transition-colors shrink-0"></div>
                    <span class="text-[11px] font-bold text-content truncate block w-full">{{ entry.name }}</span>
                    <!-- 自定义悬浮窗 (Aero Tooltip) - 提高 z-index 并增加下偏移防止遮挡 -->
                    <div class="absolute left-0 bottom-[110%] mb-1 px-3 py-2 rounded-xl bg-card/90 backdrop-blur-3xl border border-subtle shadow-2xl text-[10px] text-content whitespace-normal break-all max-w-[200px] z-[100] opacity-0 translate-y-2 group-hover/tooltip:opacity-100 group-hover/tooltip:translate-y-0 transition-all pointer-events-none font-bold">
                      {{ entry.name }}
                    </div>
                  </div>
                </td>
                <td class="px-6 py-3.5 w-[27%]">
                  <div class="flex items-center gap-2 overflow-hidden group/key w-full">
                    <code class="text-[10px] font-mono text-primary font-bold bg-primary/5 px-2 py-1 rounded-lg truncate block flex-1">{{ entry.password }}</code>
                    <i @click="copyToClipboard(entry.password)" class="pi pi-copy text-[10px] text-dim hover:text-primary cursor-pointer transition-all opacity-0 group-hover/key:opacity-100 scale-90 hover:scale-110 shrink-0"></i>
                  </div>
                </td>
                <td class="px-6 py-3.5 w-[31%]">
                  <div class="relative group/tooltip w-full">
                    <span class="text-[10px] text-muted italic truncate block w-full">{{ entry.notes || '—' }}</span>
                    <!-- 自定义悬浮窗 (Aero Tooltip) - 提高 z-index -->
                    <div v-if="entry.notes" class="absolute left-0 bottom-[110%] mb-1 px-3 py-2 rounded-xl bg-card/90 backdrop-blur-3xl border border-subtle shadow-2xl text-[10px] text-muted whitespace-normal break-all max-w-[240px] z-[100] opacity-0 translate-y-2 group-hover/tooltip:opacity-100 group-hover/tooltip:translate-y-0 transition-all pointer-events-none italic">
                      {{ entry.notes }}
                    </div>
                  </div>
                </td>
                <td class="px-6 py-3.5 text-center w-[12%]">
                  <button @click="showUsageHistory(entry)" class="text-[9px] font-black text-muted hover:text-primary bg-input w-6 h-6 rounded-full flex items-center justify-center mx-auto transition-all shadow-sm border border-subtle shrink-0">
                    {{ entry.use_count || 0 }}
                  </button>
                </td>
                <td class="px-6 py-3.5 text-right w-[12%]">
                  <div class="flex justify-end gap-3 opacity-0 group-hover:opacity-100 transition-all shrink-0">
                    <button @click="handleEdit(entry)" class="text-primary/60 hover:text-primary transition-colors"><i class="pi pi-pencil text-[10px]"></i></button>
                    <button @click="handleDelete(entry.id)" class="text-red-400/60 hover:text-red-500 transition-colors"><i class="pi pi-trash text-[10px]"></i></button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- 数据统计悬浮区 (右下角) -->
    <div class="fixed bottom-8 right-12 z-50 flex gap-6 bg-card/60 backdrop-blur-2xl border border-subtle px-6 py-3 rounded-2xl shadow-2xl">
      <div class="text-center">
        <div class="text-[7px] text-muted font-black uppercase tracking-widest mb-0.5">Vault Size</div>
        <div class="text-lg font-black text-primary leading-none">{{ stats.total }}</div>
      </div>
      <div class="w-px h-6 bg-subtle my-auto"></div>
      <div class="text-center">
        <div class="text-[7px] text-muted font-black uppercase tracking-widest mb-0.5">Total Hits</div>
        <div class="text-lg font-black text-content leading-none">{{ stats.totalUsage }}</div>
      </div>
    </div>

    <transition name="pop">
      <div v-if="showHistoryModal" class="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-md p-4">
        <div class="modal-no-glass rounded-[2.5rem] p-10 w-full max-w-[420px] shadow-2xl scale-in-center text-content">
          <h3 class="text-xs font-black mb-10 flex justify-between items-center text-muted uppercase tracking-[0.3em]">
            Lifecycle Analysis
            <i @click="showHistoryModal = false" class="pi pi-times cursor-pointer hover:text-primary"></i>
          </h3>
          <div class="flex justify-between items-end mb-12">
            <div>
              <div class="text-[9px] text-muted font-black uppercase mb-2">Total Access</div>
              <div class="text-5xl font-black text-primary tracking-tighter">{{ selectedEntryForHistory?.use_count || 0 }}</div>
            </div>
            <div class="px-3 py-1 rounded-full bg-green-500/10 border border-green-500/20 text-[10px] font-black text-green-500">SECURE</div>
          </div>
          <div class="h-32 flex items-end justify-between gap-3 mb-8">
            <div v-for="day in chartData" :key="day.date" class="flex-1 flex flex-col items-center gap-3">
              <div class="w-full bg-primary/20 rounded-xl relative transition-all hover:bg-primary/40" :style="{ height: day.height + '%' }"></div>
              <span class="text-[8px] text-muted font-bold">{{ day.date }}</span>
            </div>
          </div>
          <button @click="showHistoryModal = false" class="w-full py-4 rounded-2xl bg-input border border-subtle text-content text-[10px] font-black uppercase tracking-widest hover:brightness-110 transition-all">Dismiss</button>
        </div>
      </div>
    </transition>

    <transition name="pop">
      <div v-if="showClearConfirm" class="fixed inset-0 z-[150] flex items-center justify-center bg-black/60 backdrop-blur-xl p-4">
        <div class="modal-no-glass rounded-3xl p-10 w-full max-w-xs text-center shadow-2xl text-content">
          <h3 class="text-sm font-black mb-2 uppercase tracking-widest">确认清空?</h3>
          <p class="text-[10px] text-muted mb-8">此操作不可撤销。</p>
          <div class="flex flex-col gap-2">
            <button @click="confirmClearAll" class="w-full py-3 bg-red-500 text-white rounded-xl text-xs font-black">彻底清空</button>
            <button @click="showClearConfirm = false" class="w-full py-3 bg-input text-muted rounded-xl text-xs font-bold border border-subtle hover:text-content transition-colors">取消</button>
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
