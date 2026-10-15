<script setup lang="ts">
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
defineProps<{ modelValue?: number; disabled?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: number] }>()
</script>
<template>
  <details class="rar-recovery-field rounded-xl border border-subtle bg-input/30 px-3 py-2.5" :open="!!modelValue" data-testid="rar-recovery-settings">
    <summary class="flex cursor-pointer list-none flex-wrap items-center gap-2 text-sm font-bold text-content"><i class="pi pi-shield text-primary" aria-hidden="true"></i>恢复保护<span class="ml-auto text-xs font-normal text-muted">{{ modelValue ? `${modelValue}% 恢复记录` : '未开启 · 可选' }}</span><i class="pi pi-angle-down text-xs text-muted" aria-hidden="true"></i></summary>
    <div class="mt-3 space-y-2">
      <ThemedSelect :model-value="String(modelValue || 0)" :disabled="disabled" :options="[{value:'0',label:'关闭'}, {value:'3',label:'3% · 较小额外体积'}, {value:'5',label:'5% · 日常备份'}, {value:'10',label:'10% · 更多恢复数据'}]" test-id="rar-recovery-percent" floating @update:model-value="emit('update:modelValue', Number($event))" />
      <p class="text-xs font-normal leading-relaxed text-muted">为 RAR 添加恢复记录，包受损时可尝试修复。比例越高，额外体积越大；恢复记录不能代替缺失分卷所需的 .rev 恢复卷。</p>
    </div>
  </details>
</template>
