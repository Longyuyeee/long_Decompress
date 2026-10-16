<script setup lang="ts">
defineProps<{ recoveredEntries?: string[]; skippedEntries: string[] }>()
</script>

<template>
  <div class="recovery-details">
    <details v-if="recoveredEntries?.length" data-testid="recovery-file-details">
      <summary><i class="pi pi-check-circle"></i>查看已恢复文件 <span>{{ recoveredEntries.length }}</span></summary>
      <ul><li v-for="(entry, index) in recoveredEntries" :key="index">{{ entry }}</li></ul>
    </details>
    <details v-if="skippedEntries.length" class="recovery-details-warning" data-testid="recovery-skipped-details">
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
summary:focus-visible { outline: 2px solid var(--dynamic-accent); outline-offset: -2px; border-radius: .7rem; }
</style>
