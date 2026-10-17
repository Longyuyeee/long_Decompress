<script setup lang="ts">
import { computed } from 'vue'
import type { ArchiveRecoveryEntryResult } from '@/composables/useTauriCommands'
const props = defineProps<{ recoveredEntries?: string[]; skippedEntries: string[]; entryResults?: ArchiveRecoveryEntryResult[] }>()
const groups = computed(() => ([
  { status: 'recovered', label: '已恢复', icon: 'pi-check-circle' },
  { status: 'missing', label: '涉及缺卷', icon: 'pi-folder-open' },
  { status: 'checksum_failed', label: '校验失败', icon: 'pi-exclamation-circle' },
  { status: 'unknown', label: '无法确认', icon: 'pi-question-circle' },
].map(group => ({ ...group, entries: props.entryResults?.filter(entry => entry.status === group.status) || [] })).filter(group => group.entries.length)))
</script>

<template>
  <div class="recovery-details">
    <template v-if="entryResults?.length">
      <details v-for="group in groups" :key="group.status" :class="{ 'recovery-details-warning': group.status !== 'recovered' }" :data-testid="`recovery-outcome-${group.status}`">
        <summary><i class="pi" :class="group.icon"></i>{{ group.label }} <span>{{ group.entries.length }}</span></summary>
        <ul><li v-for="(entry, index) in group.entries" :key="index"><strong>{{ entry.name || '后续内容（文件清单未知）' }}</strong><small v-if="entry.status !== 'recovered'">{{ entry.reason }}</small></li></ul>
      </details>
    </template>
    <details v-if="!entryResults?.length && recoveredEntries?.length" data-testid="recovery-file-details">
      <summary><i class="pi pi-check-circle"></i>查看已恢复文件 <span>{{ recoveredEntries.length }}</span></summary>
      <ul><li v-for="(entry, index) in recoveredEntries" :key="index">{{ entry }}</li></ul>
    </details>
    <details v-if="!entryResults?.length && skippedEntries.length" class="recovery-details-warning" data-testid="recovery-skipped-details">
      <summary><i class="pi pi-info-circle"></i>未恢复原因与处理说明 <span>{{ skippedEntries.length }}</span></summary>
      <ul><li v-for="(entry, index) in skippedEntries" :key="index">{{ entry }}</li></ul>
    </details>
  </div>
</template>

<style scoped>
.recovery-details { display: grid; min-width: 0; gap: .5rem; }
details { min-width: 0; border: 1px solid var(--border-subtle); border-radius: .75rem; background: color-mix(in srgb,var(--bg-input) 35%,transparent); }
summary { display: flex; align-items: center; gap: .5rem; padding: .75rem; color: var(--text-content); cursor: pointer; font-size: .75rem; font-weight: 700; overflow-wrap: anywhere; }
summary span { flex: none; margin-left: auto; padding: .1rem .45rem; border-radius: .4rem; background: color-mix(in srgb,var(--dynamic-accent) 12%,transparent); color: var(--dynamic-accent); }
summary i { flex: none; color: var(--dynamic-accent); }
.recovery-details-warning summary i { color: #d97706; }
ul { max-height: 13rem; overflow-y: auto; margin: 0 .75rem .75rem; padding: .15rem .5rem .15rem 1rem; color: var(--text-muted); font-size: .75rem; line-height: 1.7; }
li { padding: .2rem 0; overflow-wrap: anywhere; list-style: disc; }
li strong { font-weight: 600; color: var(--text-content); }
li small { display: block; font-size: inherit; }
summary:focus-visible { outline: 2px solid var(--dynamic-accent); outline-offset: -2px; border-radius: .7rem; }
</style>
