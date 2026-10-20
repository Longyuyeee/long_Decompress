<script setup lang="ts">
import { useRouter } from 'vue-router'
import { usePendingDraftStore } from '@/stores/pendingDrafts'
import { useArchiveConversionStore } from '@/stores/archiveConversion'
const store = usePendingDraftStore(), router = useRouter()
async function restore() {
  const saved = store.recovery
  await store.restore()
  if (store.recovery || !saved) return
  if (saved.compression.files.length || saved.compression.groups.length) void router.push('/compress')
  else if (saved.extraction.items.length) void router.push('/decompress')
  else {
    void router.push('/compress')
    const conversion = useArchiveConversionStore()
    if (conversion.pending.length && conversion.pending.every(item => item.intent === 'zpaq-update')) conversion.showUpdate()
    else conversion.show()
  }
}
</script>
<template>
  <div v-if="store.count || store.feedback || store.saveError" class="shrink-0 border-b border-primary/15 bg-primary/5 px-4 py-2.5" data-testid="pending-draft-recovery">
    <div class="flex min-w-0 flex-wrap items-center gap-2 text-xs">
      <i class="pi pi-history shrink-0 text-primary" aria-hidden="true"></i>
      <span class="min-w-0 flex-1 text-content">{{ store.saveError || (store.count ? '上次还有 ' + store.count + ' 项待处理任务' : store.feedback) }}</span>
      <template v-if="store.count"><button type="button" class="rounded-lg bg-primary px-3 py-1.5 font-semibold text-white disabled:opacity-50" :disabled="store.busy" @click="restore">{{ store.busy ? '正在恢复…' : '恢复任务' }}</button><button type="button" class="rounded-lg px-2 py-1.5 text-muted hover:bg-input disabled:opacity-50" :disabled="store.busy" @click="store.discard">丢弃上次草稿</button></template>
      <button v-else-if="store.feedback" type="button" class="text-muted" aria-label="关闭恢复提示" @click="store.feedback = ''"><i class="pi pi-times" aria-hidden="true"></i></button>
    </div>
  </div>
</template>
