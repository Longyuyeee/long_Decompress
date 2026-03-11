<script setup lang="ts">
import { computed } from 'vue'
import { useTaskStore } from '@/stores/task'

const taskStore = useTaskStore()
const activeTask = computed(() => taskStore.tasks.find(t => t.status === 'running' || t.status === 'extracting' || t.status === 'compressing'))

const totalProgress = computed(() => {
  const activeTasks = taskStore.tasks.filter(t => !['completed', 'failed', 'cancelled'].includes(t.status))
  if (activeTasks.length === 0) return 0
  return Math.round(activeTasks.reduce((acc, t) => acc + t.progress, 0) / activeTasks.length)
})

const needsAttention = computed(() => {
  // 模拟逻辑：如果日志最后一条包含“密码”且停止了，说明需要交互
  const lastLog = activeTask.value?.logs[activeTask.value.logs.length - 1]?.message || ''
  return lastLog.includes('密码') && activeTask.value?.progress === 0
})
</script>

<template>
  <Transition name="pop">
    <div v-if="taskStore.activeTaskCount > 0" 
         class="interaction-ball fixed top-10 left-1/2 -translate-x-1/2 z-[100] group">
      <!-- 灵动球体 -->
      <div 
        class="relative w-12 h-12 rounded-full border-2 backdrop-blur-3xl overflow-hidden transition-all duration-500 shadow-2xl flex items-center justify-center cursor-pointer"
        :class="[
          needsAttention ? 'border-red-500/50 bg-red-500/10 animate-pulse' : 'border-blue-500/30 bg-white/5',
          'hover:w-32 hover:rounded-2xl'
        ]"
      >
        <!-- 流体填充背景 -->
        <div class="absolute bottom-0 left-0 w-full bg-blue-500/20 transition-all duration-1000"
             :style="{ height: `${totalProgress}%` }"></div>
        
        <!-- 图标/文字切换 -->
        <div class="relative z-10 flex items-center justify-center gap-2">
          <i v-if="!needsAttention" class="pi pi-sync animate-spin text-blue-400 text-sm"></i>
          <i v-else class="pi pi-key text-red-400 text-sm"></i>
          <span class="text-[10px] text-white font-black opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap">
            {{ needsAttention ? '需要密码' : `${totalProgress}%` }}
          </span>
        </div>
      </div>
      
      <!-- 悬浮详情气泡 -->
      <div class="absolute top-16 left-1/2 -translate-x-1/2 w-48 p-4 rounded-2xl bg-black/80 backdrop-blur-xl border border-white/10 opacity-0 group-hover:opacity-100 transition-all pointer-events-none scale-90 group-hover:scale-100">
         <div class="text-[8px] text-white/30 uppercase font-bold tracking-widest mb-2">正在处理</div>
         <div class="text-[10px] text-white font-medium truncate mb-2">{{ activeTask?.name }}</div>
         <div class="h-1 w-full bg-white/5 rounded-full overflow-hidden">
           <div class="h-full bg-blue-500" :style="{ width: `${totalProgress}%` }"></div>
         </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.interaction-ball {
  mix-blend-mode: plus-lighter;
}

.pop-enter-active, .pop-leave-active {
  transition: all 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.pop-enter-from, .pop-leave-to {
  transform: translate(-50%, -100px) scale(0);
  opacity: 0;
}
</style>
