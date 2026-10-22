<script setup lang="ts">
defineProps<{ title: string; description: string; icon: string }>()
</script>

<template>
  <header class="workspace-heading">
    <div class="workspace-heading-copy">
      <span class="workspace-heading-icon" aria-hidden="true"><i :class="icon" /></span>
      <div class="min-w-0"><h1>{{ title }}</h1><p>{{ description }}</p></div>
    </div>
    <div v-if="$slots.default" class="workspace-heading-actions"><slot /></div>
  </header>
</template>

<style scoped>
.workspace-heading{display:flex;align-items:center;justify-content:space-between;gap:16px;flex-wrap:wrap;margin-bottom:20px}
.workspace-heading-copy{display:flex;align-items:center;gap:12px;min-width:0;flex:1 1 280px}
.workspace-heading-icon{display:grid;place-items:center;flex-shrink:0;width:44px;height:44px;border:1px solid color-mix(in srgb,var(--dynamic-accent) 18%,transparent);border-radius:14px;background:color-mix(in srgb,var(--dynamic-accent) 9%,transparent);color:var(--dynamic-accent);font-size:20px}
h1{font-size:26px;font-weight:900;letter-spacing:-.035em;line-height:1.3;color:var(--text-content)}
p{font-size:12px;line-height:1.7;margin-top:5px;color:var(--text-muted);overflow-wrap:anywhere}
.workspace-heading-actions{display:flex;align-items:center;gap:8px;flex-wrap:wrap;max-width:100%}
@media(max-width:800px){h1{font-size:23px}.workspace-heading{margin-bottom:16px;gap:12px}.workspace-heading-icon{width:38px;height:38px;font-size:18px;border-radius:12px}}
</style>
