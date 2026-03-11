<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  status: 'pending' | 'running' | 'paused'
}>()

const emit = defineEmits<{
  (e: 'start'): void
  (e: 'pause'): void
  (e: 'stop'): void
}>()
</script>

<template>
  <div class="morphing-btn-container relative flex items-center justify-center gap-4 h-16 transition-all duration-700">
    <!-- 主按钮 (Start / Pause) -->
    <button 
      @click="status === 'running' ? emit('pause') : emit('start')"
      class="relative z-10 flex items-center justify-center transition-all duration-500 overflow-hidden shadow-2xl"
      :class="[
        status === 'running' 
          ? 'w-48 h-12 rounded-xl bg-white/10 border border-white/20' 
          : 'w-64 h-14 rounded-2xl bg-white text-black font-black'
      ]"
    >
      <div class="flex items-center gap-3">
        <i :class="['pi text-sm transition-all duration-500', 
                   status === 'running' ? 'pi-pause text-white' : 'pi-play text-black']"></i>
        <span class="text-[10px] uppercase font-black tracking-[0.2em] transition-all duration-500"
              :class="status === 'running' ? 'text-white' : 'text-black'">
          {{ status === 'running' ? '暂停任务' : '开始处理' }}
        </span>
      </div>
    </button>

    <!-- 侧边滑出按钮 (Stop) -->
    <Transition name="slide-side">
      <button 
        v-if="status === 'running' || status === 'paused'"
        @click="emit('stop')"
        class="w-12 h-12 rounded-xl bg-red-500/10 border border-red-500/30 flex items-center justify-center text-red-500 hover:bg-red-500/20 transition-all shadow-lg"
      >
        <i class="pi pi-stop-circle"></i>
      </button>
    </Transition>
  </div>
</template>

<style scoped>
.slide-side-enter-active, .slide-side-leave-active {
  transition: all 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.slide-side-enter-from, .slide-side-leave-to {
  transform: translateX(-20px) scale(0);
  opacity: 0;
}
</style>
