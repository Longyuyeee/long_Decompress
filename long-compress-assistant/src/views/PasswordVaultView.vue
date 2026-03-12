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
  <div class="password-vault p-responsive p-8 min-h-screen flex flex-col gap-8 transition-colors duration-700">
    <header class="flex flex-col md:flex-row md:justify-between md:items-end gap-6">
      <div class="flex items-center gap-6 shrink-0">
        <div>
          <h1 class="text-4xl font-black text-content tracking-tighter mb-1">{{ appStore.t('nav.vault') }}</h1>
          <p class="text-muted text-[10px] font-bold uppercase tracking-[0.2em] ml-1">{{ appStore.t('vault.usage_stats') }}</p>
        </div>
        
        <div class="flex gap-2">
          <button @click="handleAddNew" class="w-10 h-10 rounded-2xl bg-primary text-white flex items-center justify-center hover:scale-105 transition-all shadow-lg hover:shadow-glass-hover">
            <i class="pi pi-plus"></i>
          </button>
          <div class="w-px h-6 bg-subtle my-auto mx-1"></div>
          <button @click="handleExport" class="w-9 h-9 rounded-xl bg-input border border-subtle text-muted flex items-center justify-center hover:text-primary hover:bg-primary/5 transition-all" title="Export">
            <i class="pi pi-download text-xs"></i>
          </button>
          <button @click="handleImport" class="w-9 h-9 rounded-xl bg-input border border-subtle text-muted flex items-center justify-center hover:text-primary hover:bg-primary/5 transition-all" title="Import">
            <i class="pi pi-upload text-xs"></i>
          </button>
          <button @click="showClearConfirm = true" class="w-9 h-9 rounded-xl bg-input border border-subtle text-muted flex items-center justify-center hover:text-red-500 transition-all" title="Clear All">
            <i class="pi pi-trash text-xs"></i>
          </button>
        </div>
      </div>

      <div class="flex-1 flex items-center justify-end gap-6 min-w-0">
        <div class="relative w-full max-w-sm group">
          <i class="pi pi-search absolute left-4 top-1/2 -translate-y-1/2 text-dim text-xs group-hover:text-primary transition-colors"></i>
          <input v-model="searchQuery" type="text" :placeholder="appStore.t('common.search')" class="w-full bg-input border border-subtle rounded-[1.2rem] pl-12 pr-4 py-3 text-xs text-content focus:outline-none focus:border-primary transition-all shadow-sm placeholder:text-dim">
        </div>
        
        <div class="flex gap-8 border-l border-subtle pl-8 hide-on-small">
          <div class="text-center">
            <div class="text-[8px] text-muted font-black uppercase tracking-widest mb-1">Vault Size</div>
            <div class="text-xl font-black text-primary leading-none">{{ stats.total }}</div>
          </div>
          <div class="text-center">
            <div class="text-[8px] text-muted font-black uppercase tracking-widest mb-1">Total Hits</div>
            <div class="text-xl font-black text-content leading-none">{{ stats.totalUsage }}</div>
          </div>
        </div>
      </div>
    </header>

    <div class="flex-1 min-h-0 aero-card overflow-hidden flex flex-col">
      <div v-if="passwordStore.isLoading" class="absolute inset-0 z-50 bg-card/80 backdrop-blur-sm flex items-center justify-center">
        <i class="pi pi-spin pi-spinner text-primary text-2xl"></i>
      </div>

      <div class="flex-1 overflow-auto custom-scrollbar">
        <table class="w-full text-left border-collapse min-w-[800px]">
          <thead class="sticky top-0 z-20 bg-input/80 backdrop-blur-xl border-b border-subtle">
            <tr>
              <th class="px-8 py-5 text-[10px] font-black text-muted uppercase tracking-[0.2em] w-[25%]">Identifier</th>
              <th class="px-8 py-5 text-[10px] font-black text-muted uppercase tracking-[0.2em] w-[30%]">Access Key</th>
              <th class="px-8 py-5 text-[10px] font-black text-muted uppercase tracking-[0.2em] w-[25%] hide-on-small">Meta Notes</th>
              <th class="px-8 py-5 text-[10px] font-black text-muted uppercase tracking-[0.2em] text-center w-[10%]">Usage</th>
              <th class="px-8 py-5 text-[10px] font-black text-muted uppercase tracking-[0.2em] text-right w-[10%]">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-subtle/50">
            <tr v-for="entry in filteredAndSortedEntries" :key="entry.id" class="hover:bg-primary/[0.03] group transition-all">
              <td class="px-8 py-5">
                <div class="flex items-center gap-3">
                  <div class="w-2 h-2 rounded-full bg-primary/40 group-hover:bg-primary transition-colors"></div>
                  <span class="text-xs font-bold text-content truncate max-w-[200px]" :title="entry.name">{{ entry.name }}</span>
                </div>
              </td>
              <td class="px-8 py-5">
                <div class="flex items-center gap-3 overflow-hidden group/key">
                  <code class="text-[11px] font-mono text-primary font-bold bg-primary/5 px-3 py-1.5 rounded-lg truncate max-w-[220px]">{{ entry.password }}</code>
                  <i @click="copyToClipboard(entry.password)" class="pi pi-copy text-[10px] text-dim hover:text-primary cursor-pointer transition-all opacity-0 group-hover/key:opacity-100 scale-90 hover:scale-110"></i>
                </div>
              </td>
              <td class="px-8 py-5 hide-on-small">
                <span class="text-[10px] text-muted italic truncate max-w-[200px]" :title="entry.notes">{{ entry.notes || '—' }}</span>
              </td>
              <td class="px-8 py-5 text-center">
                <button @click="showUsageHistory(entry)" class="text-[10px] font-black text-muted hover:text-primary bg-input w-8 h-8 rounded-full flex items-center justify-center mx-auto transition-all shadow-sm hover:shadow-md border border-subtle">
                  {{ entry.use_count || 0 }}
                </button>
              </td>
              <td class="px-8 py-5 text-right">
                <div class="flex justify-end gap-4 opacity-0 group-hover:opacity-100 transition-all">
                  <button @click="handleEdit(entry)" class="text-primary/60 hover:text-primary transition-colors"><i class="pi pi-pencil text-xs"></i></button>
                  <button @click="handleDelete(entry.id)" class="text-red-400/60 hover:text-red-500 transition-colors"><i class="pi pi-trash text-xs"></i></button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
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
