<script setup lang="ts">
import { prepareCompressionRetryDraft } from '@/utils/compressionRetry'
import { useCompressionStore } from '@/stores/compression'
import TaskFailureHint from '@/components/tasks/TaskFailureHint.vue'
import { computed, ref, nextTick } from 'vue'
import { useAppStore } from '@/stores/app'
import type { Task } from '@/stores/task'
import { formatFileSize } from '@/utils'
import ArchiveOutputSummary from './ArchiveOutputSummary.vue'
import ResultActions from '@/components/tasks/ResultActions.vue'
import TaskExecutionOverview from '@/components/tasks/TaskExecutionOverview.vue'
import {
  compressionLogSeverityClass,
  compressionStageTranslationKey,
  emptyCompressionLogTranslationKey,
} from '@/utils/compressionTaskPresentation'

const props = defineProps<{ task?: Task }>()
const emit = defineEmits<{ 'retry-prepared': [] }>()
const appStore = useAppStore()
const compression = useCompressionStore()
const retryBusy = ref(false), retryError = ref('')
const retryCreated = computed(()=>compression.groups.some(group=>group.retryOfTaskId===props.task?.id))
async function prepareRetry() {
  if (!props.task || retryBusy.value) return
  retryBusy.value = true; retryError.value = ''
  try {
    await prepareCompressionRetryDraft(props.task)
    emit('retry-prepared')
    await nextTick()
    Array.from(document.querySelectorAll<HTMLElement>('[data-retry-of]')).find(row=>row.dataset.retryOf===props.task?.id)?.scrollIntoView?.({block:'start',behavior:'smooth'})
  }
  catch(error) { retryError.value=String(error) }
  finally { retryBusy.value=false }
}
const sourceBytes = computed(() => props.task?.compressionSourceProgress?.processedBytes ?? props.task?.processedBytes ?? 0)
const pendingSplitResult = computed(() => Boolean(props.task?.compressionSourceProgress && props.task.status !== 'completed'))

const liveRatio = computed(() => {
  if (pendingSplitResult.value) return undefined
  const input = sourceBytes.value
  const output = props.task?.outputBytes || 0
  return input > 0 && output > 0 ? output / input * 100 : undefined
})

const savedRatio = computed(() => liveRatio.value === undefined ? undefined : 100 - liveRatio.value)
</script>

<template>
  <div
    data-testid="compression-draft-execution"
    class="pending-execution-panel compression-execution-panel custom-scrollbar"
  >
    <TaskFailureHint :task="task">
      <button v-if="task?.executionConfig?.kind==='archive-compression'" type="button" data-testid="compression-prepare-retry" class="retry-action" :disabled="retryBusy || retryCreated" @click="prepareRetry">{{ retryCreated ? '已创建待处理项，请在配置区确认并启动' : retryBusy ? '正在带入配置…' : '带入配置，创建待处理项' }}</button>
      <p v-if="retryError" role="alert" class="text-red-500">{{ retryError }}</p>
    </TaskFailureHint>
    <ArchiveOutputSummary v-if="task?.status === 'completed' && task.archiveOutput" :output="task.archiveOutput" />
    <ResultActions v-else-if="task?.status === 'completed' && task.outputPath" :path="task.outputPath" show-path class="mb-3" />
    <TaskExecutionOverview
      :stage="appStore.t(compressionStageTranslationKey(task))"
      :status="task?.status" :progress="task?.progress" :speed="task?.speed" :current-file="task?.currentFile"
    >
      <dl v-if="task?.processedBytes !== undefined || task?.outputBytes !== undefined" class="execution-metrics" data-testid="compression-live-metrics">
        <div class="execution-metric"><dt>已读取源数据</dt><dd>{{ formatFileSize(sourceBytes) }}</dd></div>
        <div class="execution-metric"><dt>{{ task?.compressionSourceProgress ? '已写分卷' : '当前压缩包大小' }}</dt><dd class="text-primary">{{ formatFileSize(task?.outputBytes || 0) }}</dd></div>
        <div class="execution-metric"><dt>实时压缩比</dt><dd>{{ liveRatio === undefined ? '计算中' : `${liveRatio.toFixed(2)}%` }}</dd></div>
        <div class="execution-metric"><dt>空间变化</dt>
          <dd :class="savedRatio !== undefined && savedRatio >= 0 ? 'text-green-500' : 'text-amber-500'">
            <template v-if="pendingSplitResult">{{ task?.status === 'failed' || task?.status === 'cancelled' ? '未生成有效结果' : '完成后统计' }}</template>
            <template v-else-if="savedRatio === undefined">计算中</template>
            <template v-else-if="savedRatio >= 0">节省 {{ savedRatio.toFixed(2) }}%</template>
            <template v-else>增加 {{ Math.abs(savedRatio).toFixed(2) }}%</template>
          </dd>
        </div>
      </dl>
    </TaskExecutionOverview>
    <h4 class="detail-heading execution-log-heading">
      <i class="pi pi-align-left text-xs"></i>
      {{ appStore.t('decompress.config.logs_title') }}
    </h4>
    <div data-testid="compression-log-viewport" class="pending-log custom-scrollbar overflow-y-auto overflow-x-hidden space-y-1.5">
      <div
        v-for="(log, index) in task?.logs || []"
        :key="`${log.timestamp}-${index}`"
        class="pending-log-row flex min-w-0 gap-3 items-start border-l-2 border-subtle/20 pl-3 py-0.5"
      >
        <span class="pending-log-time text-dim font-mono text-xs shrink-0">
          {{ new Date(log.timestamp).toLocaleTimeString([], { hour12: false }) }}
        </span>
        <span
          class="min-w-0 flex-1 break-words [overflow-wrap:anywhere] font-mono text-xs leading-relaxed"
          :class="compressionLogSeverityClass(log.severity)"
        >
          {{ log.message }}
        </span>
      </div>
      <span v-if="!task?.logs.length" class="font-mono text-dim">
        {{ appStore.t(emptyCompressionLogTranslationKey(task)) }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.retry-action { align-self:flex-start; border:1px solid var(--border-subtle); color:var(--dynamic-accent); border-radius:.6rem; padding:.45rem .65rem; font-weight:600; }
.retry-action:disabled { opacity:.6; cursor:default; }
.detail-heading {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
  color: var(--dynamic-accent);
  font-size: 0.75rem;
  font-weight: 900;
  letter-spacing: 0.16em;
}

.pending-execution-panel {
  width: 100%;
  box-sizing: border-box;
  min-width: 0;
  max-width: 100%;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  border: 0;
  border-radius: 0;
  background: color-mix(in srgb, var(--bg-input) 18%, transparent);
}

.compression-execution-panel {
  container: compression-execution / inline-size;
  height: 100%;
  min-height: 0;
  max-height: none;
  overflow-y: auto;
  padding: 1rem;
  scrollbar-gutter: stable;
  overscroll-behavior: auto;
}

.compression-execution-panel > * {
  flex-shrink: 0;
}

.compression-execution-panel :deep([data-testid="result-actions"] > p:not([role="alert"])) {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow: hidden;
}

.execution-log-heading {
  position: sticky;
  top: 0;
  z-index: 1;
  margin: .875rem 0 .625rem;
  padding: .375rem 0;
  background: var(--bg-card);
}

.pending-log {
  width: 100%;
  box-sizing: border-box;
  min-width: 0;
  max-width: 100%;
  overflow-x: hidden;
  /* Keep logs usable when result cards and live metrics exceed the pane height. */
  flex: 1 0 12rem;
  min-height: 12rem;
  max-height: none;
  padding: 0.75rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 55%, transparent);
  border-radius: .875rem;
  background: color-mix(in srgb, var(--bg-input) 35%, transparent);
  font-size: 0.75rem;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
}

.pending-log-row {
  width: 100%;
  box-sizing: border-box;
  max-width: 100%;
}

.pending-log-row > :last-child {
  white-space: pre-wrap;
  word-break: break-word;
  font-size: .8125rem;
  line-height: 1.65;
}

@container compression-execution (max-width: 20rem) {
  .pending-log-row {
    flex-direction: column;
    gap: 0.25rem;
  }
}

@media (max-width: 520px) {
  .compression-execution-panel {
    padding: 1rem;
  }

  .pending-execution-panel > .grid {
    grid-template-columns: minmax(0, 1fr);
  }

  .pending-log-row {
    flex-direction: column;
    gap: 0.25rem;
  }

  .pending-log-time {
    min-width: 0;
    max-width: 100%;
    flex-shrink: 1;
    overflow-wrap: anywhere;
  }
}
</style>
