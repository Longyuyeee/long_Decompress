<script setup lang="ts">
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
defineProps<{ format: string; modelValue?: 'deflate' | 'zstd'; disabled?: boolean; testId?: string }>()
defineEmits<{ 'update:modelValue': [value: 'deflate' | 'zstd'] }>()
</script>

<template>
  <div v-if="format === 'zip' || format === 'zipx'" class="min-w-0 space-y-1.5" :data-testid="`${testId || 'zip-method'}-field`">
    <div class="flex items-center gap-1.5 text-xs text-muted">
      <span>压缩方法</span>
      <HelpHint label="压缩方法说明">{{ format === 'zipx' ? '本版 ZIPX 使用 XZ 方法创建，可设置密码，分卷采用 .zipx.001 数字卷。标准分卷可选择 ZIP 格式。接收方需使用支持 ZIPX / XZ 的解压软件。压缩效果取决于文件内容。' : '默认沿用兼容 ZIP（Deflate；强度 0 时仅存储）。Zstandard 提供另一种压缩方法，接收方需使用支持该方法的解压软件；仅改加密方式不能解决方法不兼容。强度 0～9 在 Zstandard 下映射为 1～9，始终使用 Zstandard。' }}</HelpHint>
    </div>
    <div v-if="format === 'zipx'" class="flex min-h-10 min-w-0 items-center justify-between gap-2 rounded-xl border border-primary/20 bg-primary/5 px-3 py-2 text-sm" :data-testid="testId || 'zip-method'">
      <span class="font-semibold text-content">XZ</span><span class="text-xs text-primary">ZIPX</span>
    </div>
    <ThemedSelect v-else :model-value="modelValue ?? 'deflate'" :disabled="disabled" :test-id="testId || 'zip-method'" aria-label="ZIP 压缩方法" floating
      :options="[{ value: 'deflate', label: '兼容 ZIP · Deflate', detail: '沿用当前默认设置' }, { value: 'zstd', label: 'Zstandard', detail: '需要支持此方法的解压软件' }]"
      @update:model-value="$emit('update:modelValue', $event as 'deflate' | 'zstd')" />
  </div>
</template>
