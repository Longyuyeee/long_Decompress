<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { appWindow } from '@tauri-apps/api/window'
import InteractionBall from '@/components/ui/InteractionBall.vue'
import PerformanceMeter from '@/components/ui/PerformanceMeter.vue'
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
  { name: 'Decompress', icon: 'pi pi-folder-open', label: 'nav.decompress' },
  { name: 'Compress', icon: 'pi pi-box', label: 'nav.compress' },
  { name: 'Vault', icon: 'pi pi-shield', label: 'nav.vault' },
  { name: 'Settings', icon: 'pi pi-cog', label: 'nav.settings' }
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
    <div class="flex-1 flex flex-col overflow-hidden bg-base text-content rounded-xl relative border border-white/5 transition-[border-color] duration-500"
         :class="[isFocused ? 'border-primary/30 shadow-[0_8px_32px_rgba(0,0,0,0.15)]' : 'border-white/5 shadow-none']">
      
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
        <!-- 侧边栏 -->
        <aside class="w-16 h-full flex flex-col items-center py-6 border-r border-subtle bg-card/40 backdrop-blur-2xl z-50 shrink-0 overflow-visible relative">
          <div class="mb-10 mt-2 shrink-0">
            <div class="w-10 h-10 rounded-2xl flex items-center justify-center shadow-lg"
                 :style="{ background: `linear-gradient(135deg, var(--dynamic-accent), color-mix(in srgb, var(--dynamic-accent), black 20%))`, boxShadow: `0 0 20px color-mix(in srgb, var(--dynamic-accent) 40%, transparent)` }">
              <i class="pi pi-box text-lg text-white"></i>
            </div>
          </div>

          <nav class="flex-1 flex flex-col gap-4 w-full px-2 overflow-visible">
            <div v-for="item in navItems" :key="item.name"
                 @click="navigateTo(item.name)"
                 class="group relative w-full aspect-square flex items-center justify-center rounded-2xl cursor-pointer transition-colors duration-300"
                 :class="route.name === item.name ? 'bg-primary/10' : 'hover:bg-primary/5'">
              
              <div class="absolute left-0 w-1 h-4 rounded-full bg-primary transition-all duration-500"
                   :class="route.name === item.name ? 'scale-y-100 opacity-100' : 'scale-y-0 opacity-0'"></div>

              <i :class="[item.icon, 'text-lg transition-all duration-500', 
                 route.name === item.name ? 'text-primary scale-110' : 'text-muted group-hover:text-content']"></i>
                 
              <div class="absolute left-full ml-4 px-4 py-2 rounded-xl backdrop-blur-3xl bg-card/90 border border-subtle text-content text-[10px] font-black tracking-widest uppercase opacity-0 group-hover:opacity-100 -translate-x-4 group-hover:translate-x-0 transition-all pointer-events-none whitespace-nowrap shadow-2xl z-[100]">
                {{ appStore.t(item.label) }}
              </div>
            </div>
          </nav>
        </aside>

        <!-- 主内容区 -->
        <main class="flex-1 relative h-full overflow-hidden min-w-0 z-10">
          <InteractionBall />
          
          <router-view v-slot="{ Component }">
            <transition name="aero-page" mode="out-in">
              <div :key="route.path" class="h-full w-full overflow-hidden absolute inset-0">
                <component :is="Component" />
              </div>
            </transition>
          </router-view>
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

.aero-page-enter-active,
.aero-page-leave-active {
  transition: all 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
  position: absolute;
  width: 100%;
  height: 100%;
}

.aero-page-enter-from { 
  opacity: 0; 
  transform: scale(0.98) translateY(10px); 
  filter: blur(8px); 
}

.aero-page-leave-to { 
  opacity: 0; 
  transform: scale(1.02) translateY(-10px); 
  filter: blur(8px); 
}

.custom-scrollbar::-webkit-scrollbar { width: 4px; background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: var(--dynamic-accent); border-radius: 10px; }
</style>
