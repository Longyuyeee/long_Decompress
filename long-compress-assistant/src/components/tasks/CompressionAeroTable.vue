<script setup lang="ts">
import { ref } from 'vue'
import { useCompressionStore } from '@/stores/compression'

const compressionStore = useCompressionStore()
const selectedRows = ref<Set<string>>(new Set())

const toggleSelection = (filePath: string) => {
  if (selectedRows.value.has(filePath)) {
    selectedRows.value.delete(filePath)
  } else {
    selectedRows.value.add(filePath)
  }
}

const handleCreateGroup = () => {
  if (selectedRows.value.size > 0) {
    compressionStore.createGroup(Array.from(selectedRows.value))
    selectedRows.value.clear()
  }
}

const formatSize = (size?: number) => {
  if (!size) return '---'
  return (size / (1024 * 1024)).toFixed(2) + ' MB'
}
</script>

<template>
  <div class="compression-table w-full space-y-6">
    <!-- 顶部操作栏 -->
    <div class="flex justify-between items-center px-4">
      <div class="text-white/40 text-[10px] font-bold uppercase tracking-widest">
        已选 {{ compressionStore.selectedFiles.length + compressionStore.groups.reduce((acc, g) => acc + g.files.length, 0) }} 个项
      </div>
      <button 
        v-if="selectedRows.size > 0"
        @click="handleCreateGroup"
        class="px-6 py-2 rounded-xl bg-blue-500/20 border border-blue-500/30 text-blue-400 text-[10px] font-black uppercase tracking-widest hover:bg-blue-500/30 transition-all scale-105 shadow-[0_0_15px_rgba(59,130,246,0.3)]"
      >
        磁吸成组 (Stacking)
      </button>
    </div>

    <!-- 表格内容 -->
    <div class="glass-table border border-white/10 rounded-2xl overflow-hidden backdrop-blur-xl bg-white/5">
      <!-- 压缩组展示 -->
      <div v-for="group in compressionStore.groups" :key="group.id" 
           class="group-container border-b border-white/10 overflow-hidden"
           :style="{ borderColor: `${group.themeColor}33` }">
        <div class="group-header flex items-center px-6 py-4 bg-white/[0.02]"
             :style="{ borderLeft: `4px solid ${group.themeColor}` }">
          <i class="pi pi-box mr-4" :style="{ color: group.themeColor }"></i>
          <span class="text-white font-bold text-sm flex-1">{{ group.name }}</span>
          
          <div class="flex items-center gap-6 mr-6">
            <div class="text-center">
               <div class="text-[10px] text-white/30 uppercase tracking-tighter">预计大小</div>
               <div class="text-xs text-white/80 font-mono">~120 MB</div>
            </div>
            <div class="w-px h-6 bg-white/10"></div>
            <div class="text-center">
               <div class="text-[10px] text-white/30 uppercase tracking-tighter">压缩率</div>
               <div class="text-xs" :style="{ color: group.themeColor }">72%</div>
            </div>
          </div>
          
          <button @click="compressionStore.dissolveGroup(group.id)" 
                  class="text-white/20 hover:text-red-400 transition-colors">
            <i class="pi pi-times-circle"></i>
          </button>
        </div>
        
        <div v-if="group.expanded" class="group-files px-12 py-2 space-y-1 bg-black/10">
          <div v-for="file in group.files" :key="file.path" class="text-[10px] text-white/40 font-mono py-1 flex justify-between">
            <span>{{ file.name }}</span>
            <span class="text-white/10 italic">{{ file.path }}</span>
          </div>
        </div>
      </div>

      <!-- 未分组文件展示 -->
      <div v-for="file in compressionStore.selectedFiles" :key="file.path" 
           class="file-row flex items-center px-6 py-4 border-b border-white/5 hover:bg-white/5 transition-all cursor-pointer"
           :class="{ 'bg-blue-500/10 border-blue-500/20': selectedRows.has(file.path) }"
           @click="toggleSelection(file.path)">
        <div class="w-8">
           <div class="w-4 h-4 rounded border border-white/20 flex items-center justify-center transition-all"
                :class="{ 'bg-blue-500 border-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.5)]': selectedRows.has(file.path) }">
             <i v-if="selectedRows.has(file.path)" class="pi pi-check text-[8px] text-white"></i>
           </div>
        </div>
        <i class="pi pi-file text-white/20 mr-4"></i>
        <span class="text-white/70 text-sm flex-1">{{ file.name }}</span>
        <span class="text-white/20 text-[10px] font-mono italic mr-6">{{ file.path }}</span>
        <span class="text-white/40 text-xs font-mono">---</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.file-row {
  transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}

.group-container {
  transition: all 0.6s cubic-bezier(0.34, 1.56, 0.64, 1);
}
</style>
