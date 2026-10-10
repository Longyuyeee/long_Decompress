<script setup lang="ts">
import { onUnmounted, ref, watch } from 'vue'
import type { TaskHistoryRecord } from '@/types/taskHistory'
import HistoryCompressionDraft from './HistoryCompressionDraft.vue'
import HistoryMediaDraft from './HistoryMediaDraft.vue'

const props = defineProps<{ records: TaskHistoryRecord[] }>()
const emit = defineEmits<{ busy: [value: boolean] }>()
interface Draft { createDraft(): Promise<void>; created: boolean; error: string }
const drafts = new Map<string, Draft>()
const running = ref(false)
const result = ref('')
watch(() => props.records.map(record => record.id).join('|'), () => { result.value = '' })
let disposed = false
onUnmounted(() => { disposed = true; emit('busy', false) })
function register(id: string, instance: unknown) {
  if (instance) drafts.set(id, instance as Draft)
  else drafts.delete(id)
}
async function createSelected() {
  if (running.value) return
  const selected = props.records.map(record => record.id)
  running.value = true
  emit('busy', true)
  result.value = ''
  let ready = 0
  try {
    for (const id of selected) {
      if (disposed) return
      const draft = drafts.get(id)
      if (!draft) continue
      await draft.createDraft()
      if (draft.created) ready++
    }
    result.value = `${ready} 项草稿已就绪，${selected.length - ready} 项需要检查或修改设置。未自动启动任何任务；请展开对应项查看原因。`
  } finally { running.value = false; emit('busy', false) }
}
</script>
<template>
  <section class="history-panel p-4 space-y-3" data-testid="history-retry-batch">
    <h3 class="font-bold">确认所选 {{ records.length }} 项的再次执行设置</h3>
    <p class="text-xs text-muted">只处理下方勾选项。可逐项修改来源、参数或输出目录；某项检查失败不会把其他未选任务加入队列。</p>
    <details v-for="record in records" :key="record.id" class="border border-subtle rounded-xl p-3">
      <summary class="cursor-pointer font-bold text-sm">{{ record.name }}</summary>
      <HistoryCompressionDraft v-if="!record.workloadKind || record.workloadKind === 'archive'" :ref="instance => register(record.id, instance)" :record="record" />
      <HistoryMediaDraft v-else :ref="instance => register(record.id, instance)" :record="record" />
    </details>
    <button class="history-control" :disabled="running || !records.length" data-testid="history-batch-create" @click="createSelected">{{ running ? '正在逐项检查…' : '仅为这些所选项创建草稿' }}</button>
    <p v-if="result" role="status" class="text-sm" data-testid="history-batch-result">{{ result }}</p>
  </section>
</template>
