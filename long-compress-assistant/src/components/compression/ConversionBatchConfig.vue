<script setup lang="ts">
import { computed, ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import SavedLocationPicker from '@/components/ui/SavedLocationPicker.vue'
import { conversionFormats, useArchiveConversionStore } from '@/stores/archiveConversion'
import ConversionSettingsFields from './ConversionSettingsFields.vue'
import { normalizeConversionSettings } from '@/utils/archiveConversionDraft'
import { supportsNativePassword, supportsArchiveSplit } from '@/utils/compressionFormat'
const store = useArchiveConversionStore()
const visible = ref(false)
const applyError = ref('')
const mode = ref<'selected' | 'defaults'>('selected')
const format = ref('keep'), level = ref('keep'), destination = ref('keep'), directory = ref('')
const changePassword = ref(false), password = ref(''), changeSourcePassword = ref(false), sourcePassword = ref('')
const changeSplit = ref(false), splitMiB = ref(0), hiddenNames = ref('keep'), recovery = ref('keep')
const recoveryVolumes = ref('keep')
const defaults = ref({ ...store.defaults })
const keep = { value: 'keep', label: '保持各自设置' }
const changed = computed(() => format.value !== 'keep' || level.value !== 'keep' || destination.value !== 'keep' || changePassword.value || changeSourcePassword.value || changeSplit.value || hiddenNames.value !== 'keep' || recovery.value !== 'keep' || recoveryVolumes.value !== 'keep')
const valid = computed(() => (mode.value === 'defaults' || changed.value) && (destination.value !== 'custom' || !!directory.value.trim()) && (!changeSplit.value || Number.isFinite(splitMiB.value) && splitMiB.value >= 0))
function show(nextMode: 'selected' | 'defaults') {
  mode.value = nextMode; format.value = level.value = destination.value = hiddenNames.value = recovery.value = 'keep'
  recoveryVolumes.value = 'keep'
  directory.value = password.value = sourcePassword.value = ''
  changePassword.value = changeSourcePassword.value = changeSplit.value = false; splitMiB.value = 0
  defaults.value = { ...store.defaults }; visible.value = true
  applyError.value = ''
}
async function chooseDirectory() {
  try {
    const path = await open({ directory: true, multiple: false, title: '选择新包输出目录' })
    if (visible.value && typeof path === 'string') { directory.value = path; destination.value = 'custom' }
  } catch { /* The editable path remains available. */ }
}
function apply() {
  if (!valid.value || store.busy) return
  if (mode.value === 'defaults') store.updateDefaults(defaults.value)
  else applyError.value = store.applySelected({
    ...(format.value === 'keep' ? {} : { format: format.value }),
    ...(level.value === 'keep' ? {} : { level: Number(level.value) }),
    ...(destination.value === 'keep' ? {} : { outputDirectory: destination.value === 'source' ? '' : directory.value.trim() }),
    ...(changePassword.value ? { password: password.value } : {}),
    ...(changeSourcePassword.value ? { sourcePassword: sourcePassword.value } : {}),
    ...(changeSplit.value ? { splitMiB: splitMiB.value } : {}),
    ...(hiddenNames.value === 'keep' ? {} : { encryptFilenames: hiddenNames.value === 'yes' }),
    ...(recovery.value === 'keep' ? {} : { rarRecoveryPercent: Number(recovery.value) }),
    ...(recoveryVolumes.value === 'keep' ? {} : { rarRecoveryVolumes: Number(recoveryVolumes.value) }),
  })
  if (applyError.value) return
  visible.value = false
}
</script>
<template>
  <div class="flex flex-wrap items-center gap-2">
    <button type="button" class="conversion-button text-muted" :disabled="store.busy || store.preparing" @click="show('defaults')"><i class="pi pi-sliders-h" aria-hidden="true"></i>默认配置</button>
    <div v-if="store.selected.length" class="flex flex-wrap items-center gap-2 rounded-xl border border-primary/25 bg-primary/5 px-3 py-1.5" data-testid="conversion-batch-bar">
      <span class="text-xs font-semibold text-primary">已选 {{ store.selected.length }} 项</span>
      <button type="button" class="conversion-button bg-primary text-white" :disabled="store.busy || store.preparing" data-testid="conversion-batch-config" @click="show('selected')">修改所选配置</button>
      <button type="button" class="conversion-button text-muted" :disabled="store.busy || store.preparing" @click="store.pending.forEach(item => item.selected = false)">取消选择</button>
    </div>
  </div>
  <Modal v-model:visible="visible" :title="mode === 'selected' ? '修改所选转换配置' : '转换默认配置'" icon="pi pi-sliders-h" layer="nested" size="lg">
    <div class="space-y-4" data-testid="conversion-batch-fields">
      <p v-if="applyError" role="alert" class="text-xs text-red-500">{{ applyError }}</p>
      <template v-if="mode === 'defaults'">
        <p class="text-xs text-muted">应用到尚未单独编辑的任务；已编辑条目保留自己的配置。</p>
        <ConversionSettingsFields :settings="defaults" @update="defaults = normalizeConversionSettings(defaults, $event)" />
        <label class="block text-sm">输出目录<input v-model="defaults.outputDirectory" class="conversion-input mt-1 w-full" placeholder="留空放在各自源文件旁" /></label>
      </template>
      <template v-else>
        <p class="text-xs text-muted">只修改指定参数；名称、目录、密码等未指定的配置保持各自原值。</p>
        <p v-if="format !== 'keep' && (!supportsNativePassword(format) || !supportsArchiveSplit(format))" class="text-xs text-amber-600">此格式不支持的密码或分卷选项会关闭，其余配置保留。</p>
        <div class="grid min-w-0 gap-4 sm:grid-cols-2">
          <label class="min-w-0 text-sm">目标格式<ThemedSelect v-model="format" :options="[keep, ...conversionFormats]" test-id="conversion-batch-format" floating /></label>
          <label class="min-w-0 text-sm">压缩强度<ThemedSelect v-model="level" :options="[keep, ...Array.from({ length: 10 }, (_, n) => ({ value: String(n), label: String(n) }))]" test-id="conversion-batch-level" floating /></label>
          <label class="min-w-0 text-sm">RAR 恢复保护<ThemedSelect v-model="recovery" :options="[keep, {value:'0',label:'关闭'}, {value:'3',label:'3%'}, {value:'5',label:'5%'}, {value:'10',label:'10%'}]" test-id="conversion-batch-rar-recovery" floating /></label>
          <label class="min-w-0 text-sm">RAR 分包恢复卷<ThemedSelect v-model="recoveryVolumes" :options="[keep, ...[0,1,2,3,5,10].map(value => ({value:String(value),label:value ? `${value} 个 .rev 恢复卷` : '不生成恢复卷'}))]" test-id="conversion-batch-rar-recovery-volumes" floating /></label>
          <label class="min-w-0 text-sm">文件名加密<ThemedSelect v-model="hiddenNames" :options="[keep, { value: 'yes', label: '启用（7Z/RAR，需要新包密码）' }, { value: 'no', label: '关闭' }]" floating /></label>
          <div class="space-y-2 text-sm"><label class="flex items-center gap-2"><input v-model="changeSplit" type="checkbox" />修改分卷大小</label><input v-if="changeSplit" v-model.number="splitMiB" type="number" min="0" class="conversion-input w-full" placeholder="0 表示不分卷" /></div>
          <div class="space-y-2 text-sm"><label class="flex items-center gap-2"><input v-model="changeSourcePassword" type="checkbox" />填写共用原包密码</label><input v-if="changeSourcePassword" v-model="sourcePassword" type="password" autocomplete="off" class="conversion-input w-full" placeholder="原包未加密可留空" /></div>
          <div class="space-y-2 text-sm"><label class="flex items-center gap-2"><input v-model="changePassword" type="checkbox" />修改新包密码</label><input v-if="changePassword" v-model="password" type="password" autocomplete="new-password" class="conversion-input w-full" placeholder="留空则新包不加密" /></div>
        </div>
        <div class="space-y-2 text-sm">
          <label>输出目录<ThemedSelect v-model="destination" :options="[{ value: 'keep', label: '保持各自目录' }, { value: 'source', label: '各自源文件目录' }, { value: 'custom', label: '统一输出目录' }]" test-id="conversion-batch-destination" floating /></label>
          <div v-if="destination === 'custom'" class="flex min-w-0 gap-2"><input v-model="directory" class="conversion-input min-w-0 flex-1" placeholder="新包保存目录" /><button type="button" class="conversion-button" @click="chooseDirectory">选择目录</button><SavedLocationPicker @select="directory = $event" /></div>
        </div>
      </template>
    </div>
    <template #footer>
      <button type="button" class="conversion-button" @click="visible = false">取消</button>
      <button type="button" class="conversion-button bg-primary text-white" data-testid="conversion-batch-apply" :disabled="!valid || store.busy" @click="apply">{{ mode === 'selected' ? `应用到 ${store.selected.length} 项` : '保存默认配置' }}</button>
    </template>
  </Modal>
</template>
