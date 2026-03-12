<template>
  <div class="fixed top-8 right-8 z-[9999] flex flex-col gap-3 pointer-events-none">
    <transition-group name="aero-toast">
      <div 
        v-for="toast in uiStore.toasts" 
        :key="toast.id"
        class="toast-item pointer-events-auto flex items-center p-5 rounded-[1.5rem] shadow-2xl border border-subtle bg-modal text-content min-w-[280px] relative overflow-hidden"
      >
        <!-- 侧边指示条 -->
        <div class="absolute left-0 top-0 bottom-0 w-1.5" :class="getTypeColor(toast.type)"></div>
        
        <div class="w-10 h-10 rounded-2xl flex items-center justify-center mr-4" :class="getTypeBg(toast.type)">
          <i :class="[getIcon(toast.type), getTypeTextColor(toast.type)]" class="text-lg"></i>
        </div>
        
        <div class="flex-1 pr-8">
          <div class="text-[9px] font-black uppercase tracking-widest opacity-30 mb-1">{{ toast.type }}</div>
          <p class="text-xs font-bold leading-relaxed">{{ toast.message }}</p>
        </div>

        <button @click="uiStore.removeToast(toast.id)" class="absolute right-4 top-1/2 -translate-y-1/2 text-dim hover:text-content transition-colors">
          <i class="pi pi-times text-[10px]"></i>
        </button>
      </div>
    </transition-group>
  </div>
</template>

<script setup lang="ts">
import { useUIStore } from '@/stores/ui'

const uiStore = useUIStore()

const getIcon = (type: string) => {
  switch (type) {
    case 'success': return 'pi pi-check-circle'
    case 'error': return 'pi pi-times-circle'
    case 'warning': return 'pi pi-exclamation-triangle'
    default: return 'pi pi-info-circle'
  }
}

const getTypeColor = (type: string) => {
  switch (type) {
    case 'success': return 'bg-green-500'
    case 'error': return 'bg-red-500'
    case 'warning': return 'bg-yellow-500'
    default: return 'bg-primary'
  }
}

const getTypeBg = (type: string) => {
  switch (type) {
    case 'success': return 'bg-green-500/10'
    case 'error': return 'bg-red-500/10'
    case 'warning': return 'bg-yellow-500/10'
    default: return 'bg-primary/10'
  }
}

const getTypeTextColor = (type: string) => {
  switch (type) {
    case 'success': return 'text-green-500'
    case 'error': return 'text-red-500'
    case 'warning': return 'text-yellow-500'
    default: return 'text-primary'
  }
}
</script>

<style scoped>
.aero-toast-enter-active,
.aero-toast-leave-active {
  transition: all 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.aero-toast-enter-from {
  opacity: 0;
  transform: translateX(50px) scale(0.9);
}
.aero-toast-leave-to {
  opacity: 0;
  transform: scale(0.8) translateY(-20px);
}
</style>
