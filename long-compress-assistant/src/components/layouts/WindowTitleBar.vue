<script setup lang="ts">
import { appWindow } from '@tauri-apps/api/window'
import { useAppStore } from '@/stores/app'

const appStore = useAppStore()

const minimize = () => appWindow.minimize()
const toggleMaximize = () => appWindow.toggleMaximize()
const closeApp = () => appWindow.close()
</script>

<template>
  <div class="window-titlebar flex items-center justify-between h-8 bg-card/30 backdrop-blur-3xl border-b border-subtle select-none relative z-[100] shadow-[0_4px_12px_rgba(0,0,0,0.08)]" data-tauri-drag-region>
    <!-- 左侧标题 & 主题指示线 -->
    <div class="flex items-center gap-3 px-4 pointer-events-none" data-tauri-drag-region>
      <div class="w-1.5 h-1.5 rounded-full bg-primary shadow-[0_0_8px_var(--dynamic-accent)]"></div>
      <span class="text-[10px] font-black text-content/60 uppercase tracking-[0.2em] mt-0.5">{{ appStore.t('app.name') || 'Long解压' }}</span>
    </div>

    <!-- 右侧控制组 -->
    <div class="flex h-full items-center">
      <button @click="minimize" class="control-btn hover:bg-content/5">
        <i class="pi pi-minus text-[8px]"></i>
      </button>
      <button @click="toggleMaximize" class="control-btn hover:bg-content/5">
        <i class="pi pi-stop text-[8px]"></i>
      </button>
      <button @click="closeApp" class="control-btn hover:bg-red-500 hover:text-white group">
        <i class="pi pi-times text-[8px] group-hover:scale-110 transition-transform"></i>
      </button>
    </div>

    <!-- 移除顶部装饰线，因为它会干扰窗口边缘缩放判定 -->
  </div>
</template>

<style scoped>
.window-titlebar {
  /* 确保标题栏不会被内容遮挡 */
  -webkit-app-region: drag;
}

.control-btn {
  @apply w-10 h-full flex items-center justify-center text-muted transition-all duration-200 cursor-default;
  -webkit-app-region: no-drag;
}

.control-btn i {
  pointer-events: none;
}
</style>
