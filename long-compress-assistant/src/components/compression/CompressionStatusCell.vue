<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import type { Task } from '@/stores/task'
import {
  compressionStatusClass,
  compressionStatusIcon,
  compressionStatusTranslationKey,
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
    class="flex-1 min-w-[160px] flex items-center gap-3"
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
        {{ appStore.t(compressionStatusTranslationKey(status)) }}
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
      {{ progress }}%
    </span>
  </div>
</template>
