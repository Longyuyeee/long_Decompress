<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import type { Task } from '@/stores/task'
import {
  compressionLogSeverityClass,
  compressionStageTranslationKey,
  compressionStatusClass,
  emptyCompressionLogTranslationKey,
} from '@/utils/compressionTaskPresentation'

defineProps<{ task?: Task }>()
const appStore = useAppStore()
</script>

<template>
  <div
    data-testid="compression-draft-execution"
    class="pending-execution-panel compression-execution-panel"
  >
    <div class="grid grid-cols-2 gap-2">
      <div class="pending-stat-card">
        <span class="text-muted">{{ appStore.t('progress.stage') }}</span>
        <div class="font-black mt-0.5" :class="compressionStatusClass(task?.status)">
          {{ appStore.t(compressionStageTranslationKey(task)) }}
        </div>
      </div>
      <div class="pending-stat-card">
        <span class="text-muted">{{ appStore.t('progress.percent') }}</span>
        <div class="font-mono font-black text-primary mt-0.5">
          {{ task?.progress || 0 }}%
          <span v-if="task?.speed" class="ml-2 text-muted">{{ task.speed }}</span>
        </div>
      </div>
    </div>
    <div
      v-if="task?.currentFile"
      class="mt-2 rounded-lg bg-input/40 border border-subtle/40 px-3 py-2 truncate font-mono text-xs text-content"
      :title="task.currentFile"
    >
      {{ task.currentFile }}
    </div>
    <h4 class="detail-heading mt-5">
      <i class="pi pi-align-left text-xs"></i>
      {{ appStore.t('decompress.config.logs_title') }}
    </h4>
    <div class="pending-log custom-scrollbar overflow-y-auto space-y-1.5">
      <div
        v-for="(log, index) in task?.logs || []"
        :key="`${log.timestamp}-${index}`"
        class="flex gap-3 items-start border-l-2 border-subtle/20 pl-3 py-0.5"
      >
        <span class="text-dim font-mono text-xs shrink-0">
          {{ new Date(log.timestamp).toLocaleTimeString([], { hour12: false }) }}
        </span>
        <span
          class="font-mono text-xs leading-relaxed"
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
  min-width: 0;
  display: flex;
  flex-direction: column;
  border: 0;
  border-radius: 0;
  background: color-mix(in srgb, var(--bg-input) 18%, transparent);
}

.compression-execution-panel {
  min-height: 17rem;
  max-height: 26rem;
  padding: 1.25rem;
}

.pending-stat-card {
  padding: 0.625rem 0.75rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 70%, transparent);
  border-radius: 0.625rem;
  background: color-mix(in srgb, var(--bg-input) 55%, transparent);
  font-size: 0.75rem;
}

.pending-log {
  flex: 1;
  min-height: 8rem;
  max-height: 16rem;
  padding: 0.75rem;
  border-left: 2px solid color-mix(in srgb, var(--dynamic-accent) 28%, transparent);
  border-radius: 0.5rem;
  background: color-mix(in srgb, var(--bg-input) 35%, transparent);
  font-size: 0.75rem;
}
</style>
