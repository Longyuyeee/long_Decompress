import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'

export interface ConfigEntry {
  key: string
  value: any
  category: string
  description?: string
}

export const useConfigStore = defineStore('config', () => {
  const configs = ref<Record<string, any>>({})
  const isLoading = ref(false)

  // 计算属�?
  const privacyMode = computed(() => configs.value['security.privacy_mode'] === 'true' || configs.value['security.privacy_mode'] === true)

  // 加载所有配�?
  const fetchAllConfigs = async () => {
    isLoading.value = true
    try {
      const all: ConfigEntry[] = await invoke('get_all_configs')
      const configMap: Record<string, any> = {}
      all.forEach(item => {
        configMap[item.key] = item.value
      })
      configs.value = configMap
    } catch (e) {
      console.error('加载配置失败:', e)
    } finally {
      isLoading.value = false
    }
  }

  // 更新单个配置
  const setConfig = async (key: string, value: any) => {
    try {
      await invoke('set_config', { key, value: String(value) })
      configs.value[key] = value
    } catch (e) {
      console.error(`设置配置 ${key} 失败:`, e)
    }
  }

  // 批量更新
  const batchUpdate = async (newConfigs: Record<string, any>) => {
    try {
      const entries = Object.entries(newConfigs).map(([key, value]) => ({
        key,
        value: String(value)
      }))
      await invoke('batch_set_configs', { configs: entries })
      configs.value = { ...configs.value, ...newConfigs }
    } catch (e) {
      console.error('批量更新配置失败:', e)
    }
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
