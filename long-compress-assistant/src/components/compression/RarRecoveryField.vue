<script setup lang="ts">
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
defineProps<{ modelValue?: number; recoveryVolumes?: number; disabled?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: number]; 'update:recoveryVolumes': [value: number] }>()
</script>
<template>
  <details class="rar-recovery-field rounded-xl border border-subtle bg-input/30 px-3 py-2.5" :open="!!modelValue || !!recoveryVolumes" data-testid="rar-recovery-settings">
    <summary class="flex cursor-pointer list-none flex-wrap items-center gap-2 text-sm font-bold text-content"><i class="pi pi-shield text-primary" aria-hidden="true"></i>恢复保护<span class="ml-auto text-xs font-normal text-muted">{{ [modelValue ? `${modelValue}% 记录` : '', recoveryVolumes ? `${recoveryVolumes} 个恢复卷` : ''].filter(Boolean).join(' · ') || '未开启 · 可选' }}</span><i class="pi pi-angle-down text-xs text-muted" aria-hidden="true"></i></summary>
    <div class="mt-3 space-y-2">
      <p class="text-xs text-muted">包内恢复记录</p>
      <ThemedSelect :model-value="String(modelValue || 0)" :disabled="disabled" :options="[{value:'0',label:'关闭'}, {value:'3',label:'3% · 较小额外体积'}, {value:'5',label:'5% · 日常备份'}, {value:'10',label:'10% · 更多恢复数据'}]" test-id="rar-recovery-percent" floating @update:model-value="emit('update:modelValue', Number($event))" />
      <p class="text-xs font-normal leading-relaxed text-muted">增加少量包内数据，文件受损时可尝试修复。</p>
      <div class="border-t border-subtle pt-3 space-y-2">
        <p class="text-xs text-muted">分包恢复卷 .rev</p>
        <ThemedSelect :model-value="String(recoveryVolumes || 0)" :disabled="disabled" :options="[0,1,2,3,5,10].map(value => ({value:String(value),label:value ? `${value} 个恢复卷` : '不生成恢复卷'}))" test-id="rar-recovery-volumes" floating @update:model-value="emit('update:recoveryVolumes', Number($event))" />
        <p class="text-xs font-normal leading-relaxed text-muted">实际生成多个 RAR 分包时附带创建。每个恢复卷大小接近一个数据卷，整组保留后可用于补回缺失或损坏的分卷。</p>
      </div>
    </div>
  </details>
</template>
