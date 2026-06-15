import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface ConfigEntry {
  key: string
  value: any
  category: string
  description?: string
}

const STORAGE_KEY = 'app-config'

const loadFromStorage = (): Record<string, any> => {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) return JSON.parse(raw)
  } catch { /* ignore */ }
  return {}
}

const saveToStorage = (configs: Record<string, any>) => {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(configs))
  } catch { /* ignore */ }
}

export const useConfigStore = defineStore('config', () => {
  const configs = ref<Record<string, any>>(loadFromStorage())
  const isLoading = ref(false)

  const privacyMode = computed(() => configs.value['security.privacy_mode'] === 'true' || configs.value['security.privacy_mode'] === true)

  const fetchAllConfigs = () => {
    configs.value = loadFromStorage()
  }

  const setConfig = (key: string, value: any) => {
    configs.value[key] = value
    saveToStorage(configs.value)
  }

  const batchUpdate = (newConfigs: Record<string, any>) => {
    configs.value = { ...configs.value, ...newConfigs }
    saveToStorage(configs.value)
  }

  return {
    configs,
    isLoading,
    privacyMode,
    fetchAllConfigs,
    setConfig,
    batchUpdate
  }
})
