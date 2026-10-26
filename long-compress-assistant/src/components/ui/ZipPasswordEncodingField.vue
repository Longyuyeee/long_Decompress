<script setup lang="ts">
import ThemedSelect from './ThemedSelect.vue'
import HelpHint from './HelpHint.vue'
import { computed } from 'vue'
import { isZipPasswordEncoding, ZIP_PASSWORD_ENCODING_OPTIONS, LZOP_NAME_ENCODING_OPTIONS, type ZipPasswordEncoding } from '@/types/zipPassword'
import { archiveSourceFormat } from '@/utils/archiveSource'

const props = defineProps<{ modelValue?: ZipPasswordEncoding; disabled?: boolean; testId?: string; mode?: 'input' | 'output'; label?: string; sourcePath?: string; sourceFormat?: string | null }>()
const emit = defineEmits<{ 'update:modelValue': [value: ZipPasswordEncoding | undefined] }>()
const isAlz = computed(() => props.mode !== 'output' && (props.sourceFormat || archiveSourceFormat(props.sourcePath || ''))?.toLowerCase() === 'alz')
const isEgg = computed(() => props.mode !== 'output' && (props.sourceFormat || archiveSourceFormat(props.sourcePath || ''))?.toLowerCase() === 'egg')
const isLzop = computed(() => props.mode !== 'output' && ['lzo', 'lzop'].includes((props.sourceFormat || archiveSourceFormat(props.sourcePath || ''))?.toLowerCase() || ''))
const fieldLabel = computed(() => isLzop.value ? 'LZOP 成员名称编码' : isAlz.value ? 'ALZ 文件名与密码编码' : isEgg.value ? 'EGG 密码编码' : props.label || 'ZIP 密码编码')
const options = computed(() => props.mode === 'output' ? ZIP_PASSWORD_ENCODING_OPTIONS : [{ value: 'auto', label: isLzop.value ? '按成员标记（默认）' : isAlz.value ? '韩文（CP949，默认）' : isEgg.value ? 'UTF-8（默认）' : '自动兼容（默认）' }, ...(isLzop.value ? LZOP_NAME_ENCODING_OPTIONS : ZIP_PASSWORD_ENCODING_OPTIONS)])
function choose(value: string) {
  if (value === 'auto' && props.mode !== 'output') emit('update:modelValue', undefined)
  else if (isZipPasswordEncoding(value) && options.value.some(option => option.value === value)) emit('update:modelValue', value)
}
</script>

<template>
  <div class="min-w-0 space-y-1.5" data-testid="zip-password-encoding-field">
    <div class="flex items-center gap-1.5 text-xs text-muted">
      <span>{{ fieldLabel }}</span>
      <HelpHint :label="`${fieldLabel}说明`"><template v-if="isLzop">先按成员的 UTF-8 或 Latin-1 标记读取。旧 Windows/DOS 来源未注明具体代码页时，可选择原工具编码并重新读取目录；浏览、预览、取回和转换沿用同一选择。默认不猜中文编码，无法确认的名称使用固定成员名。仅影响名称，不改变文件正文。</template><template v-else-if="isEgg">EGG 密码默认按 UTF-8。旧版 ALZip 可使用原系统代码页，中文密码不匹配时可明确选择简体中文 CP936。只影响密码字节，文件名仍按归档自身标记读取；明确选择后不猜其他编码，也不替换无法表示的字符。</template><template v-else-if="isAlz">ALZ 没有统一的 Unicode 文件名标记，默认按旧韩文 CP949 读取。若来自简体中文工具或名称乱码，请选择原工具代码页；同时用于文件名和密码字节。明确选择后只用该编码，不替换无法表示的字符。改动后请重新读取目录。</template><template v-else-if="mode === 'output'">新包默认使用 UTF-8。与旧工具互通时可明确指定代码页；无法表示的字符会报错，不会替换。只影响新 ZIP/ZIPX 的密码字节，不改变原包读取编码或文件名。</template><template v-else>仅影响 ZIP/ZIPX 的密码字节，不改变文件名或预览文字。通常保持自动；已知旧包使用日文、简体或繁体代码页时再指定。明确选择后只使用该编码，不轮流猜测其他编码。</template></HelpHint>
    </div>
    <ThemedSelect :model-value="modelValue ?? (mode === 'output' ? 'utf8' : 'auto')" :options="options" :disabled="disabled" :test-id="testId" :aria-label="fieldLabel" floating @update:model-value="choose" />
  </div>
</template>
