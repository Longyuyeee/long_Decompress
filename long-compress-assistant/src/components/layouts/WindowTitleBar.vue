<script setup lang="ts">
import { appWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/tauri'
const minimize = () => appWindow.minimize()
const toggleMaximize = () => appWindow.toggleMaximize()
const closeApp = () => appWindow.close()
const startDragging = (event: MouseEvent) => {
  if (event.button !== 0 || (event.target as HTMLElement | null)?.closest('.control-btn')) return
  void invoke('start_native_window_drag').catch(error => {
    console.warn('Native title-bar dragging is unavailable:', error)
  })
}
</script>

<template>
  <div class="window-titlebar flex items-center justify-end h-8 bg-gradient-to-r from-card/40 via-card/30 to-card/40 backdrop-blur-2xl border-b border-subtle/50 select-none relative z-[100] shadow-sm" @mousedown="startDragging">

    <!-- 右侧控制组 -->
    <div class="flex h-full items-center">
      <button
        type="button"
        @click="minimize"
        class="control-btn hover:bg-content/8 active:bg-content/12"
        title="最小化"
        aria-label="最小化"
      >
        <i class="pi pi-minus text-xs"></i>
      </button>
      <button
        type="button"
        @click="toggleMaximize"
        class="control-btn hover:bg-content/8 active:bg-content/12"
        title="最大化或还原"
        aria-label="最大化或还原"
      >
        <i class="pi pi-stop text-xs"></i>
      </button>
      <button
        type="button"
        @click="closeApp"
        class="control-btn hover:bg-red-500/90 hover:text-white active:bg-red-600 group"
        title="关闭"
        aria-label="关闭"
      >
        <i class="pi pi-times text-xs group-hover:scale-110 transition-transform duration-200"></i>
      </button>
    </div>
  </div>
</template>

<style scoped>
.control-btn {
  @apply w-12 h-full flex items-center justify-center text-muted transition-all duration-300 cursor-default;
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
