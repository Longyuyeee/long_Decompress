<script setup lang="ts">
import ResultActions from './ResultActions.vue'
withDefaults(defineProps<{ path: string; retrieval?: import('@/types/zpaqRetrieval').ZpaqRetrievalSelection; title?: string }>(), { title: '解压结果' })
</script>

<template>
  <section class="min-w-0 space-y-3 rounded-xl border border-subtle bg-input/30 p-4" data-testid="completed-extraction-output">
    <div class="flex min-w-0 items-start gap-3">
      <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary"><i class="pi pi-folder-open" aria-hidden="true"></i></span>
      <div class="min-w-0 flex-1">
        <p class="break-words text-sm font-bold text-content [overflow-wrap:anywhere]">{{ retrieval&&title==='解压结果'?'所选文件部分取回':title }}</p>
        <p class="mt-1 select-text break-words text-xs leading-relaxed text-muted [overflow-wrap:anywhere]">{{ path || '未记录输出目录' }}</p>
      </div>
    </div>
    <ResultActions :path="path" :retrieval="retrieval" directory />
  </section>
</template>
