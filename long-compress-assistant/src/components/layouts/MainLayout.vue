<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import InteractionBall from '@/components/ui/InteractionBall.vue'
import PerformanceMeter from '@/components/ui/PerformanceMeter.vue'
import WindowTitleBar from '@/components/layouts/WindowTitleBar.vue'
import { useAppStore } from '@/stores/app'

const route = useRoute()
const router = useRouter()
const appStore = useAppStore()

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
  <div class="main-container flex flex-col h-screen w-screen overflow-hidden bg-base text-content transition-colors duration-700">
    <!-- 顶部自定义标题栏 (自适应主题) -->
    <WindowTitleBar />

    <div class="main-layout flex flex-1 overflow-hidden transition-all duration-700">
      <!-- 侧边栏 -->
      <aside class="w-16 h-full flex flex-col items-center py-8 border-r border-subtle bg-card/50 backdrop-blur-3xl z-40 transition-all duration-700">
      <!-- Logo (随主题变色) -->
      <div class="w-10 h-10 rounded-2xl flex items-center justify-center transition-all duration-700 mb-12 shadow-lg"
           :style="{ background: `linear-gradient(135deg, var(--dynamic-accent), color-mix(in srgb, var(--dynamic-accent), black 20%))`, boxShadow: `0 0 20px color-mix(in srgb, var(--dynamic-accent) 40%, transparent)` }">
        <i class="pi pi-box text-lg text-white"></i>
      </div>

      <!-- 导航项 -->
      <nav class="flex-1 flex flex-col gap-4 w-full px-2">
        <div v-for="item in navItems" :key="item.name"
             @click="navigateTo(item.name)"
             class="group relative w-full aspect-square flex items-center justify-center rounded-2xl cursor-pointer transition-all duration-500"
             :class="route.name === item.name ? 'bg-primary/10 shadow-inner' : 'hover:bg-primary/5'">
          
          <div class="absolute left-0 w-1 h-4 rounded-full bg-primary transition-all duration-500"
               :class="route.name === item.name ? 'scale-y-100 opacity-100' : 'scale-y-0 opacity-0'"></div>

          <i :class="[item.icon, 'text-lg transition-all duration-500', 
             route.name === item.name ? 'text-primary scale-110' : 'text-muted group-hover:text-content']"></i>
             
          <div class="absolute left-full ml-4 px-4 py-2 rounded-xl backdrop-blur-2xl bg-card border border-subtle text-content text-[10px] font-black tracking-widest uppercase opacity-0 group-hover:opacity-100 -translate-x-4 group-hover:translate-x-0 transition-all pointer-events-none whitespace-nowrap shadow-2xl z-50">
            {{ appStore.t(item.label) }}
          </div>
        </div>
      </nav>
    </aside>

    <!-- 主容器 (路由动画核心) -->
    <main class="flex-1 relative h-full overflow-hidden min-w-[320px]">
      <InteractionBall />
      
      <router-view v-slot="{ Component }">
        <transition name="aero-page" mode="out-in">
          <div :key="route.path" class="h-full overflow-hidden">
            <component :is="Component" />
          </div>
        </transition>
      </router-view>
    </main>
  </div>
</div>
</template>

<style>
/* 苹果风强动效：Aero Page Transition */
.aero-page-enter-active,
.aero-page-leave-active {
  transition: all 0.6s cubic-bezier(0.34, 1.56, 0.64, 1);
  position: absolute;
  width: 100%;
}

.aero-page-enter-from {
  opacity: 0;
  transform: scale(0.94) translateY(30px);
  filter: blur(10px);
}

.aero-page-leave-to {
  opacity: 0;
  transform: scale(1.06) translateY(-20px);
  filter: blur(10px);
}

/* 统一滚动条样式 */
.custom-scrollbar::-webkit-scrollbar { width: 4px; background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: var(--dynamic-accent); opacity: 0.1; border-radius: 10px; }
</style>
