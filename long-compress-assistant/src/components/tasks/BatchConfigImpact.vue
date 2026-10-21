<script setup lang="ts">
import type { BatchSubject } from '@/utils/batchConfigSummary'
defineProps<{ count: number; subjects?: BatchSubject[]; changes: string[] }>()
</script>
<template>
  <section class="min-w-0 space-y-2 rounded-xl border border-primary/20 bg-primary/5 p-3 text-xs" data-testid="batch-config-impact">
    <div class="flex flex-wrap items-center justify-between gap-2"><strong class="text-content">本次修改 · {{ count }} 项待处理任务</strong><span class="text-muted">未指定项目保持各自设置</span></div>
    <ul v-if="changes.length" class="flex min-w-0 flex-wrap gap-1.5" data-testid="batch-change-summary"><li v-for="change in changes" :key="change" class="max-w-full select-text break-words rounded-lg bg-input/60 px-2 py-1.5 text-content [overflow-wrap:anywhere]">{{ change }}</li></ul>
    <p v-else class="text-muted">尚未指定修改项，不会改变任务配置。</p>
    <details v-if="subjects?.length" class="min-w-0 text-muted" data-testid="batch-selected-subjects"><summary class="cursor-pointer py-1 text-primary">查看所选任务（{{ subjects.length }}）</summary><ol class="mt-2 max-h-40 space-y-2 overflow-y-auto custom-scrollbar"><li v-for="(subject,index) in subjects" :key="index" class="min-w-0 border-t border-subtle pt-2"><strong class="block break-words font-medium text-content [overflow-wrap:anywhere]">{{ subject.name }}</strong><span v-if="subject.path" class="block select-text break-words leading-relaxed [overflow-wrap:anywhere]">{{ subject.path }}</span></li></ol></details>
  </section>
</template>
