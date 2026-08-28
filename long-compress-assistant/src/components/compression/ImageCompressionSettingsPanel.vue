<script setup lang="ts">
import type { ImageCompressionSettings } from '@/utils/imageCompressionWorkspace'

const props = defineProps<{
  modelValue: ImageCompressionSettings
  compact?: boolean
}>()

const emit = defineEmits<{
  (event: 'update:modelValue', value: ImageCompressionSettings): void
}>()

const update = <K extends keyof ImageCompressionSettings>(key: K, value: ImageCompressionSettings[K]) => {
  emit('update:modelValue', { ...props.modelValue, [key]: value })
}
</script>

<template>
  <div class="image-settings" :class="{ compact }" data-testid="image-settings-panel">
    <label class="setting-field">
      <span>压缩方式</span>
      <select :value="modelValue.mode" @change="update('mode', ($event.target as HTMLSelectElement).value as ImageCompressionSettings['mode'])">
        <option value="lossy">有损压缩</option>
        <option value="lossless">无损优化</option>
      </select>
      <small>PNG 输出始终使用无损优化；JPEG/WebP 按所选方式处理。</small>
    </label>

    <label class="setting-field">
      <span>输出格式</span>
      <select :value="modelValue.outputFormat" @change="update('outputFormat', ($event.target as HTMLSelectElement).value as ImageCompressionSettings['outputFormat'])">
        <option value="keep">保持原格式</option>
        <option value="jpeg">JPEG</option>
        <option value="webp">WebP</option>
        <option value="png">PNG</option>
      </select>
    </label>

    <label v-if="modelValue.mode === 'lossy'" class="setting-field setting-quality">
      <span>质量 <strong>{{ modelValue.quality }}</strong></span>
      <input type="range" min="1" max="100" :value="modelValue.quality" @input="update('quality', Number(($event.target as HTMLInputElement).value))">
    </label>

    <label class="setting-field">
      <span>尺寸策略</span>
      <select :value="modelValue.resizeMode" @change="update('resizeMode', ($event.target as HTMLSelectElement).value as ImageCompressionSettings['resizeMode'])">
        <option value="keep">保持原尺寸</option>
        <option value="limit">限制最大宽高</option>
      </select>
    </label>

    <div v-if="modelValue.resizeMode === 'limit'" class="dimension-fields">
      <label class="setting-field"><span>最大宽度</span><input type="number" min="1" :value="modelValue.maxWidth" @input="update('maxWidth', Number(($event.target as HTMLInputElement).value))"></label>
      <label class="setting-field"><span>最大高度</span><input type="number" min="1" :value="modelValue.maxHeight" @input="update('maxHeight', Number(($event.target as HTMLInputElement).value))"></label>
    </div>

    <label class="setting-toggle">
      <input type="checkbox" :checked="modelValue.preserveMetadata" @change="update('preserveMetadata', ($event.target as HTMLInputElement).checked)">
      <span><strong>保留元数据</strong><small>保留 EXIF/ICC；方向信息将在真实编码后复核</small></span>
    </label>

    <label class="setting-field">
      <span>冲突策略</span>
      <select :value="modelValue.conflictPolicy" @change="update('conflictPolicy', ($event.target as HTMLSelectElement).value as ImageCompressionSettings['conflictPolicy'])">
        <option value="replace-if-smaller">仅在更小时替换</option>
        <option value="rename">自动重命名</option>
        <option value="skip">跳过已有文件</option>
      </select>
    </label>
  </div>
</template>

<style scoped>
.image-settings { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:.75rem; min-width:0; }
.setting-field { display:flex; min-width:0; flex-direction:column; gap:.4rem; color:var(--text-muted); font-size:.72rem; font-weight:800; }
.setting-field select,.setting-field input[type="number"] { width:100%; min-width:0; height:2.5rem; border:1px solid var(--border-subtle); border-radius:.75rem; background:var(--bg-input); color:var(--text-content); padding:0 .75rem; outline:none; }
.setting-field select:focus,.setting-field input:focus { border-color:var(--dynamic-accent); }
.setting-field small { color:var(--text-muted); font-size:.6rem; font-weight:650; line-height:1.35; }
.setting-quality { grid-column:1/-1; }
.setting-quality span { display:flex; justify-content:space-between; }
.setting-quality strong { color:var(--dynamic-accent); }
.setting-quality input { accent-color:var(--dynamic-accent); }
.dimension-fields { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:.5rem; min-width:0; }
.setting-toggle { display:flex; align-items:flex-start; gap:.65rem; border:1px solid var(--border-subtle); border-radius:.85rem; background:color-mix(in srgb,var(--bg-input) 75%,transparent); padding:.7rem; color:var(--text-content); }
.setting-toggle input { margin-top:.15rem; accent-color:var(--dynamic-accent); }
.setting-toggle span { min-width:0; display:flex; flex-direction:column; gap:.15rem; font-size:.72rem; }
.setting-toggle small { color:var(--text-muted); font-weight:600; line-height:1.35; }
@media(max-width:680px){.image-settings{grid-template-columns:minmax(0,1fr)}.setting-quality{grid-column:auto}.dimension-fields{grid-template-columns:minmax(0,1fr)}}
</style>
