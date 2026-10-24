<script setup lang="ts">
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import { SEVEN_ZIP_METHOD_OPTIONS, type SevenZipMethod } from '@/utils/sevenZipMethod'
defineProps<{ format: string; modelValue?: SevenZipMethod; disabled?: boolean; testId?: string }>()
defineEmits<{ 'update:modelValue': [value: SevenZipMethod] }>()
</script>
<template>
  <div v-if="format === '7z'" class="min-w-0 space-y-1.5" :data-testid="`${testId || 'sevenz-method'}-field`">
    <div class="flex items-center gap-1.5 text-xs text-muted"><span>压缩方法</span><HelpHint label="7Z 压缩方法说明">默认 LZMA2，旧配置保持原方式。LZMA、Bzip2、Deflate 与 PPMd 提供其他选择，实际效果取决于文件内容。五种方法均可搭配密码、文件名加密、固实和数字分卷；强度为 1～9，旧配置中的 0 按 1 执行。PPMd 使用 7Z 的 PPMdH，按强度调整阶数（2～12）和内存（4～64 MiB），内存上限 64 MiB，和 ZIP PPMd 不同。接收方需使用支持对应 7Z 方法的软件。</HelpHint></div>
    <ThemedSelect :model-value="modelValue ?? 'lzma2'" :disabled="disabled" :options="SEVEN_ZIP_METHOD_OPTIONS" :test-id="testId || 'sevenz-method'" aria-label="7Z 压缩方法" floating @update:model-value="$emit('update:modelValue', $event as SevenZipMethod)" />
  </div>
</template>
