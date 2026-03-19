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
         class="interaction-ball fixed bottom-8 left-2 z-[100] group">
      <!-- 灵动球体 (侧边栏扩展：智慧核心) -->
      <div 
        class="relative h-12 rounded-full border border-subtle backdrop-blur-3xl overflow-hidden transition-all duration-700 shadow-2xl flex items-center justify-start cursor-pointer px-4 gap-3 bg-card"
        :class="[
          needsAttention ? 'border-red-500/50 bg-red-500/10 shadow-red-500/20 animate-pulse' : 'border-primary/20 hover:border-primary/50',
          'w-12 hover:w-56 group-hover:shadow-[20px_0_40px_rgba(0,0,0,0.2)]'
        ]"
      >
        <!-- 流体填充背景 (保持原位) -->
        <div class="absolute bottom-0 left-0 w-full opacity-10 transition-all duration-1000"
             :style="{ height: `${totalProgress}%`, backgroundColor: 'var(--dynamic-accent)' }"></div>
        
        <!-- 图标/文字切换 (始终居中在 48px 范围内) -->
        <div class="relative z-10 flex items-center justify-center shrink-0 w-4 -ml-0.5">
          <i v-if="!needsAttention" class="pi pi-sync animate-spin text-primary text-sm" :style="{ color: 'var(--dynamic-accent)' }"></i>
          <i v-else class="pi pi-key text-red-400 text-sm"></i>
        </div>

        <!-- 展开内容 (向右侧工作区滑出) -->
        <div class="relative z-10 flex flex-col flex-1 min-w-0 opacity-0 group-hover:opacity-100 transition-all duration-500 -translate-x-4 group-hover:translate-x-0 overflow-hidden pr-2">
          <div class="text-[8px] text-muted uppercase font-black tracking-widest truncate">{{ activeTask?.name }}</div>
          <div class="flex items-center justify-between gap-4 mt-0.5">
            <div class="text-[10px] text-content font-black">{{ totalProgress }}%</div>
            <div class="text-[7px] font-bold uppercase tracking-tighter" :style="{ color: 'var(--dynamic-accent)' }">
              {{ needsAttention ? 'Waiting' : 'Processing' }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.interaction-ball {
  /* 移除 mix-blend-mode 解决透明度问题 */
}

.pop-enter-active, .pop-leave-active {
  transition: all 0.6s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.pop-enter-from, .pop-leave-to {
  transform: translateY(100px) scale(0);
  opacity: 0;
}
</style>
