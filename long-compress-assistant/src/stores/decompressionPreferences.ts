import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { useAppStore } from './app'

// Workspace defaults survive navigation and can be restored together with pending tasks.
export const useDecompressionPreferences = defineStore('decompressionPreferences', () => {
  const app = useAppStore()
  const outputPath = ref(app.settings.defaultOutputPath)
  const sameDirectory = ref(!app.settings.defaultOutputPath)
  const extractToSubfolder = ref(app.settings.defaultExtractToSubfolder)
  const recycleSource = ref(app.settings.autoDeleteSource)
  watch(() => app.settings.defaultOutputPath, (path, previous) => {
    if (outputPath.value === previous && sameDirectory.value === !previous) { outputPath.value = path; sameDirectory.value = !path }
  })
  watch(() => app.settings.defaultExtractToSubfolder, (value, previous) => {
    if (extractToSubfolder.value === previous) extractToSubfolder.value = value
  })
  watch(() => app.settings.autoDeleteSource, (value, previous) => {
    if (recycleSource.value === previous) recycleSource.value = value
  })
  return { outputPath, sameDirectory, extractToSubfolder, recycleSource }
})
