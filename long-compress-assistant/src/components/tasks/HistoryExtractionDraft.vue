<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { open } from '@tauri-apps/api/dialog'
import type { TaskHistoryRecord } from '@/types/taskHistory'
import { useTaskStore } from '@/stores/task'
import { generateId, extractErrorMessage } from '@/utils'
import { isDecompressArchivePath, isPotentialSplitArchivePath } from '@/utils/compressionFormat'

const props = defineProps<{ record: TaskHistoryRecord }>()
const store = useTaskStore()
const source = ref('')
const destination = ref('')
const busy = ref(false)
const error = ref('')
const created = ref(false)
let disposed = false
onUnmounted(() => { disposed = true })
watch(() => props.record.id, () => {
  source.value = props.record.sourcePaths[0] || ''
  destination.value = ''
  error.value = ''
  created.value = false
}, { immediate: true })

async function selectPath(directory: boolean) {
  const recordId = props.record.id
  try {
    const queued = import.meta.env.VITE_DESKTOP_E2E === '1'
      ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection()
      : undefined
    const path = queued !== undefined ? queued
      : await open({ directory, multiple: false, title: directory ? '选择本次输出目录' : '选择归档或首个分卷' })
    if (!disposed && props.record.id === recordId && typeof path === 'string') {
      if (directory) destination.value = path
      else source.value = path
    }
  } catch (cause) { error.value = extractErrorMessage(cause) }
}

async function createDraft() {
  if (busy.value || created.value) return
  error.value = ''
  let path = source.value.trim()
  const outputPath = destination.value.trim()
  const recordId = props.record.id
  if (!path || !outputPath) { error.value = '请确认来源文件并选择本次输出目录'; return }
  if (!isDecompressArchivePath(path) && !isPotentialSplitArchivePath(path)) {
    error.value = '请选择支持的归档文件或首个分卷'
    return
  }
  busy.value = true
  try {
    const split = await invoke<{ is_split: boolean; first_part?: string; is_complete?: boolean; missing_parts?: string[] }>('detect_split_archive', { path })
    if (split?.is_split) {
      if (!split.first_part || split.is_complete === false || split.missing_parts?.length) {
        throw new Error(`分卷不完整，请补齐后重试：${split.missing_parts?.join('、') || '缺少首卷或必要分卷'}`)
      }
      path = split.first_part
    } else if (!isDecompressArchivePath(path)) {
      throw new Error('无法确认分卷组，请选择归档首卷')
    }
    const file = await invoke<{ name: string; is_dir: boolean; size: number }>('get_file_info', { path })
    if (file.is_dir || file.size === 0) throw new Error('来源必须是非空归档文件；请确认下载完成')
    // A selection changed during the asynchronous check must not create the old draft.
    if (disposed || props.record.id !== recordId) return
    const id = store.addTask({
      id: generateId(), name: file.name, type: 'decompression', workloadKind: 'archive',
      sourceFiles: [path], outputPath, configurationMode: 'individual',
      extractToSubfolder: true, recycleSourceAfterExtract: false,
    })
    store.tasks.find(task => task.id === id)!.logs.push({
      task_id: id, severity: 'info', timestamp: new Date().toISOString(),
      message: `从历史任务 ${recordId} 创建完整解压草稿；原密码、筛选范围和配置未恢复，请确认本次设置。`,
    })
    created.value = true
  } catch (cause) {
    if (props.record.id === recordId) error.value = `无法创建草稿：${extractErrorMessage(cause)}`
  } finally { busy.value = false }
}
</script>

<template>
  <section class="detail-section" data-testid="history-extraction-draft">
    <h3>创建完整解压草稿</h3>
    <p class="text-xs text-muted my-3">历史未保存完整配置，此操作不恢复密码或原选择范围。新任务将完整解压到同名子目录、保留源文件；不会自动开始。执行时仍使用当前冲突策略和密码保险箱设置。</p>
    <p class="text-xs text-muted mb-3">创建前会检查当前来源是否存在且非空；旧记录没有内容指纹，无法确认文件是否与当时相同。</p>
    <p v-if="record.sourcePaths.length > 1" class="text-xs text-amber-500 mb-2">原记录包含多个来源，请确认此处为归档或首个分卷；本次只创建一个任务。</p>
    <fieldset :disabled="busy || created" class="space-y-2 min-w-0">
      <label class="block text-sm">来源文件
        <input v-model="source" data-testid="history-draft-source" class="history-control w-full mt-1" />
      </label>
      <button class="history-control" @click="selectPath(false)">重新选择文件</button>
      <label class="block text-sm">本次输出目录
        <input v-model="destination" readonly data-testid="history-draft-output" class="history-control w-full mt-1" placeholder="请选择输出目录" />
      </label>
      <button data-testid="history-draft-select-output" class="history-control" @click="selectPath(true)">选择输出目录</button>
      <button data-testid="history-draft-create" class="history-control w-full" @click="createDraft">{{ busy ? '正在检查来源…' : '创建待执行任务' }}</button>
    </fieldset>
    <p v-if="error" role="alert" class="text-sm text-red-500 mt-2 break-words">{{ error }}</p>
    <p v-if="created" role="status" class="text-sm mt-3">草稿已创建。<a href="#/decompress" class="text-primary underline">前往解压中心确认并启动</a></p>
  </section>
</template>

<style scoped>
.history-control { border: 1px solid var(--border-subtle); border-radius: .75rem; padding: .6rem .8rem; color: var(--text-base); background: var(--bg-card); min-width: 0; }
fieldset:disabled { opacity: .65; }
button:not(:disabled):hover { border-color: var(--dynamic-accent); }
</style>
