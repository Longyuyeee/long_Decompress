<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import HelpHint from '@/components/ui/HelpHint.vue'
import Modal from '@/components/ui/Modal.vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import SavedLocationPicker from '@/components/ui/SavedLocationPicker.vue'
import { useSavedLocations } from '@/composables/useSavedLocations'
import { useAppStore } from '@/stores/app'
import type { BatchConfigPatch } from '@/types/batchConfig'
import { isZipPasswordEncoding, ZIP_PASSWORD_ENCODING_OPTIONS } from '@/types/zipPassword'

defineOptions({ inheritAttrs: false })
const props = defineProps<{ count: number; mode: 'compression' | 'decompression' }>()
const emit = defineEmits<{ apply: [patch: BatchConfigPatch, complete?: (error?: string) => void]; clear: [] }>()
const app = useAppStore()
const visible = ref(false)
const applyError = ref('')
let editingSession = 0
const format = ref('keep')
const level = ref('keep')
const zipMethod = ref('keep')
const recovery = ref('keep')
const recoveryVolumes = ref('keep')
const subfolder = ref('keep')
const zipPasswordEncoding = ref('keep')
const applying = ref(false)
const destination = ref<'keep' | 'source' | 'custom'>('keep')
const outputDirectory = ref('')
const keep = { value: 'keep', label: '保持各自设置' }
const formats = [keep, { value: 'zip', label: 'ZIP' }, { value: 'zipx', label: 'ZIPX（XZ）' }, { value: '7z', label: '7Z' }, { value: 'rar', label: 'RAR（需要 WinRAR）' }, { value: 'tar.lz', label: 'TAR.LZIP（无加密）' }]
const levels = [keep, ...Array.from({ length: 10 }, (_, value) => ({ value: String(value), label: `${value}${value === 0 ? ' · 最低强度（兼容 ZIP 仅存储）' : value === 9 ? ' · 最小体积' : value === 1 ? ' · 更快' : ''}` }))]
const subfolders = [keep, { value: 'yes', label: '创建同名子文件夹' }, { value: 'no', label: '直接放入输出目录' }]
const hasChanges = computed(() => destination.value !== 'keep' || (props.mode === 'compression'
    ? format.value !== 'keep' || zipMethod.value !== 'keep' || level.value !== 'keep' || recovery.value !== 'keep' || recoveryVolumes.value !== 'keep' : subfolder.value !== 'keep' || zipPasswordEncoding.value !== 'keep'))
const canApply = computed(() => hasChanges.value && (destination.value !== 'custom' || !!outputDirectory.value.trim()))
function openConfiguration() {
  editingSession++
  format.value = level.value = subfolder.value = recovery.value = 'keep'
  recoveryVolumes.value = zipMethod.value = 'keep'
  zipPasswordEncoding.value = 'keep'
  destination.value = 'keep'
  outputDirectory.value = applyError.value = ''
  visible.value = true
}
watch(() => props.count, count => { if (!count) visible.value = false })
function setOutput(path: string) { outputDirectory.value = path; destination.value = 'custom' }
async function chooseOutput() {
  const session = editingSession
  try {
    const path = await open({ directory: true, multiple: false, title: '选择所选任务的输出目录' })
    if (!visible.value || session !== editingSession) return
    if (typeof path === 'string') { setOutput(path); useSavedLocations().remember(path) }
  } catch { app.setError('无法选择输出目录，请重试。') }
}
function apply() {
  if (!canApply.value || applying.value) return
  const patch: BatchConfigPatch = {}
  if (destination.value !== 'keep') patch.outputDirectory = destination.value === 'source' ? '' : outputDirectory.value.trim()
  if (props.mode === 'compression') {
    if (format.value === 'zip' || format.value === 'zipx' || format.value === '7z' || format.value === 'rar' || format.value === 'tar.lz') patch.format = format.value
    if (zipMethod.value === 'zstd' || zipMethod.value === 'deflate') patch.zipMethod = zipMethod.value
    if (level.value !== 'keep') patch.level = Number(level.value)
    if (recovery.value !== 'keep') patch.rarRecoveryPercent = Number(recovery.value)
    if (recoveryVolumes.value !== 'keep') patch.rarRecoveryVolumes = Number(recoveryVolumes.value)
  } else {
    if (subfolder.value !== 'keep') patch.extractToSubfolder = subfolder.value === 'yes'
    if (zipPasswordEncoding.value === 'auto' || isZipPasswordEncoding(zipPasswordEncoding.value)) patch.zipPasswordEncoding = zipPasswordEncoding.value
  }
  applyError.value = ''
  applying.value = true
  emit('apply', patch, error => {
    applying.value = false
    if (error) applyError.value = error
    else visible.value = false
  })
}
</script>

<template>
  <section v-if="count" v-bind="$attrs" class="batch-config-bar min-w-0 rounded-xl border border-primary/25 bg-primary/5 px-3 py-2" data-testid="batch-config-bar">
    <div class="flex flex-wrap items-center gap-2">
      <span class="mr-auto text-sm font-semibold text-content">已选 {{ count }} 项</span>
      <button type="button" class="batch-button bg-primary text-white" :disabled="applying" aria-haspopup="dialog" @click="openConfiguration">
        <i class="pi pi-sliders-h" aria-hidden="true"></i>修改所选配置
      </button>
      <button type="button" class="batch-button text-muted hover:bg-input" @click="emit('clear')">取消选择</button>
    </div>
  </section>
  <Modal v-model:visible="visible" :title="mode === 'compression' ? '修改所选压缩配置' : '修改所选解压配置'" :description="`已选 ${count} 项任务`" icon="pi pi-sliders-h" size="md" cancel-text="关闭批量配置" :show-close-button="!applying" :close-on-backdrop="!applying" :close-on-escape="!applying">
    <fieldset :disabled="applying" class="space-y-4 min-w-0" data-testid="batch-config-fields">
      <p v-if="format === 'tar.lz'" class="text-xs text-muted">TAR.LZIP 不提供密码和分卷，应用到所选任务时会关闭这些选项。</p>
      <p class="text-xs leading-relaxed text-muted">只修改下面指定的项目，其他配置保持各自原值。</p>
      <div class="grid min-w-0 gap-3 sm:grid-cols-2">
        <label v-if="mode === 'compression'" class="min-w-0 space-y-1.5 text-xs text-muted">压缩格式 <HelpHint v-if="format === 'zipx'" label="ZIPX 分卷说明">ZIPX 使用 XZ，分卷采用 .zipx.001 数字卷；标准分卷可选择 ZIP。</HelpHint>
          <ThemedSelect v-model="format" :options="formats" aria-label="批量压缩格式" test-id="batch-format" floating />
        </label>
        <label v-if="mode === 'compression'" class="min-w-0 space-y-1.5 text-xs text-muted">压缩等级
          <ThemedSelect v-model="level" :options="levels" aria-label="批量压缩等级" test-id="batch-level" floating />
        </label>
        <label v-else class="min-w-0 space-y-1.5 text-xs text-muted">文件放置方式
          <ThemedSelect v-model="subfolder" :options="subfolders" aria-label="批量解压子文件夹" test-id="batch-subfolder" floating />
        </label>
      </div>
      <div v-if="mode === 'decompression'" class="min-w-0 space-y-1.5 text-xs text-muted">ZIP 密码编码
        <HelpHint label="批量 ZIP 密码编码说明">仅修改所选任务中已检测为 ZIP/ZIPX 的来源，其他格式保持原值。不修改输出默认值或密码。</HelpHint>
        <ThemedSelect v-model="zipPasswordEncoding" :disabled="applying" :options="[keep, {value:'auto',label:'自动兼容（默认）'}, ...ZIP_PASSWORD_ENCODING_OPTIONS]" aria-label="批量 ZIP 密码编码" test-id="batch-zip-password-encoding" floating />
      </div>
      <label v-if="mode === 'compression' && (format === 'keep' || format === 'zip')" class="block min-w-0 space-y-1.5 text-xs text-muted">ZIP 压缩方法
        <ThemedSelect v-model="zipMethod" :options="[keep, { value: 'deflate', label: '兼容 ZIP · Deflate' }, { value: 'zstd', label: 'Zstandard' }]" test-id="batch-zip-method" floating />
        <span class="block leading-relaxed">只修改所选 ZIP 任务的方法；ZIPX 始终使用 XZ。</span>
      </label>
      <label v-if="mode === 'compression'" class="block min-w-0 space-y-1.5 text-xs text-muted">RAR 分包恢复卷
        <ThemedSelect v-model="recoveryVolumes" :options="[keep, ...[0,1,2,3,5,10].map(value => ({value:String(value),label:value ? `${value} 个 .rev 恢复卷` : '不生成恢复卷'}))]" test-id="batch-rar-recovery-volumes" floating />
        <span class="block leading-relaxed">仅实际生成多个 RAR 分包时生效，分卷设置保持各自原值。</span>
      </label>
      <label v-if="mode === 'compression'" class="block min-w-0 space-y-1.5 text-xs text-muted">RAR 恢复保护
        <ThemedSelect v-model="recovery" :options="[keep, {value:'0',label:'关闭'}, {value:'3',label:'3%'}, {value:'5',label:'5%'}, {value:'10',label:'10%'}]" test-id="batch-rar-recovery" floating />
        <span class="block text-xs leading-relaxed">启用时所选任务需为 RAR；其他配置保持原值。</span>
      </label>
      <div class="min-w-0 space-y-2">
        <p class="text-xs text-muted">输出目录</p>
        <div class="flex flex-wrap items-center gap-2">
          <button v-for="option in [{ value: 'keep', label: '保持各自目录' }, { value: 'source', label: '各自源文件目录' }]" :key="option.value" type="button" class="batch-button border" :class="destination === option.value ? 'border-primary/40 bg-primary/10 text-primary' : 'border-subtle text-muted hover:bg-input'" :aria-pressed="destination === option.value" @click="destination = option.value as 'keep' | 'source'">{{ option.label }}</button>
          <button type="button" class="batch-button border border-subtle text-content hover:bg-input" @click="chooseOutput"><i class="pi pi-folder-open" aria-hidden="true"></i>选择目录</button>
          <SavedLocationPicker :current="destination === 'custom' ? outputDirectory : ''" @select="setOutput" />
        </div>
        <p v-if="destination === 'custom'" class="select-text break-words rounded-lg bg-input/60 px-3 py-2 text-xs text-content [overflow-wrap:anywhere]" :title="outputDirectory">{{ outputDirectory }}</p>
      </div>
      <p v-if="applyError" class="rounded-lg border border-danger/20 bg-danger/5 px-3 py-2 text-xs leading-relaxed text-danger" role="alert">{{ applyError }}</p>
    </fieldset>
    <template #footer>
      <button type="button" class="batch-button border border-subtle text-muted hover:bg-input" data-testid="batch-config-cancel" :disabled="applying" @click="visible = false">取消</button>
      <button type="button" class="batch-button bg-primary font-semibold text-white disabled:opacity-40" data-testid="batch-config-apply" :disabled="!canApply || applying" @click="apply">{{ applying ? '正在应用…' : `应用到 ${count} 项` }}</button>
    </template>
  </Modal>
</template>

<style scoped>
.batch-button { display:inline-flex; min-height:2rem; align-items:center; justify-content:center; gap:.5rem; border-radius:.6rem; padding:.4rem .7rem; font-size:.75rem; transition:background .15s,opacity .15s; }
.batch-button:focus-visible { outline:2px solid var(--dynamic-accent); outline-offset:2px; }
.batch-button i { font-size:.75rem; }
</style>
