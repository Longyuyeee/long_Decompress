<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useAppStore } from '@/stores/app'
import Modal from '@/components/ui/Modal.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import { archiveOpenOptions, type ArchiveOpenBehavior } from '@/utils/archiveOpening'

interface Association { extension: string; registered: boolean; isDefault: boolean }
const app = useAppStore()
const windows = navigator.platform.toLowerCase().includes('win')
const associations = ref<Association[]>([])
const formatsOpen = ref(false)
const busy = ref(false)
const error = ref('')
const loaded = ref(false)
const selected = ref<string[]>([])
const submitted = ref(false)
const notice = ref('')
let refreshGeneration = 0
const options = [{ value: 'ask', label: '每次询问（默认）' }, ...archiveOpenOptions]
const defaultCount = computed(() => associations.value.filter(item => item.isDefault).length)
const status = computed(() => !loaded.value ? '正在读取关联状态…' : defaultCount.value ? `已默认打开 ${defaultCount.value} 种压缩格式` : '尚未设为默认打开应用')
const pending = computed(() => associations.value.filter(item => selected.value.includes(item.extension) !== item.isDefault))
const allSelected = computed(() => associations.value.length > 0 && selected.value.length === associations.value.length)
watch(selected, () => { submitted.value = false; notice.value = '' }, { deep: true, flush: 'sync' })

async function refresh() {
  if (!windows) return false
  const generation = ++refreshGeneration
  try {
    const result = await invoke<Association[]>('archive_file_associations')
    if (!Array.isArray(result) || !result.length || result.some(item => typeof item.extension !== 'string' || typeof item.isDefault !== 'boolean')) throw new Error('暂时无法读取文件关联状态，请重试。')
    if (generation !== refreshGeneration) return false
    associations.value = result
    loaded.value = true
    error.value = ''
    return true
  } catch (cause) {
    if (generation === refreshGeneration) error.value = String(cause)
    return false
  }
}

async function editFormats() {
  if (!await refresh()) return
  // Being registered as a candidate does not mean Windows uses this app by default.
  selected.value = associations.value.filter(item => item.isDefault).map(item => item.extension)
  submitted.value = false
  notice.value = ''
  formatsOpen.value = true
}

async function configure() {
  if (busy.value) return
  busy.value = true
  error.value = ''
  try {
    if (!submitted.value) {
      await invoke('configure_archive_file_associations', { extensions: [...selected.value] })
      submitted.value = true
    }
    if (!await refresh()) return
    if (!pending.value.length) {
      notice.value = '默认打开方式已更新。'
      formatsOpen.value = false
      return
    }
    notice.value = `还有 ${pending.value.length} 种格式需在 Windows 中确认，返回后会自动更新状态。`
    await invoke('open_default_archive_apps')
    // Keep the selection and outstanding changes visible when Settings returns.
  } catch (cause) { error.value = String(cause) }
  finally { busy.value = false }
}

function selectAll() { selected.value = associations.value.map(item => item.extension) }
async function refreshOnFocus() {
  if (!await refresh()) return
  if (formatsOpen.value && submitted.value) notice.value = pending.value.length
    ? `还有 ${pending.value.length} 种格式待确认；勾选的选 Long解压，取消的改选其他应用。`
    : '所选格式已全部生效，可以关闭此窗口。'
}
onMounted(() => { void refresh(); window.addEventListener('focus', refreshOnFocus) })
onUnmounted(() => { refreshGeneration++; window.removeEventListener('focus', refreshOnFocus) })
</script>

<template>
  <section class="archive-opening-settings" data-testid="archive-opening-settings" tabindex="-1">
    <div class="opening-heading">
      <h3>默认打开压缩包</h3>
      <HelpHint label="系统打开压缩包说明">设置 Windows 双击压缩包时使用的应用，以及由 Long解压打开后的操作。右键指定的操作直接执行；软件内双击保持当前栏浏览。</HelpHint>
    </div>
    <div v-if="windows" class="association-row">
      <div><strong>{{ status }}</strong><span v-if="error && !formatsOpen" role="alert">{{ error }}</span></div>
      <button type="button" data-testid="archive-association-configure" :disabled="busy" @click="editFormats">设置默认打开方式…</button>
    </div>
    <p v-if="notice && !formatsOpen" class="association-notice" role="status">{{ notice }}</p>
    <div class="opening-fields">
      <label><span>打开后</span><ThemedSelect :model-value="app.settings.archiveOpenBehavior" :options="options" aria-label="打开压缩包后的操作" test-id="archive-open-behavior" floating @update:model-value="app.updateSettings({ archiveOpenBehavior: $event as ArchiveOpenBehavior })"/></label>
      <label><span>解压完成后</span><ThemedSelect :model-value="app.settings.archiveAfterExtract" :options="[{value:'show-results',label:'在解压中心查看结果'},{value:'open-folder',label:'打开结果文件夹'}]" aria-label="系统解压完成后的操作" test-id="archive-after-extract" floating @update:model-value="app.updateSettings({ archiveAfterExtract: $event as 'show-results' | 'open-folder' })"/></label>
    </div>
    <Modal :visible="formatsOpen" title="设置默认打开方式" description="勾选希望默认用 Long解压打开的格式，取消勾选可更换为其他应用。Windows 要求的默认应用确认会在系统设置中完成；返回后这里只显示实际生效的状态。" description-as-help icon="pi pi-box" size="md" :close-on-backdrop="!busy" :close-on-escape="!busy" @close="!busy && (formatsOpen=false)">
      <div class="association-toolbar">
        <span>已选 <strong>{{ selected.length }}</strong> / {{ associations.length }}</span>
        <div>
          <button type="button" data-testid="archive-association-select-all" :disabled="busy || allSelected" @click="selectAll">全选</button>
          <button type="button" data-testid="archive-association-clear-all" :disabled="busy || !selected.length" @click="selected=[]">全部取消</button>
          <button type="button" class="refresh-associations" aria-label="刷新默认打开状态" title="刷新默认打开状态" :disabled="busy" @click="refreshOnFocus"><i class="pi pi-refresh"/></button>
        </div>
      </div>
      <div class="association-formats" data-testid="archive-association-formats">
        <label v-for="item in associations" :key="item.extension" :class="{ chosen: selected.includes(item.extension) }">
          <input v-model="selected" type="checkbox" :value="item.extension" :disabled="busy" :aria-label="item.extension.slice(1).toUpperCase()"/>
          <strong>{{ item.extension.slice(1).toUpperCase() }}</strong>
          <span v-if="item.isDefault" :class="{ pending: !selected.includes(item.extension) }">{{ selected.includes(item.extension) ? '已默认' : '待更换' }}</span>
          <span v-else-if="submitted && selected.includes(item.extension)" class="pending">待确认</span>
        </label>
      </div>
      <p v-if="notice" class="association-notice" role="status" data-testid="archive-association-notice">{{ notice }}</p>
      <p v-if="error" role="alert" class="association-error">{{ error }}</p>
      <template #footer>
        <span class="association-footer-note">{{ submitted && pending.length ? `待系统确认 ${pending.length} 种` : `当前已默认 ${defaultCount} 种` }}</span>
        <button type="button" class="association-button" :disabled="busy" @click="formatsOpen=false">{{ submitted ? '关闭' : '取消' }}</button>
        <button type="button" class="association-button primary" :disabled="busy" data-testid="archive-association-save" @click="configure">{{ busy ? '正在处理…' : submitted && pending.length ? '继续系统确认' : '确认' }}</button>
      </template>
    </Modal>
  </section>
</template>

<style scoped>
.archive-opening-settings{display:grid;gap:14px;margin-top:20px;padding-top:18px;border-top:1px solid var(--border-subtle);min-width:0}
.opening-heading{display:flex;align-items:center;gap:8px}.opening-heading h3{font-size:13px;font-weight:800;color:var(--text-base)}
.association-row{display:flex;align-items:center;flex-wrap:wrap;gap:8px}.association-row>div{flex:1;min-width:160px}.association-row strong{display:block;font-size:12px;color:var(--text-muted);font-weight:500}
.association-row button,.association-button{padding:8px 12px;border:1px solid var(--border-subtle);border-radius:9px;background:var(--bg-input);font-size:12px;color:var(--text-base);transition:border-color .15s,color .15s}
.association-row button:hover,.association-button:hover{border-color:var(--dynamic-accent);color:var(--dynamic-accent)}
.opening-fields{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}.opening-fields>label{min-width:0}.opening-fields>label>span{display:block;margin-bottom:6px;font-size:12px;color:var(--text-muted)}
.association-toolbar{display:flex;align-items:center;justify-content:space-between;gap:8px;margin-bottom:12px;font-size:12px;color:var(--text-muted)}.association-toolbar strong{color:var(--dynamic-accent)}.association-toolbar>div{display:flex;align-items:center;gap:4px}.association-toolbar button{padding:5px 8px;border-radius:7px;color:var(--dynamic-accent);font-size:12px}.association-toolbar button:hover:not(:disabled){background:var(--bg-input)}.association-toolbar button:disabled{opacity:.4;cursor:default}
.association-formats{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px}.association-formats>label{display:flex;min-width:0;gap:7px;align-items:center;padding:10px 8px;border:1px solid var(--border-subtle);border-radius:9px;background:var(--bg-input);font-size:12px;cursor:pointer;transition:background .15s,border-color .15s}.association-formats>label:hover{border-color:var(--dynamic-accent)}.association-formats>label.chosen{background:color-mix(in srgb,var(--dynamic-accent) 9%,var(--bg-input));border-color:color-mix(in srgb,var(--dynamic-accent) 45%,var(--border-subtle))}.association-formats input{width:15px;height:15px;flex-shrink:0;accent-color:var(--dynamic-accent);cursor:pointer}.association-formats strong{font-weight:700}.association-formats>label>span{margin-left:auto;white-space:nowrap;color:var(--dynamic-accent);font-size:10px}.association-formats>label>span.pending{color:var(--text-muted)}
.association-notice{margin-top:12px;font-size:12px;line-height:1.7;color:var(--text-muted);overflow-wrap:anywhere}.association-error,.association-row [role=alert]{display:block;margin-top:8px;font-size:12px;color:#ef4444;overflow-wrap:anywhere}.association-footer-note{margin-right:auto;font-size:11px;color:var(--text-muted)}.association-button.primary{background:var(--dynamic-accent);border-color:var(--dynamic-accent);color:var(--accent-ink)}.association-button.primary:hover{filter:brightness(1.08);color:var(--accent-ink)}.association-button:disabled{opacity:.5}
@media(max-width:800px){.opening-fields{grid-template-columns:minmax(0,1fr)}}
@media(max-width:520px){.association-formats{grid-template-columns:repeat(2,minmax(0,1fr))}.association-footer-note{flex-basis:100%}}
</style>
