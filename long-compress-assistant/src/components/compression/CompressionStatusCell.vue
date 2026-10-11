<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import type { Task } from '@/stores/task'
import { formatProgressPercent } from '@/utils/progress'
import {
  compressionStatusClass,
  compressionStatusIcon,
  compressionStatusTranslationKey,
  compressionStageTranslationKey,
  showsCompressionProgress,
} from '@/utils/compressionTaskPresentation'

const props = defineProps<{ task?: Task }>()
const appStore = useAppStore()
const status = computed(() => props.task?.status || 'pending')
const progress = computed(() => props.task?.progress || 0)
</script>

<template>
  <div
    data-testid="compression-status-progress"
    class="compression-status-cell flex-1 min-w-0 flex items-center gap-3"
  >
    <div class="flex items-center gap-2 shrink-0">
      <i
        class="pi text-sm"
        :class="[compressionStatusIcon(status), compressionStatusClass(status)]"
      ></i>
      <span
        class="text-xs font-black tracking-wider"
        :class="compressionStatusClass(status)"
      >
        {{ appStore.t(['running', 'compressing'].includes(status) ? compressionStageTranslationKey(task) : compressionStatusTranslationKey(status)) }}
      </span>
    </div>
    <div
      v-if="showsCompressionProgress(status)"
      class="flex-1 h-1.5 bg-input/50 rounded-full overflow-hidden"
    >
      <div
        class="h-full bg-primary transition-all duration-300 rounded-full"
        :style="{ width: `${progress}%` }"
      ></div>
    </div>
    <span
      v-if="showsCompressionProgress(status)"
      class="text-xs font-mono text-primary font-bold"
    >
      {{ formatProgressPercent(progress) }}%
    </span>
  </div>
</template>

<style scoped>
.compression-status-cell {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  column-gap: 0.5rem;
  row-gap: 0.3rem;
}
.compression-status-cell > div:nth-child(2) {
  grid-row: 2;
  grid-column: 1 / -1;
  width: 100%;
}
.compression-status-cell > span {
  grid-row: 1;
  grid-column: 2;
}
@media (max-width: 520px) {
  .compression-status-cell {
    flex-basis: 5.5rem;
    gap: 0.375rem;
  }

  .compression-status-cell > div:nth-child(2) {
    display: none;
  }
}
</style>
