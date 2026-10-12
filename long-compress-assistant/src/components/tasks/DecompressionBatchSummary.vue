<script setup lang="ts">
import { ref, watch } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import CompletedExtractionOutput from './CompletedExtractionOutput.vue'
import BatchResultActions from './BatchResultActions.vue'
import { useDecompressionBatchStore } from '@/stores/decompressionBatch'
import { useAppStore } from '@/stores/app'

const batch = useDecompressionBatchStore()
const app = useAppStore()
const showResults = ref(false)
watch(() => batch.sequence, () => { showResults.value = false })
const emit = defineEmits<{ 'show-failures': []; 'show-all': []; 'retry-failures': [] }>()
</script>

<template>
  <section v-if="batch.counts.total" data-testid="decompression-batch-summary"
    class="shrink-0 border-b border-subtle bg-input/20 px-3 py-2 text-xs">
    <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
      <strong class="text-content">{{ app.t('decompress.batch.title').replace('{0}', String(batch.sequence)).replace('{1}', String(batch.counts.total)) }}</strong>
      <span class="text-muted" role="status" aria-live="polite" aria-atomic="true">
        {{ app.t('decompress.batch.counts').replace('{0}', String(batch.counts.completed)).replace('{1}', String(batch.counts.failed)).replace('{2}', String(batch.counts.cancelled)).replace('{3}', String(batch.counts.unfinished)) }}
      </span>
      <span v-if="batch.counts.conflicts" class="text-amber-500">{{ app.t('decompress.batch.conflicts').replace('{0}', String(batch.counts.conflicts)) }}</span>
      <span v-if="batch.counts.removed" class="text-muted">{{ app.t('decompress.batch.removed').replace('{0}', String(batch.counts.removed)) }}</span>
      <button v-if="batch.counts.completed" type="button" data-testid="show-batch-results"
        class="rounded-lg border border-subtle px-3 py-1.5 text-primary hover:bg-primary/10" @click="showResults = true">
        <i class="pi pi-folder-open mr-1.5" aria-hidden="true"></i>查看成功结果
      </button>
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
  <Modal v-model:visible="showResults" title="本批成功结果" size="lg" cancel-text="关闭成功结果">
    <div class="space-y-3" data-testid="batch-success-results">
      <p class="text-sm text-muted">已完成 {{ batch.completedResults.length }} 项，选择需要查看的输出目录。</p>
      <BatchResultActions :locations="batch.completedResults.map(result => ({ path: result.outputPath, directory: true }))" />
      <CompletedExtractionOutput v-for="result in batch.completedResults" :key="result.id"
        :title="result.name" :path="result.outputPath" />
    </div>
  </Modal>
</template>
