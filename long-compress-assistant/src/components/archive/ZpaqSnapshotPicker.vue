<script setup lang="ts">
import { computed } from 'vue'
import { snapshotDate, type ZpaqHistory, type ZpaqSnapshot } from '@/types/zpaqSnapshot'
const props = defineProps<{ history: ZpaqHistory; disabled?: boolean }>()
const emit = defineEmits<{ change: [snapshot: ZpaqSnapshot] }>()
const selected = computed(() => props.history.snapshots.find(s => s.version === props.history.selected.version))
function change(event: Event) {
  const snapshot = props.history.snapshots.find(s => s.version === Number((event.target as HTMLSelectElement).value))
  if (snapshot) emit('change', { version: snapshot.version, date: snapshot.date })
}
</script>
<template>
  <section class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2 rounded-xl border border-subtle bg-input/40 px-3 py-2" data-testid="zpaq-snapshot-picker">
    <label class="flex min-w-0 basis-full grow flex-wrap items-center gap-2 text-xs font-semibold text-content lg:basis-96">
      <i class="pi pi-history text-primary" aria-hidden="true"></i><span>历史快照</span>
      <select data-testid="zpaq-snapshot-select" :value="history.selected.version" :disabled="disabled" class="min-w-0 max-w-full flex-1 rounded-lg border border-subtle bg-card px-2 py-2 text-content outline-none focus:border-primary" @change="change">
        <option v-for="snapshot in [...history.snapshots].reverse()" :key="snapshot.version" :value="snapshot.version">{{ snapshot.version === history.snapshots.at(-1)?.version ? '最新 · ' : '' }}第 {{ snapshot.version }} 次更新 · {{ snapshotDate(snapshot.date) }}</option>
      </select>
    </label>
    <span v-if="selected" class="text-xs text-muted">{{ snapshotDate(selected.date) }} · {{ selected.updates }} 项更新 · {{ selected.deletes }} 项删除</span>
    <span class="text-xs text-muted">原包不改动</span>
    <slot />
  </section>
</template>
