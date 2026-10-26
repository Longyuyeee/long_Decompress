<script setup lang="ts">
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import SevenZipPreprocessField from './SevenZipPreprocessField.vue'
import { SEVEN_ZIP_METHOD_OPTIONS, type SevenZipMethod } from '@/utils/sevenZipMethod'
defineProps<{ format: string; modelValue?: SevenZipMethod; filter?: 'none'|'delta'|'bcj_x86'|'bcj_arm32'|'bcj_arm_thumb'; distance?: number; hidePreprocess?: boolean; disabled?: boolean; testId?: string }>()
defineEmits<{ 'update:modelValue': [value: SevenZipMethod]; 'update:filter':[value:'none'|'delta'|'bcj_x86'|'bcj_arm32'|'bcj_arm_thumb']; 'update:distance':[value:number|undefined] }>()
</script>
<template>
  <div v-if="format === '7z'" class="min-w-0 space-y-1.5" :data-testid="`${testId || 'sevenz-method'}-field`">
    <div class="flex items-center gap-1.5 text-xs text-muted"><span>压缩方法</span><HelpHint label="7Z 压缩方法说明">默认 LZMA2，旧配置保持原方式。LZMA、Bzip2、Deflate 与 PPMd 的效果取决于文件内容。仅归档（Copy）直接打包数据，不再压缩，体积可能略增；仍可使用密码、文件名加密、固实和数字分卷。仅归档不使用强度，切回其他方法保留原强度；压缩方法强度为 1～9，旧值 0 按 1。PPMd 为 PPMdH，阶数 2～12、内存 4～64 MiB，和 ZIP PPMd 不同。接收方需支持对应方法。</HelpHint></div>
    <ThemedSelect :model-value="modelValue ?? 'lzma2'" :disabled="disabled" :options="SEVEN_ZIP_METHOD_OPTIONS" :test-id="testId || 'sevenz-method'" aria-label="7Z 压缩方法" floating @update:model-value="$emit('update:modelValue', $event as SevenZipMethod)" />
    <SevenZipPreprocessField v-if="!hidePreprocess" :format="format" :method="modelValue" :model-value="filter" :distance="distance" :disabled="disabled" :test-id="`${testId || 'sevenz-method'}-preprocess`" @update:model-value="$emit('update:filter',$event as 'none'|'delta'|'bcj_x86'|'bcj_arm32'|'bcj_arm_thumb')" @update:distance="$emit('update:distance',$event)" />
  </div>
</template>
