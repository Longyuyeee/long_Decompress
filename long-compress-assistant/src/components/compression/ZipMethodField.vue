<script setup lang="ts">
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
defineProps<{ format: string; modelValue?: 'deflate' | 'zstd' | 'bzip2' | 'ppmd'; disabled?: boolean; testId?: string; level?: number }>()
defineEmits<{ 'update:modelValue': [value: 'deflate' | 'zstd' | 'bzip2' | 'ppmd'] }>()
</script>

<template>
  <div v-if="format === 'zip' || format === 'zipx'" class="min-w-0 space-y-1.5" :data-testid="`${testId || 'zip-method'}-field`">
    <div class="flex items-center gap-1.5 text-xs text-muted">
      <span>压缩方法</span>
      <HelpHint label="压缩方法说明">{{ format === 'zipx' ? 'ZIPX 使用 XZ 方法创建，可设置密码，支持数字卷和 .zx01/…/.zipx 标准卷。接收方需使用支持 ZIPX / XZ 及对应卷型的软件；标准卷已使用 WinRAR 独立验证，本机 Bandizip 7.38 Beta 对本应用生成内容存在解码异常，7-Zip 不识别该标准卷名。分享前请确认接收方能力，或改用兼容 ZIP。压缩效果取决于文件内容。' : '默认沿用兼容 ZIP（Deflate；强度 0 时仅存储）。Zstandard、BZip2 和 PPMd 提供其他压缩方法，接收方需使用支持对应方法的软件，部分系统内置解压器不支持；仅改加密方式不能解决方法不兼容。这些方法的强度有效范围为 1～9，旧配置中的 0 按 1 执行，仍使用选定方法。' }}</HelpHint>
    </div>
    <div v-if="format === 'zipx'" class="flex min-h-10 min-w-0 items-center justify-between gap-2 rounded-xl border border-primary/20 bg-primary/5 px-3 py-2 text-sm" :data-testid="testId || 'zip-method'">
      <span class="font-semibold text-content">XZ</span><span class="text-xs text-primary">ZIPX</span>
    </div>
    <ThemedSelect v-else :model-value="modelValue ?? 'deflate'" :disabled="disabled" :test-id="testId || 'zip-method'" aria-label="ZIP 压缩方法" floating
      :options="[{ value: 'deflate', label: '兼容 ZIP · Deflate', detail: '沿用当前默认设置' }, { value: 'zstd', label: 'Zstandard', detail: '需要支持此方法的解压软件' }, { value: 'bzip2', label: 'BZip2', detail: '适合尝试不同压缩效果，需接收方支持' }, { value: 'ppmd', label: 'PPMd · 文本归档', detail: '等级越高内存越多，需接收方支持' }]"
      @update:model-value="$emit('update:modelValue', $event as 'deflate' | 'zstd' | 'bzip2' | 'ppmd')" />
    <p v-if="format === 'zip' && modelValue === 'ppmd'" data-testid="zip-ppmd-memory" class="text-xs leading-relaxed text-muted">
      约 {{ 2 ** (Math.max(1, Math.min(9, level ?? 6)) - 1) }} MiB 字典 / 任务；高等级更占内存，接收方需支持 PPMd。
    </p>
  </div>
</template>
