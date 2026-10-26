<script setup lang="ts">
import { computed } from 'vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
const props = defineProps<{format: string; method?: string; modelValue?: 'none'|'delta'|'bcj_x86'|'bcj_arm32'|'bcj_arm_thumb'|'bcj_arm64'|'keep'; distance?: number; batch?: boolean; disabled?: boolean; testId?: string}>()
const emit = defineEmits<{'update:modelValue':[value:'none'|'delta'|'bcj_x86'|'bcj_arm32'|'bcj_arm_thumb'|'bcj_arm64'|'keep']; 'update:distance':[value:number|undefined]}>()
const supported = computed(() => props.batch && (!props.method || props.method === 'keep') || ['lzma2','lzma'].includes(props.method ?? 'lzma2'))
const id = computed(() => props.testId || 'sevenz-preprocess')
const options = computed(() => [...(props.batch ? [{value:'keep',label:'保持各自设置'}] : []), {value:'none',label:'无 · 默认'}, {value:'delta',label:'Delta'}, {value:'bcj_x86',label:'x86 BCJ'}, {value:'bcj_arm32',label:'ARM32 BCJ'}, {value:'bcj_arm_thumb',label:'ARM Thumb BCJ'}, {value:'bcj_arm64',label:'ARM64 BCJ'}])
function changeDistance(event: Event) {const raw=(event.target as HTMLInputElement).value;emit('update:distance',raw===''?undefined:Number(raw))}
</script>
<template>
  <details v-if="format === '7z' || batch && format === 'keep'" class="rounded-lg border border-subtle bg-input/30 text-xs" :data-testid="id">
    <summary class="flex flex-wrap cursor-pointer list-none items-center justify-between gap-x-2 gap-y-1 px-3 py-2 text-muted">
      <span class="flex min-w-0 shrink-0 items-center gap-1.5"><i class="pi pi-sliders-h" aria-hidden="true"></i>高级预处理<HelpHint label="7Z 预处理说明">Delta 适合尝试相关采样字节，距离 1～256；x86 / ARM32 / ARM Thumb / ARM64 BCJ 分别面向对应架构的程序指令。以上预处理只用于 LZMA2/LZMA，效果取决于内容，默认关闭；其他方法保留此设置但不会执行。多选只修改 7Z 选项，留空距离表示保持各自值；不会改变密码或分卷。</HelpHint></span>
      <span class="ml-auto shrink-0 text-right text-primary">{{ modelValue === 'delta' ? `Delta · ${supported ? distance ?? (batch ? '各自' : 1) : '暂未应用'}` : modelValue === 'bcj_x86' ? `x86 BCJ${supported ? '' : ' · 暂未应用'}` : modelValue === 'bcj_arm32' ? `ARM32 BCJ${supported ? '' : ' · 暂未应用'}` : modelValue === 'bcj_arm_thumb' ? `ARM Thumb BCJ${supported ? '' : ' · 暂未应用'}` : modelValue === 'bcj_arm64' ? `ARM64 BCJ${supported ? '' : ' · 暂未应用'}` : modelValue === 'keep' ? '保持各自' : '已关闭' }}<i class="pi pi-angle-down ml-1" aria-hidden="true"></i></span>
    </summary>
    <div class="grid min-w-0 grid-cols-1 items-end gap-2 border-t border-subtle p-3 sm:grid-cols-2">
      <label class="min-w-0 space-y-1" :class="{'sm:col-span-2':modelValue !== 'delta'}"><span class="text-muted">预处理</span><ThemedSelect :model-value="modelValue ?? (batch ? 'keep' : 'none')" :options="options" :disabled="disabled || !supported" :test-id="`${id}-filter`" floating @update:model-value="emit('update:modelValue',$event as 'none'|'delta'|'bcj_x86'|'bcj_arm32'|'bcj_arm_thumb'|'bcj_arm64'|'keep')" /></label>
      <label v-if="modelValue === 'delta'" class="min-w-0 space-y-1"><span class="text-muted">字节距离</span><input type="number" min="1" max="256" step="1" :value="distance ?? ''" :disabled="disabled || !supported" :placeholder="batch ? '保持各自值' : '1（默认）'" :data-testid="`${id}-distance`" class="h-10 w-full min-w-0 rounded-xl border border-subtle bg-input px-3 text-sm text-content outline-none focus:border-primary disabled:opacity-50" @input="changeDistance" /></label>
    </div>
  </details>
</template>
