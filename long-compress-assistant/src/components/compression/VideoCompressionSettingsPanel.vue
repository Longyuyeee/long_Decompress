<script setup lang="ts">
import type { VideoCompressionPreset, VideoCompressionSettings } from '@/types/video'

const props = defineProps<{ modelValue: VideoCompressionSettings, disabled?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: VideoCompressionSettings] }>()

const presetForQuality = (quality: number): VideoCompressionPreset => quality >= 84 ? 'clear' : quality >= 56 ? 'balanced' : 'small'
const updateQuality = (event: Event) => {
  const quality = Math.max(1, Math.min(100, Math.round(Number((event.target as HTMLInputElement).value))))
  emit('update:modelValue', { ...props.modelValue, quality, preset: presetForQuality(quality) })
}
const setResolution = (width: number | null, height: number | null) => emit('update:modelValue', { ...props.modelValue, maxWidth: width, maxHeight: height })
const updateDimension = (key: 'maxWidth' | 'maxHeight', event: Event) => {
  const raw = (event.target as HTMLInputElement).value
  if (raw === '') {
    setResolution(null, null)
    return
  }
  const value = Math.trunc(Number(raw))
  if (!Number.isFinite(value)) return
  emit('update:modelValue', {
    ...props.modelValue,
    [key]: value,
    ...(key === 'maxWidth' && props.modelValue.maxHeight === null ? { maxHeight: 1080 } : {}),
    ...(key === 'maxHeight' && props.modelValue.maxWidth === null ? { maxWidth: 1920 } : {}),
  })
}
const isResolution = (width: number | null, height: number | null) => props.modelValue.maxWidth === width && props.modelValue.maxHeight === height
</script>

<template>
  <div class="video-settings" :class="{ disabled }" data-testid="video-settings-panel">
    <section class="setting-section quality-section">
      <div class="setting-title">
        <span><strong>视频质量</strong><small>质量越高，画面细节和目标码率越高</small></span>
        <output data-testid="video-quality-value">{{ modelValue.quality }}</output>
      </div>
      <input data-testid="video-quality-slider" class="quality-slider" type="range" min="1" max="100" step="1" :value="modelValue.quality" :disabled="disabled" @input="updateQuality">
      <div class="range-labels"><span>更小体积</span><span>均衡</span><span>更清晰</span></div>
    </section>

    <section class="setting-section">
      <div class="setting-title"><span><strong>输出分辨率</strong><small>默认保持原尺寸；超过 4K 时按比例限制</small></span></div>
      <div class="resolution-grid" role="radiogroup" aria-label="输出分辨率">
        <button type="button" role="radio" :aria-checked="isResolution(null, null)" :class="{ active: isResolution(null, null) }" :disabled="disabled" data-testid="video-resolution-original" @click="setResolution(null, null)"><strong>原尺寸</strong><small>不主动降分辨率</small></button>
        <button type="button" role="radio" :aria-checked="isResolution(1920, 1080)" :class="{ active: isResolution(1920, 1080) }" :disabled="disabled" @click="setResolution(1920, 1080)"><strong>1080p</strong><small>最大 1920×1080</small></button>
        <button type="button" role="radio" :aria-checked="isResolution(1280, 720)" :class="{ active: isResolution(1280, 720) }" :disabled="disabled" @click="setResolution(1280, 720)"><strong>720p</strong><small>最大 1280×720</small></button>
        <button type="button" role="radio" :aria-checked="isResolution(854, 480)" :class="{ active: isResolution(854, 480) }" :disabled="disabled" @click="setResolution(854, 480)"><strong>480p</strong><small>最大 854×480</small></button>
      </div>
      <div class="custom-resolution">
        <label><span>自定义宽度</span><input type="number" min="2" max="3840" step="2" :value="modelValue.maxWidth ?? ''" :disabled="disabled" placeholder="原尺寸" @change="updateDimension('maxWidth', $event)"></label>
        <span>×</span>
        <label><span>自定义高度</span><input type="number" min="2" max="3840" step="2" :value="modelValue.maxHeight ?? ''" :disabled="disabled" placeholder="原尺寸" @change="updateDimension('maxHeight', $event)"></label>
      </div>
    </section>
  </div>
</template>

<style scoped>
.video-settings{display:grid;gap:.7rem;min-width:0}.video-settings.disabled{opacity:.65;pointer-events:none}.setting-section{display:grid;gap:.55rem;min-width:0;border:1px solid var(--border-subtle);border-radius:.9rem;background:color-mix(in srgb,var(--bg-input) 76%,transparent);padding:.8rem}.setting-title{display:flex;align-items:center;justify-content:space-between;gap:.75rem}.setting-title span,.setting-title strong,.setting-title small{display:block;min-width:0}.setting-title strong{color:var(--text-content);font-size:.7rem;font-weight:900}.setting-title small{margin-top:.12rem;color:var(--text-muted);font-size:.57rem;line-height:1.4}.setting-title output{flex:0 0 auto;border-radius:999px;background:color-mix(in srgb,var(--dynamic-accent) 14%,transparent);padding:.28rem .55rem;color:var(--dynamic-accent);font-size:.75rem;font-weight:950}.quality-slider{width:100%;height:.45rem;accent-color:var(--dynamic-accent);cursor:pointer}.range-labels{display:flex;justify-content:space-between;color:var(--text-muted);font-size:.52rem;font-weight:750}.resolution-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:.45rem}.resolution-grid button{min-width:0;border:1px solid var(--border-subtle);border-radius:.72rem;background:var(--bg-card);padding:.58rem;text-align:left;color:var(--text-content);transition:border-color .18s ease,background .18s ease,transform .18s ease}.resolution-grid button:hover{transform:translateY(-1px);border-color:color-mix(in srgb,var(--dynamic-accent) 34%,var(--border-subtle))}.resolution-grid button.active{border-color:color-mix(in srgb,var(--dynamic-accent) 62%,transparent);background:color-mix(in srgb,var(--dynamic-accent) 11%,var(--bg-card));box-shadow:0 8px 24px -20px var(--dynamic-accent)}.resolution-grid strong,.resolution-grid small{display:block}.resolution-grid strong{font-size:.66rem;font-weight:900}.resolution-grid small{margin-top:.14rem;color:var(--text-muted);font-size:.51rem;line-height:1.35}.custom-resolution{display:grid;grid-template-columns:minmax(0,1fr) auto minmax(0,1fr);align-items:end;gap:.45rem}.custom-resolution label{min-width:0;color:var(--text-muted);font-size:.55rem;font-weight:800}.custom-resolution label span{display:block;margin-bottom:.25rem}.custom-resolution input{box-sizing:border-box;width:100%;border:1px solid var(--border-subtle);border-radius:.65rem;background:var(--bg-card);padding:.5rem .6rem;color:var(--text-content);outline:none}.custom-resolution input:focus{border-color:var(--dynamic-accent);box-shadow:0 0 0 2px color-mix(in srgb,var(--dynamic-accent) 12%,transparent)}.custom-resolution>span{padding-bottom:.5rem;color:var(--text-muted);font-weight:900}@media(max-width:760px){.resolution-grid{grid-template-columns:repeat(2,minmax(0,1fr))}}@media(max-width:480px){.resolution-grid{grid-template-columns:1fr}.custom-resolution{grid-template-columns:1fr}.custom-resolution>span{display:none}}
</style>
