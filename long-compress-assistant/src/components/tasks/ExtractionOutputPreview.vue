<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useAppStore } from '@/stores/app'

const props = defineProps<{ source: string; output: string; subfolder: boolean; compact?: boolean; sharedCount?: number }>()
const emit = defineEmits<{ resolved: [path: string | null]; 'change-output': [] }>()
const app = useAppStore()
const path = ref('')
const error = ref('')
const copied = ref(false)
const loading = ref(false)
const destinationState = ref('')
const destinationMessage = computed(() => ({
  empty: '目录已存在，目前为空', occupied: '目录已有内容', file: '此位置是文件，不能作为输出目录',
  unavailable: '暂时无法读取此目录',
}[destinationState.value] || ''))
const needsAttention = computed(() => ['occupied', 'file', 'unavailable'].includes(destinationState.value) || (props.sharedCount || 0) > 1)
let generation = 0
let timer: ReturnType<typeof setTimeout> | undefined
const conflictLabel = computed(() => ({ ask: '询问如何处理', rename: '自动重命名', skip: '跳过已有文件', overwrite: '覆盖已有文件' }[app.settings.conflictPolicy] || '按当前设置处理'))

watch(() => [props.source, props.output, props.subfolder], () => {
  const request = ++generation
  clearTimeout(timer)
  path.value = ''; error.value = ''; copied.value = false
  destinationState.value = ''
  emit('resolved', null)
  loading.value = !!props.source.trim() && !!props.output.trim()
  if (!loading.value) return
  if (props.subfolder && /\.(?:z|r)\d{2,3}$/i.test(props.source)) {
    loading.value = false
    error.value = '请选择这组分卷的 ZIP 或 RAR 主文件后预览。'
    return
  }
  const args = { filePath: props.source, outputPath: props.output, createSubdirectory: props.subfolder }
  timer = setTimeout(async () => {
    try {
      const result = await invoke<string>('preview_extraction_output', args)
      if (request === generation) {
        if (typeof result !== 'string' || !result) throw new Error('Preview unavailable')
        path.value = result
        emit('resolved', result)
        // Let the path appear immediately while checking the destination off-thread.
        void invoke<string>('inspect_extraction_destination', { path: result }).then(state => {
          if (request === generation) destinationState.value = state
        }).catch(() => { if (request === generation) destinationState.value = 'unavailable' })
      }
    } catch {
      if (request === generation) error.value = '暂时无法预览，请核对来源和输出目录。'
    } finally {
      if (request === generation) loading.value = false
    }
  }, 150)
}, { immediate: true })
onUnmounted(() => { generation++; clearTimeout(timer); emit('resolved', null) })

async function copyPath() {
  const current = path.value
  try {
    await navigator.clipboard.writeText(current)
    if (path.value === current) copied.value = true
  } catch { app.setError('复制失败，可选中下方路径手动复制。') }
}
</script>

<template>
  <div data-testid="extraction-output-preview" class="min-w-0 text-xs" :class="compact ? 'w-full text-muted' : 'rounded-xl border border-primary/20 bg-primary/5 p-3 space-y-2'">
    <div class="flex items-start gap-2 min-w-0">
      <span class="shrink-0" :class="{ 'font-bold text-primary': !compact }">预计解压到</span>
      <span v-if="path" class="min-w-0 flex-1 select-text" :class="compact ? 'truncate' : 'break-all'" :title="path" data-testid="extraction-output-path">{{ path }}</span>
      <span v-else class="text-muted" role="status">{{ loading ? '正在更新…' : error || '选择来源和输出目录后显示' }}</span>
      <button v-if="path" type="button" class="shrink-0 text-primary hover:underline" :aria-label="copied ? '路径已复制' : '复制输出路径'" @click.stop="copyPath" @keydown.stop>{{ copied ? '已复制' : '复制' }}</button>
    </div>
    <p v-if="!compact" class="text-muted">{{ subfolder ? '在输出目录内创建同名子目录。' : '直接解压到此目录，不另建同名子目录。' }}同名文件：{{ conflictLabel }}。归档内的文件夹会保留。</p>
    <div v-if="destinationMessage || (sharedCount || 0) > 1" data-testid="extraction-destination-notice" class="flex flex-wrap items-center gap-x-2 gap-y-1 mt-1" :class="needsAttention ? 'text-amber-600 dark:text-amber-400' : 'text-muted'">
      <span v-if="destinationMessage" :title="destinationState === 'occupied' ? `已有内容不代表一定有同名文件；若有同名文件，将${conflictLabel}。` : undefined">{{ destinationMessage }}</span>
      <span v-if="(sharedCount || 0) > 1" data-testid="extraction-shared-destination" title="这些待执行任务会写入同一目录。如需分开放置，可更换目录或调整同名子目录选项。">{{ sharedCount }} 项任务共用此目录</span>
      <button v-if="needsAttention" type="button" class="text-primary hover:underline" @click.stop="emit('change-output')" @keydown.stop>更换目录</button>
      <p v-if="!compact && destinationState === 'occupied'" class="w-full text-muted">新内容会放入已有目录；是否存在同名文件，将在解压时确认。</p>
    </div>
  </div>
</template>
