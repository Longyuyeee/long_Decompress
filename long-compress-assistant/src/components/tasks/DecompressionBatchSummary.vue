<script setup lang="ts">
import { useDecompressionBatchStore } from '@/stores/decompressionBatch'
import { useAppStore } from '@/stores/app'

const batch = useDecompressionBatchStore()
const app = useAppStore()
const emit = defineEmits<{ 'show-failures': []; 'show-all': []; 'retry-failures': [] }>()
</script>

<template>
  <section v-if="batch.counts.failed || batch.showingFailures" data-testid="decompression-batch-summary"
    class="shrink-0 border-b border-subtle bg-input/20 px-3 py-2 text-xs">
    <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
      <button v-if="batch.counts.failed && !batch.showingFailures" type="button" data-testid="show-batch-failures"
        class="rounded-lg border border-subtle px-3 py-1.5 text-primary hover:bg-primary/10" @click="emit('show-failures')">
        {{ app.t('decompress.batch.show_failures') }}
      </button>
      <button v-if="batch.showingFailures" type="button" data-testid="show-all-decompression-tasks"
        class="rounded-lg border border-subtle px-3 py-1.5 text-primary hover:bg-primary/10" @click="emit('show-all')">
        {{ app.t('decompress.batch.show_all') }}
      </button>
      <button v-if="batch.visibleFailureIds.length" type="button" data-testid="retry-batch-failures"
        class="rounded-lg border border-subtle px-3 py-1.5 text-primary hover:bg-primary/10" @click="emit('retry-failures')">
        {{ app.t('decompress.batch.retry_failures') }}
      </button>
    </div>
    <p v-if="batch.showingFailures" class="mt-2 text-muted">{{ app.t('decompress.batch.filtered') }}</p>
  </section>
</template>
