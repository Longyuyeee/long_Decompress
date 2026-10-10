<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useCompressionStore, type CompressionOptions, type FileObject } from '@/stores/compression'
import { useTaskStore } from '@/stores/task'
import type { TaskHistoryRecord } from '@/types/taskHistory'
import { copyExecutionConfig } from '@/types/executionConfig'
import CompressionSettingsPanel from '@/components/compression/CompressionSettingsPanel.vue'
import { COMPRESSIBLE_FORMATS } from '@/utils/compressionFormat'

const props = defineProps<{ record: TaskHistoryRecord }>()
const store = useCompressionStore()
const tasks = useTaskStore()
const sources = ref('')
const output = ref('')
const settings = ref<CompressionOptions>({ ...store.globalSettings, password: '', deleteAfter: false })
const busy = ref(false)
const error = ref('')
let disposed = false
let generation = 0
onUnmounted(() => { disposed = true; generation++; settings.value.password = '' })
const restored = computed(() => {
  const config = props.record.executionConfig
  if (config?.schemaVersion !== 1 || config.kind !== 'archive-compression' || !config.settings) return null
  const s = config.settings
  if (!COMPRESSIBLE_FORMATS.some(format => format.value === s.format)
    || !Number.isInteger(s.level) || s.level < 0 || s.level > 9
    || typeof config.outputDirectory !== 'string' || typeof config.passwordRequired !== 'boolean'
    || typeof s.filename !== 'string' || typeof s.splitSize !== 'string'
    || [s.splitArchive, s.keepStructure, s.verifyAfter, s.createSolidArchive, s.fastPack].some(v => typeof v !== 'boolean')) return null
  return config
})
const created = computed(() => store.groups.some(group => group.retryOfTaskId === props.record.id)
  || tasks.tasks.some(task => task.retryOfTaskId === props.record.id))
watch(() => props.record.id, () => {
  generation++
  sources.value = props.record.sourcePaths.join('\n')
  output.value = restored.value?.outputDirectory || ''
  const config = restored.value && copyExecutionConfig(restored.value)
  settings.value = { ...(config?.kind === 'archive-compression' ? config.settings : store.globalSettings), password: '', deleteAfter: false }
  error.value = ''
}, { immediate: true })

async function createDraft() {
  if (busy.value || created.value) return
  error.value = ''
  const paths = [...new Set(sources.value.split(/\r?\n/).map(path => path.trim()).filter(Boolean))]
  const destination = output.value.trim()
  const confirmed = { ...settings.value }
  const id = props.record.id
  const name = props.record.name
  const request = generation
  if (!paths.length || !destination) { error.value = '请确认来源文件并选择本次输出目录'; return }
  if (restored.value?.passwordRequired && !confirmed.password) { error.value = '上次任务使用了加密，请重新填写本次密码；历史不保存密码。'; return }
  busy.value = true
  try {
    const files: FileObject[] = []
    for (const path of paths) {
      const info = await invoke<{ name: string; size: number; is_dir: boolean }>('get_file_info', { path })
      if (disposed || generation !== request) return
      files.push({ path, name: info.name, size: info.size, isDirectory: info.is_dir, type: info.is_dir ? 'directory' : 'file' })
    }
    const directory = await invoke<{ is_dir: boolean }>('get_file_info', { path: destination })
    if (!directory.is_dir) throw new Error('输出位置不是目录，请重新选择')
    if (disposed || generation !== request) return
    store.addHistoryDraft(files, name, confirmed, destination, id)
    settings.value.password = ''
  } catch (cause) {
    if (!disposed && generation === request) error.value = `无法创建草稿：${String(cause)}`
  } finally { busy.value = false }
}
</script>

<template>
  <section class="detail-section" data-testid="history-compression-draft">
    <h3>再次压缩</h3>
    <p class="text-xs text-muted my-3" data-testid="compression-restore-notice">{{ restored ? '已带入上次执行参数，请确认后创建新草稿。' : '这条历史没有完整的可恢复配置，请重新设置；下方是当前默认参数，不是上次配置。' }}</p>
    <p class="text-xs text-muted mb-3">不会自动启动，不改写原记录；密码需重新填写，删除来源默认关闭。</p>
    <fieldset :disabled="busy || created" class="min-w-0 space-y-3">
      <label class="block text-sm">来源文件或目录（每行一个）<textarea v-model="sources" data-testid="compression-retry-sources" class="retry-path w-full" rows="3" /></label>
      <label class="block text-sm">本次输出目录<input v-model="output" data-testid="compression-retry-output" class="retry-path w-full" /></label>
      <CompressionSettingsPanel v-model="settings" v-model:output-path="output" :compact="true" />
      <button class="retry-path w-full" data-testid="compression-retry-create" @click="createDraft">{{ busy ? '正在检查来源…' : '创建待执行压缩草稿' }}</button>
    </fieldset>
    <p v-if="error" role="alert" class="text-sm text-red-500 mt-2 break-words">{{ error }}</p>
    <p v-if="created" role="status" class="text-sm mt-3">草稿已创建。<a href="#/compress" class="text-primary underline">前往压缩中心确认并启动</a></p>
  </section>
</template>

<style scoped>
.retry-path { min-width: 0; border: 1px solid var(--border-subtle); border-radius: .75rem; padding: .6rem .8rem; color: var(--text-base); background: var(--bg-card); }
fieldset:disabled { opacity: .65; }
</style>
