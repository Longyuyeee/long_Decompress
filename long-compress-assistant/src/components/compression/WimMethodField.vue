<script setup lang="ts">
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
type WimMethod = 'store' | 'xpress' | 'lzx' | 'lzms' | 'solid'
defineProps<{ modelValue?: WimMethod; format: string; disabled?: boolean; testId?: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: WimMethod] }>()
</script>
<template>
  <div v-if="['wim','esd'].includes(format)" class="min-w-0 space-y-1.5">
    <div class="flex items-center gap-2 text-xs font-semibold text-muted">{{ format === 'esd' ? 'ESD 压缩方法' : 'WIM 压缩方法' }}<HelpHint label="WIM 压缩方法说明">仅存储仍会对相同内容去重；XPRESS 速度优先，LZX 更侧重体积；LZMS 可进一步尝试压缩，通常更耗时且旧工具兼容性较少。实际效果随内容变化，无需调整强度。固实归档会把多个文件一起压缩，适合尝试保存相似内容；只取单个文件也可能读取同块数据，旧工具兼容性较少。ESD 固定使用 LZMS 固实。WIM/ESD 没有原生密码，数字分卷需集齐后读取。</HelpHint></div>
    <div v-if="format === 'esd'" :data-testid="testId || 'wim-method'" class="flex min-h-10 min-w-0 items-center justify-between gap-2 rounded-xl border border-primary/20 bg-primary/5 px-3 py-2 text-sm"><span class="font-semibold text-content">LZMS · 固实归档</span><span class="text-xs text-primary">ESD</span></div>
    <ThemedSelect v-else :model-value="modelValue || 'store'" :disabled="disabled" :test-id="testId || 'wim-method'" :options="[{value:'store',label:'仅存储 · 去重'},{value:'xpress',label:'XPRESS · 快速压缩'},{value:'lzx',label:'LZX · 体积优先'},{value:'lzms',label:'LZMS · 高压缩'},{value:'solid',label:'LZMS · 固实归档'}]" floating @update:model-value="emit('update:modelValue', $event as WimMethod)" />
    <p v-if="format === 'esd' || modelValue === 'solid'" data-testid="wim-solid-hint" class="text-xs leading-relaxed text-muted">固实模式更耗内存和时间，当前单线程编码预算约 640 MiB；只取一个文件也可能需要读取同块内容。</p>
  </div>
</template>
