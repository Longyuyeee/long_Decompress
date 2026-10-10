<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useCompressionStore } from '@/stores/compression'
import { usePdfWorkspaceStore } from '@/stores/pdfWorkspace'
import { useTaskStore } from '@/stores/task'
import type { TaskHistoryRecord } from '@/types/taskHistory'
import { copyExecutionConfig } from '@/types/executionConfig'
import { inferImageFormat, validateImageCandidate } from '@/utils/imageCompressionWorkspace'
import { validateVideoCandidate } from '@/utils/videoCompressionWorkspace'
import { isPdfCandidate } from '@/utils/pdfOptimizationWorkspace'

const props = defineProps<{ record: TaskHistoryRecord }>()
const store = useCompressionStore()
const pdf = usePdfWorkspaceStore()
const tasks = useTaskStore()
const source = ref('')
const output = ref('')
const busy = ref(false)
const error = ref('')
let generation = 0
let disposed = false
onUnmounted(() => { disposed = true; generation++ })
const restored = computed(() => {
  try {
    const config = copyExecutionConfig(props.record.executionConfig)
    const kind = props.record.workloadKind
    if (!config || !((kind === 'image' && config.kind === 'image-compression')
      || (kind === 'video' && config.kind === 'video-compression') || (kind === 'pdf' && config.kind === 'pdf-optimization'))) return null
    const values = [...Object.values(config), ...('settings' in config ? Object.values(config.settings) : [])]
    if (values.some(value => value === undefined || (typeof value === 'number' && !Number.isFinite(value)))) return null
    if (config.kind === 'pdf-optimization' && (!['lossless-organization', 'compatible-image-optimization'].includes(config.mode) || typeof config.allowLargerOutput !== 'boolean')) return null
    if (config.kind === 'video-compression' && (!['clear', 'balanced', 'small'].includes(config.settings.preset) || !Number.isInteger(config.settings.quality) || config.settings.quality < 0 || config.settings.quality > 100)) return null
    if (config.kind === 'image-compression' && (!['lossy', 'lossless'].includes(config.settings.mode)
      || !['keep', 'limit'].includes(config.settings.resizeMode) || !['keep', 'jpeg', 'png', 'webp'].includes(config.settings.outputFormat)
      || !['rename', 'skip', 'replace-if-smaller'].includes(config.settings.conflictPolicy)
      || typeof config.settings.preserveMetadata !== 'boolean' || typeof config.settings.outputDirectory !== 'string'
      || !Number.isInteger(config.settings.quality) || config.settings.quality < 0 || config.settings.quality > 100)) return null
    if ('outputDirectory' in config && typeof config.outputDirectory !== 'string') return null
    return config
  } catch { return null }
})
const created = computed(() => [...store.imageItems, ...store.videoItems, ...pdf.items, ...tasks.tasks]
  .some(item => item.retryOfTaskId === props.record.id))
watch(() => props.record.id, () => {
  generation++
  source.value = props.record.sourcePaths.length === 1 ? props.record.sourcePaths[0] : ''
  const config = restored.value
  output.value = config?.kind === 'image-compression' ? config.settings.outputDirectory : config?.outputDirectory || ''
  error.value = ''
}, { immediate: true })

async function createDraft() {
  if (busy.value || created.value) return
  error.value = ''
  const path = source.value.trim()
  const destination = output.value.trim()
  const recordId = props.record.id
  const kind = props.record.workloadKind
  const config = restored.value && copyExecutionConfig(restored.value)
  const request = generation
  if (!path) { error.value = '请确认本次要处理的单个来源文件'; return }
  busy.value = true
  try {
    const info = await invoke<{ name: string; size: number; is_dir: boolean }>('get_file_info', { path })
    const candidate = { path, name: info.name, size: info.size, isDirectory: info.is_dir }
    const validation = kind === 'image' ? validateImageCandidate(candidate) : kind === 'video' ? validateVideoCandidate(candidate) : { accepted: isPdfCandidate(candidate) }
    if (!validation.accepted) throw new Error('来源类型不匹配，请选择对应类型的文件')
    if (destination) {
      const directory = await invoke<{ is_dir: boolean }>('get_file_info', { path: destination })
      if (!directory.is_dir) throw new Error('输出位置不是目录')
    }
    if (disposed || generation !== request) return
    const common = { id: `history-${crypto.randomUUID()}`, retryOfTaskId: recordId, path, name: info.name }
    if (kind === 'image') {
      store.imageItems.push({ ...common, inputSize: info.size, inputFormat: inferImageFormat(info.name),
        status: 'inspecting', progress: 0, expanded: true,
        settings: { ...(config?.kind === 'image-compression' ? config.settings : store.imageGlobalSettings), outputDirectory: destination } })
    } else if (kind === 'video') {
      store.videoItems.push({ ...common, inputSize: info.size, outputDirectory: destination,
        status: 'planning', expanded: true, planRevision: 1,
        settings: { ...(config?.kind === 'video-compression' ? config.settings : store.videoGlobalSettings) } })
    } else if (kind === 'pdf') {
      pdf.items.push({ ...common, outputDirectory: destination, status: 'analyzing', report: null,
        mode: config?.kind === 'pdf-optimization' ? config.mode : 'lossless-organization',
        allowLargerOutput: config?.kind === 'pdf-optimization' ? config.allowLargerOutput : false,
        password: '', riskConfirmed: false, frozen: false, taskId: null, error: '' })
    }
  } catch (cause) {
    if (!disposed && generation === request) error.value = `无法创建草稿：${String(cause)}`
  } finally { busy.value = false }
}
defineExpose({ createDraft, created, error })
</script>

<template>
  <section class="detail-section" data-testid="history-media-draft">
    <h3>再次处理{{ record.workloadKind === 'image' ? '图片' : record.workloadKind === 'video' ? '视频' : ' PDF' }}</h3>
    <p class="text-xs text-muted my-3" data-testid="media-restore-notice">{{ restored ? '已保存上次参数，将带入对应工作区供你检查和修改。' : '没有完整的上次配置，将使用当前默认值，请在工作区重新设置。' }}</p>
    <p class="text-xs text-muted mb-3">创建的是独立草稿，不自动执行，也不更改其他文件的设置。来源将重新分析；PDF 有损确认需要重新勾选。</p>
    <fieldset :disabled="busy || created" class="min-w-0 space-y-3">
      <label class="block text-sm">来源文件<input v-model="source" class="retry-path w-full" data-testid="media-retry-source" /></label>
      <label class="block text-sm">本次输出目录（留空使用源目录）<input v-model="output" class="retry-path w-full" data-testid="media-retry-output" /></label>
      <button class="retry-path w-full" data-testid="media-retry-create" @click="createDraft">{{ busy ? '正在检查来源…' : '创建待确认草稿' }}</button>
    </fieldset>
    <p v-if="error" role="alert" class="text-sm text-red-500 mt-2 break-words">{{ error }}</p>
    <p v-if="created" role="status" class="text-sm mt-3">草稿已创建。<a :href="`#/special-compression?mode=${record.workloadKind}`" class="text-primary underline">前往对应工作区确认并启动</a></p>
  </section>
</template>
<style scoped>
.retry-path { min-width: 0; border: 1px solid var(--border-subtle); border-radius: .75rem; padding: .6rem .8rem; color: var(--text-base); background: var(--bg-card); }
fieldset:disabled { opacity: .65; }
</style>
