<script setup lang="ts">
import type { ImageCompressionSettings } from '@/utils/imageCompressionWorkspace'
import ThemedSelect, { type ThemedSelectOption } from '@/components/ui/ThemedSelect.vue'

const props = defineProps<{ modelValue: ImageCompressionSettings, compact?: boolean }>()
const emit = defineEmits<{ (event: 'update:modelValue', value: ImageCompressionSettings): void }>()
const update = <K extends keyof ImageCompressionSettings>(key: K, value: ImageCompressionSettings[K]) => emit('update:modelValue', { ...props.modelValue, [key]: value })
const modeOptions: ThemedSelectOption[] = [{ value: 'lossy', label: '有损压缩', detail: '可调质量，优先减小体积' }, { value: 'lossless', label: '无损优化', detail: '保持像素内容不变' }]
const formatOptions: ThemedSelectOption[] = [{ value: 'keep', label: '保持原格式' }, { value: 'jpeg', label: 'JPEG' }, { value: 'webp', label: 'WebP' }, { value: 'png', label: 'PNG' }]
const resizeOptions: ThemedSelectOption[] = [{ value: 'keep', label: '保持原尺寸' }, { value: 'limit', label: '限制最大宽高' }]
const conflictOptions: ThemedSelectOption[] = [{ value: 'replace-if-smaller', label: '仅在更小时替换' }, { value: 'rename', label: '自动重命名' }, { value: 'skip', label: '跳过已有文件' }]
</script>

<template>
  <div class="image-settings" :class="{ compact }" data-testid="image-settings-panel">
    <div class="setting-field"><span>压缩方式</span><ThemedSelect test-id="image-setting-mode" aria-label="压缩方式" :model-value="modelValue.mode" :options="modeOptions" @update:model-value="update('mode', $event as ImageCompressionSettings['mode'])" /></div>
    <div class="setting-field"><span>输出格式</span><ThemedSelect test-id="image-setting-format" aria-label="输出格式" :model-value="modelValue.outputFormat" :options="formatOptions" @update:model-value="update('outputFormat', $event as ImageCompressionSettings['outputFormat'])" /></div>

    <label v-if="modelValue.mode === 'lossy'" class="setting-field setting-quality"><span>质量 <strong>{{ modelValue.quality }}</strong></span><input data-testid="image-setting-quality" type="range" min="1" max="100" :value="modelValue.quality" @input="update('quality', Number(($event.target as HTMLInputElement).value))"><small>质量越高，细节保留越多，输出通常也越大。</small></label>

    <div class="setting-field"><span>尺寸策略</span><ThemedSelect test-id="image-setting-resize" aria-label="尺寸策略" :model-value="modelValue.resizeMode" :options="resizeOptions" @update:model-value="update('resizeMode', $event as ImageCompressionSettings['resizeMode'])" /></div>
    <div v-if="modelValue.resizeMode === 'limit'" class="dimension-fields"><label class="setting-field"><span>最大宽度</span><input type="number" min="1" :value="modelValue.maxWidth" @change="update('maxWidth', Number(($event.target as HTMLInputElement).value))"></label><label class="setting-field"><span>最大高度</span><input type="number" min="1" :value="modelValue.maxHeight" @change="update('maxHeight', Number(($event.target as HTMLInputElement).value))"></label></div>

    <label class="setting-toggle"><input type="checkbox" :checked="modelValue.preserveMetadata" @change="update('preserveMetadata', ($event.target as HTMLInputElement).checked)"><span><strong>保留元数据</strong><small>保留 EXIF/ICC，方向信息在编码后复核</small></span></label>
    <div class="setting-field"><span>冲突策略</span><ThemedSelect test-id="image-setting-conflict" aria-label="冲突策略" placement="top" :model-value="modelValue.conflictPolicy" :options="conflictOptions" @update:model-value="update('conflictPolicy', $event as ImageCompressionSettings['conflictPolicy'])" /></div>
  </div>
</template>

<style scoped>
.image-settings{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));align-items:start;gap:.72rem;min-width:0}.setting-field{display:flex;min-width:0;flex-direction:column;gap:.38rem;color:var(--text-muted);font-size:.66rem;font-weight:800}.setting-field>span{min-height:.9rem}.setting-field input[type="number"]{box-sizing:border-box;width:100%;min-width:0;height:2.65rem;border:1px solid var(--border-subtle);border-radius:.78rem;background:var(--bg-input);color:var(--text-content);padding:0 .72rem;outline:none}.setting-field input:focus{border-color:var(--dynamic-accent);box-shadow:0 0 0 3px color-mix(in srgb,var(--dynamic-accent) 12%,transparent)}.setting-field small{color:var(--text-muted);font-size:.55rem;font-weight:650;line-height:1.35}.setting-quality{grid-column:1/-1;border:1px solid var(--border-subtle);border-radius:.85rem;background:color-mix(in srgb,var(--bg-input) 72%,transparent);padding:.72rem}.setting-quality>span{display:flex;justify-content:space-between}.setting-quality strong{color:var(--dynamic-accent)}.setting-quality input{width:100%;accent-color:var(--dynamic-accent)}.dimension-fields{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:.5rem;min-width:0}.setting-toggle{display:flex;min-height:4.05rem;box-sizing:border-box;align-items:flex-start;gap:.62rem;border:1px solid var(--border-subtle);border-radius:.85rem;background:color-mix(in srgb,var(--bg-input) 75%,transparent);padding:.68rem;color:var(--text-content)}.setting-toggle input{margin-top:.12rem;accent-color:var(--dynamic-accent)}.setting-toggle span{display:flex;min-width:0;flex-direction:column;gap:.13rem;font-size:.66rem}.setting-toggle small{color:var(--text-muted);font-size:.54rem;font-weight:600;line-height:1.35}.image-settings.compact{grid-template-columns:minmax(0,1fr);gap:.58rem}.image-settings.compact .setting-quality{grid-column:auto}.image-settings.compact .dimension-fields{grid-template-columns:repeat(2,minmax(0,1fr))}@media(max-width:680px){.image-settings{grid-template-columns:minmax(0,1fr)}.setting-quality{grid-column:auto}.dimension-fields{grid-template-columns:minmax(0,1fr)}}
</style>
