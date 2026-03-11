<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'

const cpuUsage = ref(0)
const memoryUsage = ref(0)
const isExpanded = ref(false)
const threadLimit = ref(8)

let interval: any = null

const updateStats = async () => {
  try {
    const stats = await invoke<any>('get_resource_usage')
    cpuUsage.value = Math.round(stats.cpu_usage)
    memoryUsage.value = Math.round(stats.memory_usage)
  } catch (e) {
    // 降级模拟数据 (开发环境)
    cpuUsage.value = Math.floor(Math.random() * 30) + 10
    memoryUsage.value = 45
  }
}

onMounted(() => {
  updateStats()
  interval = setInterval(updateStats, 2000)
})

onUnmounted(() => {
  if (interval) clearInterval(interval)
})
</script>

<template>
  <div class="performance-meter fixed bottom-6 left-1/2 -translate-x-1/2 z-50">
    <div 
      @click="isExpanded = !isExpanded"
      class="meter-pill flex items-center gap-6 px-6 py-2.5 rounded-full border border-white/10 backdrop-blur-3xl bg-white/5 hover:bg-white/10 transition-all cursor-pointer shadow-2xl"
      :class="{ 'rounded-3xl p-6 -translate-y-4': isExpanded }"
    >
      <!-- 基础显示 (胶囊态) -->
      <div v-if="!isExpanded" class="flex items-center gap-6">
        <div class="flex items-center gap-3">
          <div class="w-1.5 h-1.5 rounded-full" :class="cpuUsage > 80 ? 'bg-red-500 animate-pulse' : 'bg-blue-400'"></div>
          <span class="text-[10px] text-white/40 font-black uppercase tracking-widest">CPU</span>
          <span class="text-xs text-white/80 font-mono w-8">{{ cpuUsage }}%</span>
        </div>
        
        <div class="w-px h-3 bg-white/10"></div>
        
        <div class="flex items-center gap-3">
          <span class="text-[10px] text-white/40 font-black uppercase tracking-widest">MEM</span>
          <span class="text-xs text-white/80 font-mono w-8">{{ memoryUsage }}%</span>
        </div>
      </div>

      <!-- 展开设置 (面板态) -->
      <div v-else class="w-64 space-y-6">
        <div class="flex justify-between items-center">
          <h4 class="text-[10px] text-white/60 font-black uppercase tracking-widest">算力调度盘</h4>
          <i class="pi pi-sliders-h text-blue-400"></i>
        </div>

        <div class="space-y-4">
           <div class="flex justify-between text-[10px]">
             <span class="text-white/30">并行线程限制</span>
             <span class="text-blue-400 font-mono">{{ threadLimit }} Threads</span>
           </div>
           <input type="range" min="1" max="16" v-model="threadLimit" 
                  class="w-full h-1 bg-white/10 rounded-full appearance-none cursor-pointer accent-blue-400">
           
           <div class="grid grid-cols-2 gap-2">
             <button class="py-2 rounded-lg bg-white/5 border border-white/5 text-[8px] text-white/40 uppercase hover:text-white transition-all">静默模式</button>
             <button class="py-2 rounded-lg bg-blue-500/20 border border-blue-500/20 text-[8px] text-blue-400 uppercase font-bold">全速模式</button>
           </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.meter-pill {
  transition: all 0.6s cubic-bezier(0.34, 1.56, 0.64, 1);
}

input[type='range']::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 12px;
  height: 12px;
  background: white;
  border-radius: 50%;
  box-shadow: 0 0 10px rgba(59, 130, 246, 0.5);
}
</style>
