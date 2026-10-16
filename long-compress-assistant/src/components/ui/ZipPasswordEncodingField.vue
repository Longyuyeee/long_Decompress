<script setup lang="ts">
import ThemedSelect from './ThemedSelect.vue'
import HelpHint from './HelpHint.vue'
import { computed } from 'vue'
import { isZipPasswordEncoding, ZIP_PASSWORD_ENCODING_OPTIONS, type ZipPasswordEncoding } from '@/types/zipPassword'

const props = defineProps<{ modelValue?: ZipPasswordEncoding; disabled?: boolean; testId?: string; mode?: 'input' | 'output'; label?: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: ZipPasswordEncoding | undefined] }>()
const options = computed(() => props.mode === 'output' ? ZIP_PASSWORD_ENCODING_OPTIONS : [{ value: 'auto', label: '自动兼容（默认）' }, ...ZIP_PASSWORD_ENCODING_OPTIONS])
function choose(value: string) {
  if (value === 'auto' && props.mode !== 'output') emit('update:modelValue', undefined)
  else if (isZipPasswordEncoding(value)) emit('update:modelValue', value)
}
</script>

<template>
  <div class="min-w-0 space-y-1.5" data-testid="zip-password-encoding-field">
    <div class="flex items-center gap-1.5 text-xs text-muted">
      <span>{{ label || 'ZIP 密码编码' }}</span>
      <HelpHint :label="`${label || 'ZIP 密码编码'}说明`"><template v-if="mode === 'output'">新包默认使用 UTF-8。与旧工具互通时可明确指定代码页；无法表示的字符会报错，不会替换。只影响新 ZIP/ZIPX 的密码字节，不改变原包读取编码或文件名。</template><template v-else>仅影响 ZIP/ZIPX 的密码字节，不改变文件名或预览文字。通常保持自动；已知旧包使用日文、简体或繁体代码页时再指定。明确选择后只使用该编码，不轮流猜测其他编码。</template></HelpHint>
    </div>
    <ThemedSelect :model-value="modelValue ?? (mode === 'output' ? 'utf8' : 'auto')" :options="options" :disabled="disabled" :test-id="testId" :aria-label="label || 'ZIP 密码编码'" floating @update:model-value="choose" />
  </div>
</template>
