<script setup lang="ts">
import { ref } from 'vue'
import ResultActions from '@/components/tasks/ResultActions.vue'
import { formatFileSize } from '@/utils'
import type { PublishedArchiveOutput } from '@/types/archiveOutput'
defineProps<{ output: PublishedArchiveOutput }>()
const emit = defineEmits<{ checksum: [] }>()
const expanded = ref(false)
</script>
<template>
  <section class="min-w-0 space-y-3 rounded-xl border border-subtle bg-input/30 p-3 mb-3" data-testid="archive-output-summary">
    <p v-if="output.updateSummary" data-testid="zpaq-update-summary" class="text-xs leading-relaxed text-muted"><template v-if="output.updateSummary.unchanged">所选内容和时间未变化，已另存副本，未新增快照。</template><template v-else>已添加/更新 {{ output.updateSummary.updatedEntries }} 项；历史快照 {{ output.updateSummary.baseSnapshots }} → {{ output.updateSummary.snapshots }}，未选旧内容保留。</template></p>
    <ResultActions :path="output.primaryPath" :split="output.split" :output="output" show-path @checksum="emit('checksum')" />
    <button class="flex w-full items-center justify-between gap-2 rounded-lg px-1 py-1 text-xs text-primary text-left" type="button" :aria-expanded="expanded" @click="expanded = !expanded">
      <span>查看产物 · {{ output.files.length }} {{ output.split ? '卷' : '个文件' }} · {{ formatFileSize(output.totalBytes) }}</span><i :class="expanded ? 'pi pi-chevron-up' : 'pi pi-chevron-down'" aria-hidden="true"></i>
    </button>
    <ol v-if="expanded" class="max-h-52 overflow-y-auto custom-scrollbar space-y-2 text-xs">
      <li v-for="file in output.files" :key="file.path" class="border-t border-subtle pt-2">
        <code class="block break-all select-text">{{ file.path }}</code>
        <span class="text-muted">{{ formatFileSize(file.bytes) }}</span>
      </li>
    </ol>
  </section>
</template>
