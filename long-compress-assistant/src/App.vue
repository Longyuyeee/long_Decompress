<template>
  <div id="app" @mousemove="resetIdleTimer" @keydown="resetIdleTimer">
    <MainLayout>
      <PageTransition>
        <RouterView />
      </PageTransition>
    </MainLayout>
    <ToastContainer />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { RouterView } from 'vue-router'
import MainLayout from '@/components/layouts/MainLayout.vue'
import PageTransition from '@/components/transitions/PageTransition.vue'
import ToastContainer from '@/components/ui/ToastContainer.vue'
import { useConfigStore } from '@/stores/config'
import { usePasswordStore } from '@/stores/password'

const configStore = useConfigStore()
const passwordStore = usePasswordStore()

let idleTimer: number | null = null

const resetIdleTimer = () => {
  if (idleTimer) clearTimeout(idleTimer)
  
  const lockTimeStr = configStore.configs['security.auto_lock']
  if (!lockTimeStr || lockTimeStr === '0') return

  const lockTimeMs = parseInt(lockTimeStr) * 60 * 1000
  if (isNaN(lockTimeMs) || lockTimeMs <= 0) return

  idleTimer = window.setTimeout(() => {
    if (passwordStore.isUnlocked) {
      passwordStore.lock()
      console.log('应用闲置，已自动锁定密码库')
    }
  }, lockTimeMs)
}

onMounted(() => {
  resetIdleTimer()
})

onUnmounted(() => {
  if (idleTimer) clearTimeout(idleTimer)
})
</script>

<style>
#app {
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

/* 自定义滚动条样式 */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  @apply bg-transparent;
}

::-webkit-scrollbar-thumb {
  @apply bg-gray-300 dark:bg-gray-700 rounded-full;
}

::-webkit-scrollbar-thumb:hover {
  @apply bg-gray-400 dark:bg-gray-600;
}
</style>
