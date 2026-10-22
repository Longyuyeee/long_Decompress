<script setup lang="ts">
import { provide } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { TOOL_NAVIGATION, TOOLS_CENTER_EMBEDDED } from '@/utils/navigation'

const app = useAppStore(), route = useRoute(), router = useRouter()
provide(TOOLS_CENTER_EMBEDDED, true)
function open(name: string) { void router.push({ name }) }
</script>

<template>
  <div class="tools-center" data-testid="tools-center">
    <header class="tools-header">
      <h1><i class="pi pi-th-large" aria-hidden="true" />{{ app.t('nav.tools') }}</h1>
      <nav class="tools-tabs" :aria-label="app.t('nav.tools')" data-testid="tools-mode-switch">
        <button v-for="tool in TOOL_NAVIGATION" :key="tool.name" type="button"
          :data-testid="`tool-${tool.name}`" :title="`${app.t(tool.label)} (${tool.shortcut})`"
          :aria-current="route.name === tool.name ? 'page' : undefined"
          :class="{ active: route.name === tool.name }" @click="open(tool.name)">
          <i :class="tool.icon" aria-hidden="true" /><span>{{ app.t(tool.label) }}</span>
        </button>
      </nav>
    </header>
    <section class="tools-stage" :aria-label="app.t(TOOL_NAVIGATION.find(tool => tool.name === route.name)?.label || 'nav.tools')">
      <RouterView v-slot="{ Component }">
        <KeepAlive><component :is="Component" :key="route.name" /></KeepAlive>
      </RouterView>
    </section>
  </div>
</template>

<style scoped>
.tools-center{height:100%;min-width:0;min-height:0;display:flex;flex-direction:column;gap:10px;padding:12px 16px;overflow:hidden;container:tools / inline-size}
.tools-header{display:flex;align-items:center;gap:10px 16px;flex-wrap:wrap;flex-shrink:0;min-width:0}
h1{display:flex;align-items:center;gap:9px;font-size:22px;line-height:1.3;font-weight:900;letter-spacing:-.035em;color:var(--text-content);white-space:nowrap}
h1>i{font-size:19px;color:var(--dynamic-accent)}
.tools-tabs{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:3px;min-width:0;padding:3px;border:1px solid var(--border-subtle);border-radius:10px;background:color-mix(in srgb,var(--bg-card) 72%,transparent)}
.tools-tabs button{display:flex;align-items:center;justify-content:center;gap:7px;height:32px;padding:0 10px;min-width:0;border:1px solid transparent;border-radius:7px;font-size:11px;font-weight:850;color:var(--text-muted);white-space:nowrap;transition:background .18s,color .18s,border-color .18s}
.tools-tabs button:hover{color:var(--text-content);background:color-mix(in srgb,var(--dynamic-accent) 7%,transparent)}
.tools-tabs button.active{color:var(--dynamic-accent);border-color:color-mix(in srgb,var(--dynamic-accent) 24%,transparent);background:color-mix(in srgb,var(--dynamic-accent) 13%,transparent);box-shadow:0 8px 20px -15px var(--dynamic-accent)}
.tools-tabs button:focus-visible{outline:2px solid var(--dynamic-accent);outline-offset:2px}
.tools-stage{flex:1;min-width:0;min-height:0;overflow:hidden}
.tools-stage:deep(.module-page){padding:0;height:100%}
.tools-stage:deep(.integrity-view){padding:0;height:100%}
.tools-stage:deep(.workspace-heading){margin-bottom:10px}
@container tools (max-width:560px){.tools-header{gap:8px}h1{font-size:20px}.tools-tabs{width:100%}.tools-tabs button{padding-inline:6px;gap:5px}}
@container tools (max-width:380px){.tools-tabs{grid-template-columns:repeat(2,minmax(0,1fr))}}
@media(max-width:840px){.tools-center{padding:10px 12px}}
@media(prefers-reduced-motion:reduce){.tools-tabs button{transition:none}}
</style>
