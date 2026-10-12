<script setup lang="ts">
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import type { ConversionSettings, ConversionPatch } from '@/types/archiveConversion'
import { conversionFormats } from '@/stores/archiveConversion'
defineProps<{ settings: ConversionSettings; disabled?: boolean; sourcePasswordRequired?: boolean; targetPasswordRequired?: boolean }>()
const emit = defineEmits<{ update: [patch: ConversionPatch] }>()
const value = (event: Event) => (event.target as HTMLInputElement).value
</script>
<template>
  <fieldset :disabled="disabled" class="grid min-w-0 grid-cols-1 gap-3 sm:grid-cols-2 disabled:opacity-60">
    <label class="conversion-label">目标格式<ThemedSelect :model-value="settings.format" :options="conversionFormats" :disabled="disabled" test-id="conversion-format" floating @update:model-value="emit('update', { format: $event })" /></label>
    <label class="conversion-label">压缩强度 · {{ settings.level }} / 9<input type="range" min="0" max="9" :value="settings.level" class="mt-3 w-full accent-primary" @input="emit('update', { level: Number(value($event)) })" /></label>
    <div class="conversion-label"><label>原包密码<input :value="settings.sourcePassword" type="password" autocomplete="off" data-testid="conversion-source-password" class="conversion-input mt-1.5" placeholder="原包未加密可留空" @input="emit('update', { sourcePassword: value($event) })" /></label><div v-if="sourcePasswordRequired" class="flex flex-wrap items-center justify-between gap-2 text-xs text-amber-600"><span>请重新填写原包密码</span><button type="button" class="text-primary hover:underline" data-testid="conversion-no-source-password" @click="emit('update', { sourcePassword: '' })">原包无需密码</button></div></div>
    <div class="conversion-label"><label>新包密码<input :value="settings.password" :disabled="!['zip', '7z'].includes(settings.format)" type="password" autocomplete="new-password" data-testid="conversion-password" class="conversion-input mt-1.5" placeholder="留空表示新包不加密" @input="emit('update', { password: value($event) })" /></label><div v-if="targetPasswordRequired" class="flex flex-wrap items-center justify-between gap-2 text-xs text-amber-600"><span>请重新填写新包密码</span><button type="button" class="text-primary hover:underline" data-testid="conversion-no-target-password" @click="emit('update', { password: '', encryptFilenames: false })">新包不加密</button></div></div>
    <label class="conversion-label">分卷大小（MiB）<input :value="settings.splitMiB" :disabled="!['zip', '7z'].includes(settings.format)" type="number" min="0" data-testid="conversion-split" class="conversion-input" @input="emit('update', { splitMiB: Number(value($event)) })" /><span class="text-xs text-muted">0 表示不分卷，也可用于合并原来的分卷</span></label>
    <label v-if="settings.format === '7z'" class="flex items-center gap-2 text-sm"><input type="checkbox" :checked="settings.encryptFilenames" data-testid="conversion-hidden-names" class="accent-primary" @change="emit('update', { encryptFilenames: ($event.target as HTMLInputElement).checked })" />加密文件名</label>
    <template v-if="settings.format === 'zip' && settings.password">
      <label class="conversion-label">ZIP 加密方式<ThemedSelect :model-value="settings.zipEncryption" :disabled="disabled" :options="[{ value: 'aes256', label: 'AES-256' }, { value: 'zipcrypto', label: 'ZipCrypto（传统兼容）' }]" floating @update:model-value="emit('update', { zipEncryption: $event as ConversionSettings['zipEncryption'] })" /></label>
      <label class="conversion-label">密码编码<ThemedSelect :model-value="settings.zipPasswordEncoding" :disabled="disabled" :options="[{ value: 'utf8', label: 'UTF-8' }, { value: 'windows_local', label: 'Windows 兼容' }]" floating @update:model-value="emit('update', { zipPasswordEncoding: $event as ConversionSettings['zipPasswordEncoding'] })" /></label>
    </template>
  </fieldset>
</template>
<style scoped>
.conversion-label { display:flex; min-width:0; flex-direction:column; gap:.4rem; font-size:.8rem; }
.conversion-input { width:100%; min-width:0; border:1px solid var(--border-subtle); border-radius:.65rem; padding:.6rem .7rem; background:var(--bg-input); }
</style>
