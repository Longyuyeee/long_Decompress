<template>
  <div class="p-8 space-y-4 bg-base text-content">
    <h1 class="text-2xl font-bold">Diagnostic Page</h1>

    <div class="space-y-2">
      <h2 class="text-lg font-semibold">Window State</h2>
      <pre class="bg-card p-4 rounded text-sm">{{ windowState }}</pre>

      <h2 class="text-lg font-semibold">Screen Info</h2>
      <pre class="bg-card p-4 rounded text-sm">{{ screenInfo }}</pre>

      <h2 class="text-lg font-semibold">LocalStorage</h2>
      <pre class="bg-card p-4 rounded text-sm">{{ localStorageData }}</pre>

      <div class="space-x-2">
        <button @click="clearStorage" class="px-4 py-2 bg-red-500 text-white rounded">
          Clear localStorage
        </button>
        <button @click="refreshData" class="px-4 py-2 bg-blue-500 text-white rounded">
          Refresh Data
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { appWindow } from '@tauri-apps/api/window'

const windowState = ref<any>({})
const screenInfo = ref<any>({})
const localStorageData = ref<any>({})

const loadData = async () => {
  try {
    const pos = await appWindow.outerPosition()
    const size = await appWindow.outerSize()
    windowState.value = {
      position: { x: pos.x, y: pos.y },
      size: { width: size.width, height: size.height }
    }
  } catch (e) {
    windowState.value = { error: String(e) }
  }

  screenInfo.value = {
    availWidth: window.screen.availWidth,
    availHeight: window.screen.availHeight,
    width: window.screen.width,
    height: window.screen.height
  }

  const storage: any = {}
  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i)
    if (key) storage[key] = localStorage.getItem(key)
  }
  localStorageData.value = storage
}

const clearStorage = () => {
  localStorage.clear()
  alert('localStorage cleared!')
  loadData()
}

const refreshData = () => {
  loadData()
}

onMounted(() => {
  loadData()
})
</script>
