<script setup lang="ts">
import { appWindow } from '@tauri-apps/api/window'
import { useAppStore } from '@/stores/app'

const appStore = useAppStore()

const minimize = () => appWindow.minimize()
const toggleMaximize = () => appWindow.toggleMaximize()
const closeApp = () => appWindow.close()
</script>

<template>
  <div class="window-titlebar flex items-center justify-between h-11 bg-gradient-to-r from-card/40 via-card/30 to-card/40 backdrop-blur-2xl border-b border-subtle/50 select-none relative z-[100] shadow-sm" data-tauri-drag-region>
    <!-- 左侧标题 & 动态指示点 -->
    <div class="flex items-center gap-3.5 px-5" data-tauri-drag-region>
      <div class="relative">
        <div class="w-2 h-2 rounded-full bg-primary shadow-[0_0_12px_var(--dynamic-accent)] animate-pulse"></div>
        <div class="absolute inset-0 w-2 h-2 rounded-full bg-primary/30 animate-ping"></div>
      </div>
      <span class="text-sm font-bold text-content/70 tracking-wide">{{ appStore.t('app.name') }}</span>
    </div>

    <!-- 右侧控制组 -->
    <div class="flex h-full items-center">
      <button
        type="button"
        @click="minimize"
        class="control-btn hover:bg-content/8 active:bg-content/12"
        :title="appStore.t('common.minimize')"
        :aria-label="appStore.t('common.minimize')"
      >
        <i class="pi pi-minus text-xs"></i>
      </button>
      <button
        type="button"
        @click="toggleMaximize"
        class="control-btn hover:bg-content/8 active:bg-content/12"
        :title="appStore.t('common.maximize')"
        :aria-label="appStore.t('common.maximize')"
      >
        <i class="pi pi-stop text-xs"></i>
      </button>
      <button
        type="button"
        @click="closeApp"
        class="control-btn hover:bg-red-500/90 hover:text-white active:bg-red-600 group"
        :title="appStore.t('common.close')"
        :aria-label="appStore.t('common.close')"
      >
        <i class="pi pi-times text-xs group-hover:scale-110 transition-transform duration-200"></i>
      </button>
    </div>
  </div>
</template>

<style scoped>
.window-titlebar {
  -webkit-app-region: drag;
}

.control-btn {
  @apply w-12 h-full flex items-center justify-center text-muted transition-all duration-300 cursor-default;
  -webkit-app-region: no-drag;
}

.control-btn:active {
  transform: scale(0.95);
}

.control-btn i {
  pointer-events: none;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

@keyframes ping {
  75%, 100% {
    transform: scale(2);
    opacity: 0;
  }
}
</style>
