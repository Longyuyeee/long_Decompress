<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { appWindow } from '@tauri-apps/api/window'
import PerformanceMeter from '@/components/ui/PerformanceMeter.vue'
import GlobalProgressBar from '@/components/ui/GlobalProgressBar.vue'
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

      <div class="main-layout flex flex-1 overflow-hidden relative">
        <!-- 侧边栏 - 华丽扁平版 -->
        <aside class="w-16 h-full flex flex-col items-center pt-6 pb-6 border-r border-subtle/50 bg-gradient-to-b from-card/70 via-card/60 to-card/50 backdrop-blur-xl z-50 shrink-0 overflow-visible relative">
          <nav class="flex-1 flex flex-col gap-3 w-full px-2 overflow-visible">
            <div v-for="item in navItems" :key="item.name"
                 @click="navigateTo(item.name)"
                 class="group relative w-full aspect-square flex items-center justify-center rounded-xl cursor-pointer transition-all duration-300"
                 :class="route.name === item.name ? 'bg-primary/20 shadow-lg scale-105' : 'hover:bg-primary/8 hover:scale-102'">

              <div class="absolute left-0 w-1 h-7 rounded-full bg-gradient-to-b from-primary to-primary/50 transition-all duration-300 shadow-[0_0_8px_var(--dynamic-accent)]"
                   :class="route.name === item.name ? 'scale-y-100 opacity-100' : 'scale-y-0 opacity-0'"></div>

              <i :class="[item.icon, 'text-lg transition-all duration-300',
                 route.name === item.name ? 'text-primary scale-110' : 'text-muted group-hover:text-content group-hover:scale-105']"></i>

              <div class="absolute left-full ml-4 px-4 py-2 rounded-xl backdrop-blur-3xl bg-card border border-subtle shadow-[0_8px_32px_rgba(0,0,0,0.24)] text-content text-xs font-semibold tracking-wide opacity-0 group-hover:opacity-100 -translate-x-3 group-hover:translate-x-0 transition-all duration-300 pointer-events-none whitespace-nowrap z-[100]">
                {{ appStore.t(item.label) }}
                <span class="text-[0.625rem] text-muted ml-2 font-mono">({{ item.shortcut }})</span>
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
  transition: opacity 0.35s cubic-bezier(0.4, 0, 0.2, 1), transform 0.35s cubic-bezier(0.4, 0, 0.2, 1);
}

.aero-page-leave-active {
  transition: opacity 0.25s cubic-bezier(0.4, 0, 1, 1), transform 0.25s cubic-bezier(0.4, 0, 1, 1);
}

.aero-page-enter-from {
  opacity: 0;
  transform: translateY(12px) scale(0.98);
}

.aero-page-leave-to {
  opacity: 0;
  transform: translateY(-8px) scale(1.01);
}

.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: var(--dynamic-accent);
  border-radius: 10px;
  transition: background 0.3s ease;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: var(--dynamic-accent-alt);
}
</style>
