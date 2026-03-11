<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import InteractionBall from '@/components/ui/InteractionBall.vue'
import PerformanceMeter from '@/components/ui/PerformanceMeter.vue'
import { useAppStore } from '@/stores/app'

const route = useRoute()
const router = useRouter()
const appStore = useAppStore()

const navItems = [
  { name: 'Decompress', icon: 'pi pi-folder-open', label: '解压' },
  { name: 'Compress', icon: 'pi pi-file-zip', label: '压缩' },
  { name: 'Vault', icon: 'pi pi-shield', label: '保险箱' },
  { name: 'Settings', icon: 'pi pi-cog', label: '设置' }
]

const navigateTo = (name: string) => {
  router.push({ name })
}
</script>

<template>
  <div class="main-layout flex h-screen w-screen overflow-hidden text-white transition-colors duration-500"
       :class="appStore.currentTheme === 'dark' ? 'bg-black' : 'bg-gray-900'">
    
    <!-- 极简侧边栏 -->
    <aside class="w-20 h-full flex flex-col items-center py-8 border-r border-white/10 bg-white/5 backdrop-blur-3xl z-40">
      <!-- Logo -->
      <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center shadow-[0_0_20px_rgba(59,130,246,0.5)] mb-12">
        <i class="pi pi-box text-xl text-white"></i>
      </div>

      <!-- 核心导航 -->
      <nav class="flex-1 flex flex-col gap-6 w-full px-4">
        <div v-for="item in navItems" :key="item.name"
             @click="navigateTo(item.name)"
             class="group relative w-full aspect-square flex items-center justify-center rounded-2xl cursor-pointer transition-all duration-300"
             :class="route.name === item.name ? 'bg-white/10 shadow-inner' : 'hover:bg-white/5'">
          
          <i :class="[item.icon, 'text-xl transition-all duration-500', 
             route.name === item.name ? 'text-blue-400 scale-110' : 'text-white/40 group-hover:text-white/80']"></i>
             
          <!-- 悬浮提示 -->
          <div class="absolute left-full ml-4 px-3 py-1.5 rounded-lg bg-white/10 backdrop-blur-xl border border-white/10 text-[10px] font-bold tracking-widest uppercase opacity-0 group-hover:opacity-100 -translate-x-4 group-hover:translate-x-0 transition-all pointer-events-none whitespace-nowrap">
            {{ item.label }}
          </div>
        </div>
      </nav>
    </aside>

    <!-- 主工作区 -->
    <main class="flex-1 relative h-full overflow-y-auto custom-scrollbar">
      <!-- 灵动通知球 (MS5) -->
      <InteractionBall />
      
      <!-- 路由视图过渡 -->
      <router-view v-slot="{ Component }">
        <transition name="fade" mode="out-in">
          <component :is="Component" />
        </transition>
      </router-view>
    </main>

    <!-- 系统负载计 (MS5) -->
    <PerformanceMeter />

    <!-- 全局报错提示 -->
    <transition name="toast">
      <div v-if="appStore.error" 
           class="fixed top-6 right-6 p-4 rounded-2xl bg-red-500/20 border border-red-500/30 backdrop-blur-xl flex items-start gap-4 shadow-2xl z-[999] max-w-sm">
        <i class="pi pi-exclamation-triangle text-red-400 mt-0.5"></i>
        <div class="flex-1 text-sm text-red-100 font-medium">{{ appStore.error }}</div>
        <button @click="appStore.clearError" class="text-red-400/50 hover:text-red-400"><i class="pi pi-times"></i></button>
      </div>
    </transition>
  </div>
</template>

<style>
/* 全局基础样式重置与排版 */
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;700;900&family=JetBrains+Mono:wght@400;700&display=swap');

body {
  font-family: 'Inter', system-ui, sans-serif;
  margin: 0;
  padding: 0;
  overflow: hidden;
  background: black;
}

.font-mono {
  font-family: 'JetBrains Mono', monospace;
}

/* 路由过渡动效 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.4s ease, transform 0.4s ease;
}
.fade-enter-from {
  opacity: 0;
  transform: translateY(10px) scale(0.99);
}
.fade-leave-to {
  opacity: 0;
  transform: translateY(-10px) scale(0.99);
}

/* 报错提示过渡 */
.toast-enter-active,
.toast-leave-active {
  transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(50px);
}

/* 隐形滚动条 */
.custom-scrollbar::-webkit-scrollbar {
  width: 0px;
  background: transparent;
}
</style>
