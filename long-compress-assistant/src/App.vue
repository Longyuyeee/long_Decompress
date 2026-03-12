<template>
  <div id="app" @mousemove="resetIdleTimer" @keydown="resetIdleTimer">
    <MainLayout />
    <ToastContainer />
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import MainLayout from '@/components/layouts/MainLayout.vue'
import ToastContainer from '@/components/ui/ToastContainer.vue'
import { useConfigStore } from '@/stores/config'
import { usePasswordStore } from '@/stores/password'

const configStore = useConfigStore()
const passwordStore = usePasswordStore()

let idleTimer: any = null

const resetIdleTimer = () => {
  if (idleTimer) clearTimeout(idleTimer)
  const lockTimeStr = configStore.configs['security.auto_lock']
  if (!lockTimeStr || lockTimeStr === '0') return
  const lockTimeMs = parseInt(lockTimeStr) * 60 * 1000
  if (isNaN(lockTimeMs) || lockTimeMs <= 0) return

  idleTimer = window.setTimeout(() => {
    if (passwordStore.isUnlocked) {
      passwordStore.lock()
    }
  }, lockTimeMs)
}

onMounted(() => resetIdleTimer())
onUnmounted(() => { if (idleTimer) clearTimeout(idleTimer) })
</script>

<style>
#app {
  font-family: 'Plus Jakarta Sans', system-ui, sans-serif;
  -webkit-font-smoothing: antialiased;
}
</style>
