<script setup lang="ts">
import type { VideoCompressionPreset, VideoCompressionSettings } from '@/types/video'

const props = defineProps<{
  modelValue: VideoCompressionSettings
  disabled?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: VideoCompressionSettings]
}>()

const presets: Array<{ id: VideoCompressionPreset, label: string, detail: string }> = [
  { id: 'clear', label: '清晰', detail: '较高码率 · 默认 1080p' },
  { id: 'balanced', label: '均衡', detail: '质量与体积平衡 · 默认 720p' },
  { id: 'small', label: '小体积', detail: '较低码率 · 默认 480p' },
]

const updatePreset = (preset: VideoCompressionPreset) => {
  emit('update:modelValue', { ...props.modelValue, preset })
}

const toggleCustomMaximum = (enabled: boolean) => {
  emit('update:modelValue', {
    ...props.modelValue,
    maxWidth: enabled ? 1920 : null,
    maxHeight: enabled ? 1080 : null,
  })
}

const updateDimension = (key: 'maxWidth' | 'maxHeight', event: Event) => {
  const value = Math.trunc(Number((event.target as HTMLInputElement).value))
  emit('update:modelValue', { ...props.modelValue, [key]: Number.isFinite(value) ? value : 0 })
}
</script>

<template>
  <div class="video-settings" :class="{ disabled }" data-testid="video-settings-panel">
    <div class="preset-grid" role="radiogroup" aria-label="视频压缩档位">
      <button
        v-for="preset in presets"
        :key="preset.id"
        type="button"
        role="radio"
        :aria-checked="modelValue.preset === preset.id"
        :class="{ active: modelValue.preset === preset.id }"
        :disabled="disabled"
        :data-testid="`video-preset-${preset.id}`"
        @click="updatePreset(preset.id)"
      >
        <strong>{{ preset.label }}</strong>
        <small>{{ preset.detail }}</small>
      </button>
    </div>

    <label class="custom-toggle">
      <input
        type="checkbox"
        :checked="modelValue.maxWidth !== null && modelValue.maxHeight !== null"
        :disabled="disabled"
        @change="toggleCustomMaximum(($event.target as HTMLInputElement).checked)"
      >
      <span><strong>自定义最大分辨率</strong><small>按旋转后的可见方向约束；不会放大输入</small></span>
    </label>

    <div v-if="modelValue.maxWidth !== null && modelValue.maxHeight !== null" class="dimension-grid">
      <label>
        <span>最大宽度</span>
        <input type="number" min="2" max="3840" step="2" :value="modelValue.maxWidth" :disabled="disabled" @input="updateDimension('maxWidth', $event)">
      </label>
      <span>×</span>
      <label>
        <span>最大高度</span>
        <input type="number" min="2" max="3840" step="2" :value="modelValue.maxHeight" :disabled="disabled" @input="updateDimension('maxHeight', $event)">
      </label>
    </div>
  </div>
</template>

<style scoped>
.video-settings { display: grid; gap: .75rem; min-width: 0; }
.video-settings.disabled { opacity: .65; pointer-events: none; }
.preset-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: .5rem; }
.preset-grid button { min-width: 0; border: 1px solid var(--border-subtle); border-radius: .8rem; background: var(--bg-input); padding: .7rem; text-align: left; color: var(--text-content); }
.preset-grid button.active { border-color: color-mix(in srgb, var(--dynamic-accent) 55%, transparent); background: color-mix(in srgb, var(--dynamic-accent) 10%, var(--bg-input)); }
.preset-grid strong, .preset-grid small { display: block; }
.preset-grid strong { font-size: .72rem; font-weight: 900; }
.preset-grid small { margin-top: .2rem; color: var(--text-muted); font-size: .58rem; line-height: 1.35; }
.custom-toggle { display: flex; align-items: center; gap: .6rem; border: 1px solid var(--border-subtle); border-radius: .75rem; padding: .65rem .75rem; color: var(--text-content); }
.custom-toggle input { accent-color: var(--dynamic-accent); }
.custom-toggle span, .custom-toggle strong, .custom-toggle small { display: block; min-width: 0; }
.custom-toggle strong { font-size: .68rem; }
.custom-toggle small { margin-top: .12rem; color: var(--text-muted); font-size: .58rem; }
.dimension-grid { display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr); align-items: end; gap: .5rem; }
.dimension-grid label { min-width: 0; color: var(--text-muted); font-size: .6rem; font-weight: 800; }
.dimension-grid label span { display: block; margin-bottom: .25rem; }
.dimension-grid input { box-sizing: border-box; width: 100%; border: 1px solid var(--border-subtle); border-radius: .65rem; background: var(--bg-input); padding: .55rem .65rem; color: var(--text-content); }
.dimension-grid > span { padding-bottom: .55rem; color: var(--text-muted); font-weight: 900; }
@media (max-width: 620px) { .preset-grid { grid-template-columns: 1fr; } }
</style>
