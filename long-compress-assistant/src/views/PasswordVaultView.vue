<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { usePasswordStore, PasswordCategory } from '@/stores/password'
import { useAppStore } from '@/stores/app'
import GlassCard from '@/components/ui/GlassCard.vue'
import GlassButton from '@/components/ui/GlassButton.vue'
import { gsap } from 'gsap'
import PasswordEntryModal from '@/components/passwords/PasswordEntryModal.vue'

const passwordStore = usePasswordStore()
const appStore = useAppStore()

const showAddModal = ref(false)
const searchQuery = ref('')

onMounted(async () => {
  await passwordStore.checkUnlockStatus()
})

const filteredAndSortedEntries = computed(() => {
  let result = [...passwordStore.entries]
  
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(e => 
      e.name.toLowerCase().includes(q) || 
      (e.notes && e.notes.toLowerCase().includes(q)) ||
      e.password.toLowerCase().includes(q)
    )
  }

  // 过滤掉标记为“钥匙丢失”的占位条目（可选，或者保留让用户手动删）
  return result.sort((a, b) => {
    if ((b.use_count || 0) !== (a.use_count || 0)) {
      return (b.use_count || 0) - (a.use_count || 0)
    }
    const timeA = a.last_used ? new Date(a.last_used).getTime() : 0
    const timeB = b.last_used ? new Date(b.last_used).getTime() : 0
    return timeB - timeA
  })
})

const editingEntry = ref<any>(null)

const handleEdit = (entry: any) => {
  editingEntry.value = entry
  showAddModal.value = true
}

const handleAddNew = () => {
  editingEntry.value = null
  showAddModal.value = true
}

const handleDelete = async (id: string) => {
  if (confirm('确定要删除这个密码吗？')) {
    try {
      await passwordStore.deleteEntry(id)
      appStore.successMessage = "密码已安全移除"
    } catch (e) {
      appStore.errorMessage = "删除失败: " + e
    }
  }
}

import { save, open } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'

const handleExport = async () => {
  try {
    const filePath = await save({
      filters: [{ name: 'JSON', extensions: ['json'] }],
      defaultPath: 'password_book_backup.json'
    })
    
    if (filePath) {
      await invoke('export_passwords_command', { filePath })
      appStore.successMessage = "密码本已成功导出至: " + filePath
    }
  } catch (e) {
    appStore.errorMessage = "导出失败: " + e
  }
}

const handleImport = async () => {
  try {
    const filePath = await open({
      filters: [{ name: 'JSON', extensions: ['json'] }],
      multiple: false
    })
    
    if (filePath && typeof filePath === 'string') {
      const count = await invoke<number>('import_passwords_command', { filePath })
      appStore.successMessage = `成功导入 ${count} 条新密码`
      await passwordStore.fetchAllData()
    }
  } catch (e) {
    appStore.errorMessage = "导入失败: " + e
  }
}

const formatTime = (date?: string | null) => {
  if (!date) return '从不'
  return new Date(date).toLocaleString('zh-CN', { 
    month: 'numeric', 
    day: 'numeric', 
    hour: '2-digit', 
    minute: '2-digit' 
  })
}

const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text)
  appStore.successMessage = "密码已复制到剪贴板"
}
</script>

<template>
  <div class="password-book p-8 min-h-screen relative overflow-hidden bg-[#0a0a0a]">
    <!-- 背景装饰 -->
    <div class="absolute top-0 left-1/2 -translate-x-1/2 w-full h-[500px] bg-blue-500/5 blur-[120px] pointer-events-none"></div>

    <!-- 页头 -->
    <header class="mb-8 flex justify-between items-end relative z-10">
      <div class="flex items-center gap-6">
        <div class="w-14 h-14 rounded-2xl bg-white/5 border border-white/10 flex items-center justify-center shadow-2xl backdrop-blur-xl">
          <i class="pi pi-book text-blue-400 text-2xl"></i>
        </div>
        <div>
          <h1 class="text-3xl font-black text-white tracking-tighter mb-0.5">密码本</h1>
          <div class="flex items-center gap-3">
            <span class="text-[9px] text-white/20 uppercase font-bold tracking-[0.2em]">Password Book v2.2</span>
            <div class="w-1 h-1 rounded-full bg-white/10"></div>
            <span class="text-[9px] text-blue-400/60 font-bold uppercase tracking-widest">Secure Storage</span>
          </div>
        </div>
      </div>

      <div class="flex gap-3">
        <GlassButton @click="handleAddNew" icon="pi pi-plus" label="新增密码" type="primary" />
        <GlassButton @click="handleImport" icon="pi pi-upload" label="导入" />
        <GlassButton @click="handleExport" icon="pi pi-download" label="导出" />
      </div>
    </header>

    <!-- 工具栏：查询框 -->
    <div class="mb-6 relative z-10">
      <div class="relative max-w-md group">
        <i class="pi pi-search absolute left-4 top-1/2 -translate-y-1/2 text-white/20 group-focus-within:text-blue-400 transition-colors"></i>
        <input 
          v-model="searchQuery"
          type="text" 
          placeholder="搜索名称、密码或备注..."
          class="w-full bg-white/5 border border-white/10 rounded-2xl pl-11 pr-4 py-3 text-sm text-white focus:outline-none focus:border-blue-500/50 focus:bg-white/10 transition-all placeholder:text-white/10"
        >
      </div>
    </div>

    <!-- 密码表格 (Excel 风格) -->
    <div class="relative z-10 bg-white/[0.02] border border-white/10 rounded-3xl overflow-hidden backdrop-blur-xl shadow-2xl">
      <div class="overflow-x-auto custom-scrollbar">
        <table class="w-full text-left border-collapse">
          <thead>
            <tr class="border-b border-white/5 bg-white/[0.02]">
              <th class="px-6 py-4 text-[10px] font-black text-white/30 uppercase tracking-[0.2em] w-1/4">名称</th>
              <th class="px-6 py-4 text-[10px] font-black text-white/30 uppercase tracking-[0.2em] w-1/4">密码</th>
              <th class="px-6 py-4 text-[10px] font-black text-white/30 uppercase tracking-[0.2em] w-1/4">备注</th>
              <th class="px-6 py-4 text-[10px] font-black text-white/30 uppercase tracking-[0.2em] text-center">调用次数</th>
              <th class="px-6 py-4 text-[10px] font-black text-white/30 uppercase tracking-[0.2em] text-right">操作</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-white/5">
            <tr v-for="entry in filteredAndSortedEntries" :key="entry.id" class="hover:bg-white/[0.03] transition-colors group">
              <td class="px-6 py-5">
                <div class="flex items-center gap-3">
                  <div class="w-2 h-2 rounded-full bg-blue-500/40"></div>
                  <span class="text-sm font-bold text-white/80">{{ entry.name }}</span>
                </div>
              </td>
              <td class="px-6 py-5">
                <div class="flex items-center gap-2">
                  <code class="text-xs font-mono text-blue-400/80 bg-blue-500/5 px-2 py-1 rounded">
                    {{ entry.password }}
                  </code>
                  <button @click="copyToClipboard(entry.password)" class="opacity-0 group-hover:opacity-100 p-1.5 rounded-lg hover:bg-white/10 text-white/20 hover:text-white/60 transition-all">
                    <i class="pi pi-copy text-xs"></i>
                  </button>
                </div>
              </td>
              <td class="px-6 py-5">
                <span class="text-xs text-white/40 truncate block max-w-[200px]">{{ entry.notes || '-' }}</span>
              </td>
              <td class="px-6 py-5 text-center">
                <span class="inline-block px-3 py-1 rounded-full bg-white/5 text-[10px] font-black text-white/60 border border-white/5">
                  {{ entry.use_count || 0 }}
                </span>
              </td>
              <td class="px-6 py-5 text-right">
                <div class="flex justify-end gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                  <button @click="handleEdit(entry)" class="p-2 rounded-xl bg-blue-500/10 text-blue-400 hover:bg-blue-500 hover:text-white transition-all">
                    <i class="pi pi-pencil text-xs"></i>
                  </button>
                  <button @click="handleDelete(entry.id)" class="p-2 rounded-xl bg-red-500/10 text-red-400 hover:bg-red-500 hover:text-white transition-all">
                    <i class="pi pi-trash text-xs"></i>
                  </button>
                </div>
              </td>
            </tr>

            <!-- 空状态 -->
            <tr v-if="filteredAndSortedEntries.length === 0">
              <td colspan="5" class="px-6 py-20 text-center">
                <div class="flex flex-col items-center gap-4">
                  <i class="pi pi-inbox text-4xl text-white/5"></i>
                  <p class="text-xs font-bold text-white/20 uppercase tracking-widest">暂无相关密码条目</p>
                  <GlassButton @click="handleAddNew" label="立即添加" size="sm" />
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 新增/编辑凭证弹窗 -->
    <PasswordEntryModal 
      v-model:visible="showAddModal" 
      :entry="editingEntry"
      @saved="passwordStore.fetchAllData" 
    />
  </div>
</template>

<style scoped>
.password-book {
  background: radial-gradient(circle at 50% 0%, rgba(59, 130, 246, 0.03) 0%, transparent 50%);
}

.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
  height: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.1);
}

/* 表格渐入动画 */
tbody tr {
  animation: slideUp 0.4s ease forwards;
  opacity: 0;
}

@keyframes slideUp {
  from { transform: translateY(10px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}

/* 错位进场 */
tbody tr:nth-child(1) { animation-delay: 0.05s; }
tbody tr:nth-child(2) { animation-delay: 0.1s; }
tbody tr:nth-child(3) { animation-delay: 0.15s; }
tbody tr:nth-child(4) { animation-delay: 0.2s; }
</style>
