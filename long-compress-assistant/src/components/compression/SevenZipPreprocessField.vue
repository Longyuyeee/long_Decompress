<script setup lang="ts">
import { computed } from 'vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
const props = defineProps<{format: string; method?: string; modelValue?: 'none'|'delta'|'keep'; distance?: number; batch?: boolean; disabled?: boolean; testId?: string}>()
const emit = defineEmits<{'update:modelValue':[value:'none'|'delta'|'keep']; 'update:distance':[value:number|undefined]}>()
const supported = computed(() => props.batch && (!props.method || props.method === 'keep') || ['lzma2','lzma'].includes(props.method ?? 'lzma2'))
const id = computed(() => props.testId || 'sevenz-preprocess')
const options = computed(() => [...(props.batch ? [{value:'keep',label:'保持各自设置'}] : []), {value:'none',label:'无 · 默认'}, {value:'delta',label:'Delta'}])
function changeDistance(event: Event) {const raw=(event.target as HTMLInputElement).value;emit('update:distance',raw===''?undefined:Number(raw))}
</script>
<template>
  <details v-if="format === '7z' || batch && format === 'keep'" class="rounded-lg border border-subtle bg-input/30 text-xs" :data-testid="id">
    <summary class="flex cursor-pointer list-none items-center justify-between gap-2 px-3 py-2 text-muted">
      <span class="flex min-w-0 items-center gap-1.5"><i class="pi pi-sliders-h" aria-hidden="true"></i>高级预处理<HelpHint label="7Z 预处理说明">Delta 在 LZMA2/LZMA 前处理相关字节，适合尝试原始采样数据，效果取决于内容，默认关闭。距离 1～256；其他方法保留此设置但不会执行。多选只修改 7Z 选项，留空距离表示保持各自值；不会改变密码或分卷。</HelpHint></span>
      <span class="shrink-0 text-primary">{{ modelValue === 'delta' ? `Delta · ${supported ? distance ?? (batch ? '各自' : 1) : '暂未应用'}` : modelValue === 'keep' ? '保持各自' : '已关闭' }}<i class="pi pi-angle-down ml-1" aria-hidden="true"></i></span>
    </summary>
    <div class="grid min-w-0 grid-cols-1 items-end gap-2 border-t border-subtle p-3 sm:grid-cols-2">
      <label class="min-w-0 space-y-1"><span class="text-muted">预处理</span><ThemedSelect :model-value="modelValue ?? (batch ? 'keep' : 'none')" :options="options" :disabled="disabled || !supported" :test-id="`${id}-filter`" floating @update:model-value="emit('update:modelValue',$event as 'none'|'delta'|'keep')" /></label>
      <label v-if="modelValue === 'delta'" class="min-w-0 space-y-1"><span class="text-muted">字节距离</span><input type="number" min="1" max="256" step="1" :value="distance ?? ''" :disabled="disabled || !supported" :placeholder="batch ? '保持各自值' : '1（默认）'" :data-testid="`${id}-distance`" class="h-10 w-full min-w-0 rounded-xl border border-subtle bg-input px-3 text-sm text-content outline-none focus:border-primary disabled:opacity-50" @input="changeDistance" /></label>
    </div>
  </details>
</template>
