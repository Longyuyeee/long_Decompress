<template>
  <div class="toast-stack fixed z-[9999] flex flex-col gap-3 pointer-events-none" aria-live="polite" aria-atomic="false">
    <transition-group name="aero-toast">
      <div 
        v-for="toast in uiStore.toasts" 
        :key="toast.id"
        class="toast-item pointer-events-auto flex items-center p-4 rounded-2xl shadow-2xl border border-subtle bg-modal/95 backdrop-blur-xl text-content relative overflow-hidden"
        :role="toast.type === 'error' ? 'alert' : 'status'"
      >
        <!-- 侧边指示条 -->
        <div class="absolute left-0 top-0 bottom-0 w-1.5" :class="getTypeColor(toast.type)"></div>
        
        <div class="w-10 h-10 rounded-2xl flex items-center justify-center mr-4" :class="getTypeBg(toast.type)">
          <i :class="[getIcon(toast.type), getTypeTextColor(toast.type)]" class="text-lg"></i>
        </div>
        
        <div class="flex-1 pr-8">
          <div class="text-xs font-black tracking-widest opacity-75 mb-1">{{ getTypeLabel(toast.type) }}</div>
          <p class="text-xs font-bold leading-relaxed">{{ toast.message }}</p>
        </div>

        <button @click="uiStore.removeToast(toast.id)" :aria-label="`关闭${getTypeLabel(toast.type)}提示`" class="absolute right-4 top-1/2 -translate-y-1/2 w-8 h-8 rounded-lg flex items-center justify-center text-dim hover:text-content hover:bg-input transition-colors">
          <i class="pi pi-times text-sm"></i>
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

const getTypeLabel = (type: string) => ({
  success: '成功', error: '错误', warning: '警告', info: '信息'
}[type] || '信息')

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
.toast-stack {
  top: 1.25rem;
  right: 1.25rem;
  width: min(24rem, calc(100vw - 2rem));
}

.toast-item {
  min-width: 17.5rem;
  box-shadow: 0 18px 50px rgba(0, 0, 0, 0.28), 0 0 0 1px color-mix(in srgb, var(--dynamic-accent) 8%, transparent);
}

@media (max-width: 640px) {
  .toast-stack { top: 0.75rem; right: 0.75rem; width: calc(100vw - 1.5rem); }
  .toast-item { min-width: 0; }
}

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
