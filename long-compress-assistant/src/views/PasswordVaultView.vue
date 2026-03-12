<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { usePasswordStore, PasswordCategory } from '@/stores/password'
import { useAppStore } from '@/stores/app'
import GlassCard from '@/components/ui/GlassCard.vue'
import GlassButton from '@/components/ui/GlassButton.vue'
import PasswordEntryModal from '@/components/passwords/PasswordEntryModal.vue'
import { invoke } from '@tauri-apps/api/tauri'

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

// 数据总览
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
    appStore.successMessage = "已安全删除"
  } catch (e) {
    appStore.errorMessage = "删除失败"
  }
}

const confirmClearAll = async () => {
  showClearConfirm.value = false
  try {
    await passwordStore.clearAll()
    appStore.successMessage = "密码本已清空"
  } catch (e) {
    appStore.errorMessage = "清空失败"
  }
}

const showUsageHistory = (entry: any) => {
  selectedEntryForHistory.value = entry
  showHistoryModal.value = true
}

// 计算图表数据 (最近7天)
const chartData = computed(() => {
  if (!selectedEntryForHistory.value?.usage_history) return []
  const history = selectedEntryForHistory.value.usage_history
  const days = []
  const maxCount = Math.max(...Object.values(history) as number[], 1)
  
  // 获取最近7天的日期
  for (let i = 6; i >= 0; i--) {
    const d = new Date()
    d.setDate(d.getDate() - i)
    const dateStr = d.toISOString().split('T')[0]
    const count = history[dateStr] || 0
    days.push({
      date: dateStr.slice(5), // 只显示月-日
      count,
      height: Math.round((count / maxCount) * 100)
    })
  }
  return days
})

watch(() => appStore.successMessage, (newVal) => { if (newVal) setTimeout(() => { appStore.successMessage = null }, 3000) })
watch(() => appStore.errorMessage, (newVal) => { if (newVal) setTimeout(() => { appStore.errorMessage = null }, 5000) })

const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text)
  appStore.successMessage = "已复制"
}
</script>

<template>
  <div class="password-book h-screen flex flex-col p-6 relative overflow-hidden bg-[#0a0a0a] text-white">
    <div class="absolute top-0 left-1/2 -translate-x-1/2 w-full h-[500px] bg-blue-500/5 blur-[120px] pointer-events-none"></div>

    <!-- 反馈通知 -->
    <div class="fixed top-12 left-1/2 -translate-x-1/2 z-[200] flex flex-col gap-2 pointer-events-none">
      <Transition name="slide-down">
        <div v-if="appStore.successMessage" class="bg-blue-500 text-white px-6 py-2 rounded-full shadow-2xl flex items-center gap-3 backdrop-blur-xl border border-white/10">
          <i class="pi pi-check-circle text-xs"></i>
          <span class="text-[10px] font-black uppercase">{{ appStore.successMessage }}</span>
        </div>
      </Transition>
    </div>

    <!-- 页头 -->
    <header class="flex-none mb-6 flex justify-between items-center relative z-10 overflow-hidden">
      <div class="flex items-center gap-4 shrink-0">
        <h1 class="text-xl font-black tracking-tighter whitespace-nowrap">密码本</h1>
        <div class="flex gap-1.5">
          <GlassButton @click="handleAddNew" icon="pi pi-plus" size="xs" type="primary" />
          <GlassButton @click="showClearConfirm = true" icon="pi pi-trash" size="xs" type="danger" />
        </div>
      </div>

      <div class="flex items-center gap-4 min-w-0 overflow-hidden">
        <div class="relative group w-32 sm:w-48 transition-all">
          <i class="pi pi-search absolute left-3 top-1/2 -translate-y-1/2 text-white/10 text-[10px]"></i>
          <input v-model="searchQuery" type="text" placeholder="搜索..." class="w-full bg-white/5 border border-white/5 rounded-lg pl-8 pr-3 py-1.5 text-[10px] focus:outline-none focus:border-blue-500/30 transition-all placeholder:text-white/5">
        </div>
        
        <div class="flex gap-3 border-l border-white/5 pl-4 shrink-0">
          <div class="text-right">
            <div class="text-[8px] text-white/10 font-bold uppercase whitespace-nowrap">Total</div>
            <div class="text-xs font-black text-blue-400/80 leading-none">{{ stats.total }}</div>
          </div>
          <div class="text-right">
            <div class="text-[8px] text-white/10 font-bold uppercase whitespace-nowrap">Usage</div>
            <div class="text-xs font-black text-white/40 leading-none">{{ stats.totalUsage }}</div>
          </div>
        </div>
      </div>
    </header>

    <!-- 表格 -->
    <div class="flex-1 min-h-0 relative z-10 bg-white/[0.01] border border-white/5 rounded-2xl overflow-hidden flex flex-col backdrop-blur-sm">
      <div v-if="passwordStore.isLoading" class="absolute inset-0 z-50 bg-black/20 flex items-center justify-center">
        <i class="pi pi-spin pi-spinner text-blue-500/50"></i>
      </div>

      <div class="flex-1 overflow-auto custom-scrollbar">
        <table class="w-full text-left border-collapse table-fixed">
          <thead class="sticky top-0 z-20 bg-[#0d0d0d]">
            <tr class="border-b border-white/5">
              <th class="px-4 py-3 text-[9px] font-black text-white/10 uppercase w-[25%] whitespace-nowrap">名称</th>
              <th class="px-4 py-3 text-[9px] font-black text-white/10 uppercase w-[30%] whitespace-nowrap">密码</th>
              <th class="px-4 py-3 text-[9px] font-black text-white/10 uppercase w-[25%] whitespace-nowrap">备注</th>
              <th class="px-4 py-3 text-[9px] font-black text-white/10 uppercase text-center w-[10%] whitespace-nowrap">调用</th>
              <th class="px-4 py-3 text-[9px] font-black text-white/10 uppercase text-right w-[10%] whitespace-nowrap">操作</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-white/5">
            <tr v-for="entry in filteredAndSortedEntries" :key="entry.id" class="hover:bg-white/[0.01] group transition-colors">
              <td class="px-4 py-3"><span class="text-[11px] font-bold text-white/60 block truncate" :title="entry.name">{{ entry.name }}</span></td>
              <td class="px-4 py-3">
                <div class="flex items-center gap-2 overflow-hidden">
                  <code class="text-[10px] font-mono text-blue-400/40 bg-blue-500/5 px-1.5 py-0.5 rounded truncate max-w-full" :title="entry.password">{{ entry.password }}</code>
                  <i @click="copyToClipboard(entry.password)" class="pi pi-copy text-[9px] text-white/5 hover:text-white/30 cursor-pointer flex-none transition-colors"></i>
                </div>
              </td>
              <td class="px-4 py-3"><span class="text-[10px] text-white/20 block truncate" :title="entry.notes || '-'">{{ entry.notes || '-' }}</span></td>
              <td class="px-4 py-3 text-center">
                <button @click="showUsageHistory(entry)" class="text-[10px] font-black text-white/30 hover:text-blue-400 transition-colors">{{ entry.use_count || 0 }}</button>
              </td>
              <td class="px-4 py-3 text-right">
                <div class="flex justify-end gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                  <i @click="handleEdit(entry)" class="pi pi-pencil text-[10px] text-blue-400/40 hover:text-blue-400 cursor-pointer p-1"></i>
                  <i @click="handleDelete(entry.id)" class="pi pi-trash text-[10px] text-red-400/40 hover:text-red-400 cursor-pointer p-1"></i>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 调用详情 (图表) -->
    <div v-if="showHistoryModal" class="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-md p-4">
      <div class="bg-[#111] border border-white/10 rounded-3xl p-6 w-full max-w-[320px] shadow-2xl scale-in-center">
        <h3 class="text-xs font-black mb-6 flex justify-between items-center text-white/40 uppercase tracking-widest">
          趋势分析
          <i @click="showHistoryModal = false" class="pi pi-times cursor-pointer hover:text-white transition-colors"></i>
        </h3>
        
        <!-- 总览 -->
        <div class="flex justify-between items-end mb-8">
          <div>
            <div class="text-[8px] text-white/10 font-bold uppercase mb-1">Total Usage</div>
            <div class="text-3xl font-black text-blue-400 tracking-tighter">{{ selectedEntryForHistory?.use_count || 0 }}</div>
          </div>
          <div class="text-right">
            <div class="text-[8px] text-white/10 font-bold uppercase mb-1">Status</div>
            <div class="text-[10px] font-bold text-green-500/80 bg-green-500/10 px-2 py-0.5 rounded-full">ACTIVE</div>
          </div>
        </div>

        <!-- CSS 简易柱状图 -->
        <div class="h-24 flex items-end justify-between gap-2 mb-4 px-2">
          <div v-for="day in chartData" :key="day.date" class="flex-1 flex flex-col items-center gap-2 group">
            <div class="w-full bg-blue-500/10 rounded-t-sm relative transition-all group-hover:bg-blue-500/20" :style="{ height: day.height + '%' }">
              <div v-if="day.count > 0" class="absolute -top-4 left-1/2 -translate-x-1/2 text-[8px] font-black text-blue-400 opacity-0 group-hover:opacity-100 transition-opacity">{{ day.count }}</div>
            </div>
            <span class="text-[7px] text-white/10 font-bold whitespace-nowrap">{{ day.date }}</span>
          </div>
        </div>

        <GlassButton @click="showHistoryModal = false" label="返回列表" size="sm" class="w-full mt-4" />
      </div>
    </div>

    <!-- 清空确认 -->
    <div v-if="showClearConfirm" class="fixed inset-0 z-[150] flex items-center justify-center bg-black/80 backdrop-blur-xl p-4">
      <div class="bg-[#111] border border-red-500/10 rounded-3xl p-8 w-full max-w-xs shadow-2xl text-center scale-in-center">
        <h3 class="text-sm font-black text-white mb-2 uppercase tracking-widest">清空数据?</h3>
        <p class="text-[10px] text-white/20 mb-8">此操作将移除所有已保存的凭证。</p>
        <div class="flex flex-col gap-2">
          <button @click="confirmClearAll" class="w-full py-2.5 bg-red-500/80 hover:bg-red-500 text-white rounded-xl text-[10px] font-black transition-all">彻底清空</button>
          <button @click="showClearConfirm = false" class="w-full py-2.5 bg-white/5 text-white/20 rounded-xl text-[10px] font-bold transition-all">取消</button>
        </div>
      </div>
    </div>

    <PasswordEntryModal v-model:visible="showAddModal" :entry="editingEntry" @saved="passwordStore.fetchAllData" />
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 2px; height: 2px; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.03); border-radius: 10px; }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.slide-down-enter-active, .slide-down-leave-active { transition: all 0.4s cubic-bezier(0.16, 1, 0.3, 1); }
.slide-down-enter-from, .slide-down-leave-to { transform: translate(-50%, -20px); opacity: 0; }
.scale-in-center { animation: scale-in-center 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94) both; }
@keyframes scale-in-center { 0% { transform: scale(0.95); opacity: 0; } 100% { transform: scale(1); opacity: 1; } }
</style>
