<template>
  <div class="fixed top-4 right-4 z-[9999] flex flex-col gap-2 pointer-events-none">
    <transition-group name="toast">
      <div 
        v-for="toast in uiStore.toasts" 
        :key="toast.id"
        class="toast-item pointer-events-auto flex items-center p-4 rounded-xl shadow-lg border backdrop-blur-md"
        :class="{
          'bg-green-50/90 border-green-200 text-green-800 dark:bg-green-900/80 dark:border-green-800 dark:text-green-100': toast.type === 'success',
          'bg-red-50/90 border-red-200 text-red-800 dark:bg-red-900/80 dark:border-red-800 dark:text-red-100': toast.type === 'error',
          'bg-yellow-50/90 border-yellow-200 text-yellow-800 dark:bg-yellow-900/80 dark:border-yellow-800 dark:text-yellow-100': toast.type === 'warning',
          'bg-blue-50/90 border-blue-200 text-blue-800 dark:bg-blue-900/80 dark:border-blue-800 dark:text-blue-100': toast.type === 'info',
        }"
      >
        <i :class="getIcon(toast.type)" class="text-xl mr-3"></i>
        <p class="text-sm font-medium pr-6">{{ toast.message }}</p>
        <button @click="uiStore.removeToast(toast.id)" class="absolute right-3 top-1/2 -translate-y-1/2 opacity-50 hover:opacity-100">
          <i class="pi pi-times text-xs"></i>
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
</script>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
.toast-enter-from {
  opacity: 0;
  transform: translateX(100%) scale(0.9);
}
.toast-leave-to {
  opacity: 0;
  transform: scale(0.9) translateY(-20px);
}
</style>
