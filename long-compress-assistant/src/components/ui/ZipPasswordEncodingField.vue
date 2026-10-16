<script setup lang="ts">
import ThemedSelect from './ThemedSelect.vue'
import HelpHint from './HelpHint.vue'
import { isZipPasswordEncoding, ZIP_PASSWORD_ENCODING_OPTIONS, type ZipPasswordEncoding } from '@/types/zipPassword'

defineProps<{ modelValue?: ZipPasswordEncoding; disabled?: boolean; testId?: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: ZipPasswordEncoding | undefined] }>()
const options = [{ value: 'auto', label: '自动兼容（默认）' }, ...ZIP_PASSWORD_ENCODING_OPTIONS]
function choose(value: string) {
  if (value === 'auto') emit('update:modelValue', undefined)
  else if (isZipPasswordEncoding(value)) emit('update:modelValue', value)
}
</script>

<template>
  <div class="min-w-0 space-y-1.5" data-testid="zip-password-encoding-field">
    <div class="flex items-center gap-1.5 text-xs text-muted">
      <span>ZIP 密码编码</span>
      <HelpHint label="ZIP 密码编码说明">仅影响 ZIP/ZIPX 的密码字节，不改变文件名或预览文字。通常保持自动；已知旧包使用日文、简体或繁体代码页时再指定。明确选择后只使用该编码，不轮流猜测其他编码。</HelpHint>
    </div>
    <ThemedSelect :model-value="modelValue ?? 'auto'" :options="options" :disabled="disabled" :test-id="testId" aria-label="ZIP 密码编码" floating @update:model-value="choose" />
  </div>
</template>
