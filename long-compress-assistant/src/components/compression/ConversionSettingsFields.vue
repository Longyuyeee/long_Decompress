<script setup lang="ts">
import CompressionPurposePicker from './CompressionPurposePicker.vue'
import { conversionPurposePatch } from '@/utils/compressionPurpose'
import WimMethodField from '@/components/compression/WimMethodField.vue'
import SevenZipMethodField from '@/components/compression/SevenZipMethodField.vue'
import SevenZipPreprocessField from '@/components/compression/SevenZipPreprocessField.vue'
import ZipMethodField from '@/components/compression/ZipMethodField.vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import RarRecoveryField from './RarRecoveryField.vue'
import type { ConversionSettings, ConversionPatch } from '@/types/archiveConversion'
import { conversionFormats, useArchiveConversionStore } from '@/stores/archiveConversion'
import { watch } from 'vue'
import { isSingleFileStreamFormat, isLz4Format, supportsNativePassword, supportsArchiveSplit, supportsFilenameEncryption } from '@/utils/compressionFormat'
import ZipPasswordEncodingField from '@/components/ui/ZipPasswordEncodingField.vue'
import { supportsZipPasswordEncoding } from '@/utils/zipPasswordSource'
const props = defineProps<{ settings: ConversionSettings; disabled?: boolean; sourcePasswordRequired?: boolean; targetPasswordRequired?: boolean; sourcePath?: string; sourceFormat?: string }>()
const store = useArchiveConversionStore()
watch(() => props.settings.format, format => { if (format === 'rar') void store.refreshRarSupport() }, { immediate: true })
const emit = defineEmits<{ update: [patch: ConversionPatch] }>()
const value = (event: Event) => (event.target as HTMLInputElement).value
</script>
<template>
  <fieldset :disabled="disabled" class="grid min-w-0 grid-cols-1 gap-3 sm:grid-cols-2 disabled:opacity-60">
    <CompressionPurposePicker class="sm:col-span-2" :disabled="disabled" :has-password="!!settings.password" @apply="emit('update', conversionPurposePatch($event))" />
    <div v-if="settings.format === 'rar'" class="flex flex-wrap items-center gap-2 text-xs sm:col-span-2" :class="store.rarSupport?.available ? 'text-muted' : 'text-amber-600'">
      <span>{{ store.rarSupport === null ? '正在检测 RAR 编码器…' : store.rarSupport.available ? '已检测到 RAR 编码器，可创建密码和分卷归档。' : '创建 RAR 需要本机安装 WinRAR；也可选择 ZIP 或 7Z。' }}</span>
      <button type="button" class="text-primary hover:underline" @click="store.refreshRarSupport()">重新检测</button>
    </div>
    <label class="conversion-label">目标格式<ThemedSelect :model-value="settings.format" :options="conversionFormats" :disabled="disabled" test-id="conversion-format" floating @update:model-value="emit('update', { format: $event })" /></label>
    <WimMethodField :format="settings.format" :model-value="settings.wimMethod" :disabled="disabled" test-id="conversion-wim-method" @update:model-value="emit('update', { wimMethod: $event })" />
    <SevenZipMethodField hide-preprocess :format="settings.format" :model-value="settings.sevenZipMethod" :disabled="disabled" test-id="conversion-sevenz-method" @update:model-value="emit('update', { sevenZipMethod: $event })" />
    <SevenZipPreprocessField class="sm:col-span-2" :format="settings.format" :method="settings.sevenZipMethod" :model-value="settings.sevenZipFilter" :distance="settings.sevenZipDeltaDistance" :disabled="disabled" test-id="conversion-sevenz-method-preprocess" @update:model-value="emit('update', {sevenZipFilter:$event as 'none'|'delta'})" @update:distance="emit('update', {sevenZipDeltaDistance:$event})" />
    <ZipMethodField :format="settings.format" :model-value="settings.zipMethod" :level="settings.level" :disabled="disabled" test-id="conversion-zip-method" @update:model-value="emit('update', { zipMethod: $event })" />
    <p v-if="isSingleFileStreamFormat(settings.format)" class="text-xs leading-relaxed text-muted sm:col-span-2" data-testid="conversion-single-file-hint">此格式只适合原包中的一个顶层文件；目录或多个文件请选择 TAR 或对应的 TAR 组合，以保留原有目录结构。</p>
    <p v-if="['tar.lzma', 'lzma.aes', 'tar.lzma.aes'].includes(settings.format)" class="text-xs leading-relaxed text-muted sm:col-span-2" data-testid="conversion-lzma-combination-hint">{{ settings.format === 'tar.lzma' ? 'TAR.LZMA 保留目录结构；需要密码请选择 TAR.LZMA.AES。' : 'Long 自有加密格式，需要填写新包密码；接收方需使用 Long解压 v1.5.24 或更新版本。' }}</p>
    <p v-if="['wim','esd'].includes(settings.format)" class="text-xs leading-relaxed text-muted sm:col-span-2" data-testid="conversion-wim-hint">新建 WIM 可选仅存储、XPRESS、LZX、LZMS 或固实压缩；ESD 固定固实，不含原生密码；数字切片需集齐整组。多镜像输入保留编号目录视图，转换结果是新的单镜像归档，不生成部署用 SWM。</p>
    <label v-if="!((settings.format === '7z' && settings.sevenZipMethod === 'copy') || (settings.format === 'zip' && settings.zipMethod === 'store'))" data-testid="conversion-level-control" class="conversion-label">{{ ['wim','esd','swm'].includes(settings.format) ? (settings.format === 'esd' || settings.wimMethod === 'solid' ? 'LZMS · 固实归档' : settings.wimMethod === 'lzms' ? 'WIM · LZMS 固定算法' : settings.wimMethod === 'lzx' ? 'WIM · LZX 固定算法' : settings.wimMethod === 'xpress' ? 'WIM · XPRESS 固定算法' : 'WIM · 仅存储与去重') : isLz4Format(settings.format) ? 'LZ4 · 快速压缩' : `压缩强度 · ${(settings.format === '7z' || settings.format === 'zip' && ['bzip2', 'ppmd', 'lzma'].includes(settings.zipMethod || '')) ? Math.max(1, settings.level) : settings.level} / 9` }}<input :disabled="isLz4Format(settings.format) || ['wim','esd','swm'].includes(settings.format)" type="range" :min="(settings.format === '7z' || settings.format === 'zip' && ['bzip2', 'ppmd', 'lzma'].includes(settings.zipMethod || '')) ? 1 : 0" max="9" :value="settings.level" class="mt-3 w-full accent-primary" @input="emit('update', { level: Number(value($event)) })" /></label>
    <div class="conversion-label"><label>原包密码<input :value="settings.sourcePassword" type="password" autocomplete="off" data-testid="conversion-source-password" class="conversion-input mt-1.5" :placeholder="sourcePasswordRequired ? '请重新填写原包密码' : '原包未加密可留空'" @input="emit('update', { sourcePassword: value($event) })" /></label><div v-if="sourcePasswordRequired" class="flex flex-wrap items-center justify-between gap-2 text-xs text-amber-600"><span>请重新填写原包密码</span><button type="button" class="text-primary hover:underline" data-testid="conversion-no-source-password" @click="emit('update', { sourcePassword: '' })">原包无需密码</button></div></div>
    <div class="conversion-label"><label>新包密码<input :value="settings.password" :disabled="!supportsNativePassword(settings.format)" type="password" autocomplete="new-password" data-testid="conversion-password" class="conversion-input mt-1.5" :placeholder="targetPasswordRequired ? '请重新填写新包密码，或选择下方不加密' : !supportsNativePassword(settings.format) ? '此格式不支持密码加密' : settings.format.endsWith('.aes') ? '此格式需要填写密码' : '留空表示新包不加密'" @input="emit('update', { password: value($event) })" /></label><div v-if="targetPasswordRequired" class="flex flex-wrap items-center justify-between gap-2 text-xs text-amber-600"><span>请重新填写新包密码</span><button type="button" class="text-primary hover:underline" data-testid="conversion-no-target-password" @click="emit('update', { password: '', encryptFilenames: false })">新包不加密</button></div></div>
    <ZipPasswordEncodingField v-if="supportsZipPasswordEncoding(sourcePath || '', sourceFormat) || (!sourceFormat && settings.sourceZipPasswordEncoding !== undefined)" :model-value="settings.sourceZipPasswordEncoding" :source-path="sourcePath" :source-format="sourceFormat" :disabled="disabled" label="原包 ZIP 密码编码" test-id="conversion-source-zip-password-encoding" @update:model-value="emit('update',{sourceZipPasswordEncoding:$event})" />
    <ZipPasswordEncodingField v-if="['zip','zipx'].includes(settings.format) && settings.password" mode="output" :model-value="settings.zipPasswordEncoding" :disabled="disabled" label="新包 ZIP 密码编码" test-id="conversion-target-zip-password-encoding" @update:model-value="$event !== undefined && emit('update', { zipPasswordEncoding: $event })" />
    <label v-if="['zip','zipx'].includes(settings.format) && settings.password" class="conversion-label">ZIP 加密方式<ThemedSelect :model-value="settings.zipEncryption" :disabled="disabled" :options="[{ value: 'aes256', label: 'AES-256' }, { value: 'zipcrypto', label: 'ZipCrypto（传统兼容）' }]" floating @update:model-value="emit('update', { zipEncryption: $event as ConversionSettings['zipEncryption'] })" /></label>
    <p v-if="settings.format === 'swm'" class="text-xs leading-relaxed text-muted sm:col-span-2" data-testid="conversion-swm-hint">生成真正的 name.swm、name2.swm 等资源分包，需整组保留；单个资源可能超过目标大小。支持仅存储、XPRESS、LZX，不含原生密码，不保证系统部署。</p>
    <label class="conversion-label">{{ settings.format === 'swm' ? '目标卷大小（MiB）' : '分卷大小（MiB）' }}<input :value="settings.splitMiB" :disabled="!supportsArchiveSplit(settings.format)" type="number" :min="settings.format === 'swm' ? 0.0625 : 0" step="any" data-testid="conversion-split" class="conversion-input" @change="emit('update', { splitMiB: Number(value($event)) })" /><span class="text-xs text-muted">{{ settings.format === 'swm' ? '至少 64 KiB，单个资源可能超过目标大小' : supportsArchiveSplit(settings.format) ? '0 表示不分卷，也可用于合并原来的分卷' : '此格式暂不支持创建分卷' }}</span></label>
    <label v-if="['zip','zipx'].includes(settings.format) && settings.splitMiB > 0" class="conversion-label">ZIP 卷型<ThemedSelect :model-value="settings.zipSplitMode ?? 'numeric'" :disabled="disabled" :options="[{ value: 'numeric', label: `数字切卷 · .${settings.format}.001` }, { value: 'standard', label: settings.format === 'zipx' ? '标准 ZIPX' : '标准 ZIP · .z01/.zip', detail: settings.format === 'zipx' ? '.zx01/.zipx' : undefined }]" test-id="conversion-zip-split-mode" floating @update:model-value="emit('update', { zipSplitMode: $event as 'numeric' | 'standard' })" /></label>
    <label v-if="supportsFilenameEncryption(settings.format)" class="flex items-center gap-2 text-sm"><input type="checkbox" :checked="settings.encryptFilenames" data-testid="conversion-hidden-names" class="accent-primary" @change="emit('update', { encryptFilenames: ($event.target as HTMLInputElement).checked })" />加密文件名</label>
    <RarRecoveryField v-if="settings.format === 'rar'" class="sm:col-span-2" :model-value="settings.rarRecoveryPercent" :recovery-volumes="settings.rarRecoveryVolumes" :disabled="disabled" @update:model-value="emit('update', { rarRecoveryPercent: $event })" @update:recovery-volumes="emit('update', { rarRecoveryVolumes: $event })" />
  </fieldset>
</template>
<style scoped>
.conversion-label { display:flex; min-width:0; flex-direction:column; gap:.4rem; font-size:.8rem; }
.conversion-input { width:100%; min-width:0; border:1px solid var(--border-subtle); border-radius:.65rem; padding:.6rem .7rem; background:var(--bg-input); }
</style>
