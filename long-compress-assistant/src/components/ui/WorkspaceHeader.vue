<script setup lang="ts">
import HelpHint from './HelpHint.vue'
import { inject } from 'vue'
import { TOOLS_CENTER_EMBEDDED } from '@/utils/navigation'
defineProps<{ title: string; description?: string; icon: string }>()
const embedded = inject(TOOLS_CENTER_EMBEDDED, false)
</script>

<template>
  <header class="workspace-heading">
    <div class="workspace-heading-copy">
      <span v-if="!embedded" class="workspace-heading-icon" aria-hidden="true"><i :class="icon" /></span>
      <h1 v-if="!embedded">{{ title }}</h1><HelpHint v-if="description" :label="title+'使用说明'">{{ description }}</HelpHint>
    </div>
    <div v-if="$slots.navigation" class="workspace-heading-navigation"><slot name="navigation" /></div>
    <div v-if="$slots.default" class="workspace-heading-actions"><slot /></div>
  </header>
</template>

<style scoped>
.workspace-heading{display:flex;align-items:center;gap:10px 16px;flex-wrap:wrap;margin-bottom:12px;min-width:0}
.workspace-heading-copy{display:flex;align-items:center;gap:9px;min-width:0;flex:0 1 auto}
.workspace-heading-icon{display:grid;place-items:center;flex-shrink:0;width:32px;height:32px;border:1px solid color-mix(in srgb,var(--dynamic-accent) 18%,transparent);border-radius:10px;background:color-mix(in srgb,var(--dynamic-accent) 9%,transparent);color:var(--dynamic-accent);font-size:16px}
h1{font-size:23px;font-weight:900;letter-spacing:-.035em;line-height:1.3;color:var(--text-content)}
.workspace-heading-actions{display:flex;align-items:center;gap:8px;flex-wrap:wrap;max-width:100%;margin-left:auto}
.workspace-heading-navigation{min-width:0;flex:1 1 auto;max-width:100%}.workspace-heading-navigation :deep(nav){margin-bottom:0}
@media(max-width:800px){h1{font-size:20px}.workspace-heading{gap:9px}.workspace-heading-navigation{order:3;flex-basis:100%}}
</style>
