<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { appWindow } from '@tauri-apps/api/window'
import PerformanceMeter from '@/components/ui/PerformanceMeter.vue'
import GlobalProgressBar from '@/components/ui/GlobalProgressBar.vue'
import WindowTitleBar from '@/components/layouts/WindowTitleBar.vue'
import { useAppStore } from '@/stores/app'

const route = useRoute()
const router = useRouter()
const appStore = useAppStore()
const isFocused = ref(true)
let unlistenFocus: any = null

onMounted(async () => {
  unlistenFocus = await appWindow.onFocusChanged(({ payload: focused }) => {
    isFocused.value = focused
  })
})

onUnmounted(() => {
  if (unlistenFocus) unlistenFocus()
})

const navItems = [
  { name: 'Decompress', icon: 'pi pi-folder-open', label: 'nav.decompress', shortcut: 'Ctrl+O' },
  { name: 'Compress', icon: 'pi pi-box', label: 'nav.compress', shortcut: 'Ctrl+N' },
  { name: 'Vault', icon: 'pi pi-shield', label: 'nav.vault', shortcut: 'Ctrl+V' },
  { name: 'Settings', icon: 'pi pi-cog', label: 'nav.settings', shortcut: 'Ctrl+,' }
]

const navigateTo = (name: string) => {
  router.push({ name })
}
</script>

<template>
  <!-- 主容器：通过 p-[1px] 留出系统 Resize 缓冲带 -->
  <div 
    class="main-container flex flex-col h-screen w-screen bg-transparent p-[1px] overflow-hidden"
    style="box-sizing: border-box;"
  >
    <div class="flex-1 flex flex-col overflow-hidden bg-base text-content rounded-xl relative border transition-all duration-300"
         :class="[isFocused ? 'border-primary/20 shadow-[0_8px_32px_rgba(0,0,0,0.2)]' : 'border-subtle shadow-sm']">
      
      <!-- 绝杀闪烁：8个隐形游标同步区 -->
      <div class="absolute top-0 left-0 right-0 h-[6px] cursor-n-resize z-[9999]"></div>
      <div class="absolute bottom-0 left-0 right-0 h-[6px] cursor-s-resize z-[9999]"></div>
      <div class="absolute top-0 bottom-0 left-0 w-[6px] cursor-w-resize z-[9999]"></div>
      <div class="absolute top-0 bottom-0 right-0 w-[6px] cursor-e-resize z-[9999]"></div>
      <div class="absolute top-0 left-0 w-[8px] h-[8px] cursor-nw-resize z-[10000]"></div>
      <div class="absolute top-0 right-0 w-[8px] h-[8px] cursor-ne-resize z-[10000]"></div>
      <div class="absolute bottom-0 left-0 w-[8px] h-[8px] cursor-sw-resize z-[10000]"></div>
      <div class="absolute bottom-0 right-0 w-[8px] h-[8px] cursor-se-resize z-[10000]"></div>

      <!-- 顶部自定义标题栏 -->
      <WindowTitleBar class="shrink-0" />

      <div class="main-layout flex flex-1 overflow-hidden relative">
        <!-- 侧边栏 - 精简专业版 -->
        <aside class="w-14 h-full flex flex-col items-center pt-4 pb-4 border-r border-subtle bg-card/60 backdrop-blur-xl z-50 shrink-0 overflow-visible relative">
          <nav class="flex-1 flex flex-col gap-2 w-full px-1.5 overflow-visible">
            <div v-for="item in navItems" :key="item.name"
                 @click="navigateTo(item.name)"
                 class="group relative w-full aspect-square flex items-center justify-center rounded-lg cursor-pointer transition-all duration-200"
                 :class="route.name === item.name ? 'bg-primary/15 shadow-sm' : 'hover:bg-primary/5'">

              <div class="absolute left-0 w-0.5 h-5 rounded-full bg-primary transition-all duration-200"
                   :class="route.name === item.name ? 'scale-y-100 opacity-100' : 'scale-y-0 opacity-0'"></div>

              <i :class="[item.icon, 'text-base transition-all duration-200',
                 route.name === item.name ? 'text-primary' : 'text-muted group-hover:text-content']"></i>

              <div class="absolute left-full ml-3 px-3 py-1.5 rounded-lg backdrop-blur-xl bg-card/95 border border-subtle text-content text-[0.625rem] font-bold tracking-wide uppercase opacity-0 group-hover:opacity-100 -translate-x-2 group-hover:translate-x-0 transition-all pointer-events-none whitespace-nowrap shadow-xl z-[100]">
                {{ appStore.t(item.label) }}
                <span class="text-[0.4375rem] text-dim ml-1.5 font-mono opacity-60">({{ item.shortcut }})</span>
              </div>
            </div>
          </nav>
        </aside>

        <!-- 主内容区 -->
        <main class="flex-1 relative h-full overflow-hidden min-w-0 z-10 bg-base">
          <router-view v-slot="{ Component }">
            <transition name="aero-page" mode="out-in">
              <div :key="route.path" class="h-full w-full overflow-hidden">
                <component :is="Component" />
              </div>
            </transition>
          </router-view>

          <!-- 全局进度指示器 -->
          <GlobalProgressBar />
        </main>
      </div>
    </div>
  </div>
</template>

<style>
:global(html), :global(body), :global(#app) {
  background-color: transparent !important;
  margin: 0;
  padding: 0;
  overflow: hidden;
  height: 100vh;
  width: 100vw;
}

.aero-page-enter-active {
  transition: opacity 0.2s ease-out, transform 0.2s ease-out;
}

.aero-page-leave-active {
  transition: opacity 0.15s ease-in, transform 0.15s ease-in;
}

.aero-page-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.aero-page-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

.custom-scrollbar::-webkit-scrollbar { width: 4px; background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: var(--dynamic-accent); border-radius: 10px; }
</style>
