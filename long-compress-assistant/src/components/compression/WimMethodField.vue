<script setup lang="ts">
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
defineProps<{ modelValue?: 'store' | 'xpress' | 'lzx' | 'lzms'; format: string; disabled?: boolean; testId?: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: 'store' | 'xpress' | 'lzx' | 'lzms'] }>()
</script>
<template>
  <div v-if="format === 'wim'" class="min-w-0 space-y-1.5">
    <div class="flex items-center gap-2 text-xs font-semibold text-muted">WIM 压缩方法<HelpHint label="WIM 压缩方法说明">仅存储仍会对相同内容去重；XPRESS 速度优先，LZX 更侧重体积；LZMS 可进一步尝试压缩，通常更耗时且旧工具兼容性较少。实际效果随内容变化，无需调整强度。WIM 没有原生密码，数字分卷需集齐后读取。</HelpHint></div>
    <ThemedSelect :model-value="modelValue || 'store'" :disabled="disabled" :test-id="testId || 'wim-method'" :options="[{value:'store',label:'仅存储 · 去重'},{value:'xpress',label:'XPRESS · 快速压缩'},{value:'lzx',label:'LZX · 体积优先'},{value:'lzms',label:'LZMS · 高压缩'}]" floating @update:model-value="emit('update:modelValue', $event as 'store' | 'xpress' | 'lzx' | 'lzms')" />
  </div>
</template>
