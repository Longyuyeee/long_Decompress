<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '@/stores/app'
import ImageCompressionWorkspace from '@/components/compression/ImageCompressionWorkspace.vue'
import VideoCompressionWorkspace from '@/components/compression/VideoCompressionWorkspace.vue'
import PdfCompressionWorkspace from '@/components/compression/PdfCompressionWorkspace.vue'

type SpecialCompressionMode = 'image' | 'video' | 'pdf'

const appStore = useAppStore()
const activeMode = ref<SpecialCompressionMode>('image')
const modes: Array<{ id: SpecialCompressionMode, label: string, icon: string }> = [
  { id: 'image', label: '图片压缩', icon: 'pi pi-images' },
  { id: 'video', label: '视频压缩', icon: 'pi pi-video' },
  { id: 'pdf', label: 'PDF 压缩', icon: 'pi pi-file-pdf' },
]
</script>

<template>
  <div class="special-compression-view p-4 md:p-6 h-full flex flex-col gap-4 transition-colors duration-700 overflow-hidden relative" data-testid="special-compression-center">
    <header class="special-compression-header shrink-0 min-w-0">
      <div class="special-compression-heading">
        <h1 class="text-2xl md:text-3xl font-black text-content tracking-tight">{{ appStore.t('nav.special_compression') }}</h1>
        <p class="text-xs md:text-sm text-muted font-semibold mt-1">{{ appStore.t('special_compression.subtitle') }}</p>
      </div>
      <nav class="special-compression-mode-switch" aria-label="特殊压缩类型" data-testid="special-compression-mode-switch">
        <button
          v-for="mode in modes"
          :key="mode.id"
          type="button"
          :data-testid="`compression-mode-${mode.id}`"
          :aria-current="activeMode === mode.id ? 'page' : undefined"
          :class="{ active: activeMode === mode.id }"
          @click="activeMode = mode.id"
        >
          <i :class="mode.icon"></i>
          <span>{{ mode.label }}</span>
        </button>
      </nav>
    </header>

    <main class="special-compression-shell aero-card">
      <div :key="activeMode" class="special-compression-stage">
        <ImageCompressionWorkspace v-if="activeMode === 'image'" />
        <VideoCompressionWorkspace v-else-if="activeMode === 'video'" />
        <PdfCompressionWorkspace v-else />
      </div>
    </main>
  </div>
</template>

<style scoped>
.special-compression-view {
  min-width: 0;
  overflow-x: hidden;
  background: radial-gradient(circle at 100% 100%, color-mix(in srgb, var(--dynamic-accent) 4%, transparent) 0%, transparent 40%);
}

.special-compression-header {
  display: grid;
  grid-template-columns: minmax(13rem, auto) minmax(25rem, 1fr);
  align-items: center;
  gap: 1.25rem;
}

.special-compression-heading { min-width: 0; }

.special-compression-shell {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex: 1;
  overflow: hidden;
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-card) 40%, transparent);
  box-shadow: 0 20px 45px -32px rgb(0 0 0 / .35);
  padding: .85rem;
}

.special-compression-stage {
  display: flex;
  width: 100%;
  min-width: 0;
  min-height: 0;
  flex: 1;
  animation: workspace-enter .2s ease both;
}

@keyframes workspace-enter {
  from { opacity: 0; transform: translateY(6px) scale(.997); }
  to { opacity: 1; transform: none; }
}

.special-compression-mode-switch {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.5rem;
  flex-shrink: 0;
  min-width: 0;
  padding: 0.28rem;
  border: 1px solid var(--border-subtle);
  border-radius: 1rem;
  background: color-mix(in srgb, var(--bg-card) 72%, transparent);
}

.special-compression-mode-switch button {
  display: flex;
  min-width: 0;
  height: 2.35rem;
  align-items: center;
  justify-content: center;
  gap: 0.45rem;
  border: 1px solid transparent;
  border-radius: 0.75rem;
  color: var(--text-muted);
  font-size: 0.72rem;
  font-weight: 850;
  transition: 0.2s ease;
}

.special-compression-mode-switch button:hover {
  color: var(--text-content);
  background: color-mix(in srgb, var(--dynamic-accent) 7%, transparent);
}

.special-compression-mode-switch button.active {
  border-color: color-mix(in srgb, var(--dynamic-accent) 24%, transparent);
  background: color-mix(in srgb, var(--dynamic-accent) 13%, transparent);
  color: var(--dynamic-accent);
  box-shadow: 0 8px 20px -15px var(--dynamic-accent);
}

@media (max-width: 960px) {
  .special-compression-header { grid-template-columns: minmax(11rem, auto) minmax(18rem, 1fr); }
}

@media (max-width: 640px) {
  .special-compression-header { grid-template-columns: minmax(0, 1fr) auto; gap: .65rem; }
  .special-compression-mode-switch { grid-template-columns: repeat(3, 2.4rem); }
  .special-compression-mode-switch button span { display: none; }
  .special-compression-mode-switch button i { font-size: 1rem; }
}

@media (prefers-reduced-motion: reduce) {
  .special-compression-mode-switch button { transition: none; }
  .special-compression-stage { animation: none; }
}
</style>
