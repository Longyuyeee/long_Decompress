<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useAppStore } from '@/stores/app'
import Modal from '@/components/ui/Modal.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import { archiveOpenOptions, type ArchiveOpenBehavior } from '@/utils/archiveOpening'
interface Association { extension: string; registered: boolean; isDefault: boolean }
const app = useAppStore()
const windows = navigator.platform.toLowerCase().includes('win')
const associations = ref<Association[]>([]), formatsOpen = ref(false), busy = ref(false), error = ref(''), loaded = ref(false)
const selected = ref<string[]>([])
const options = [{ value: 'ask', label: '每次询问（默认）' }, ...archiveOpenOptions]
const defaultCount = computed(() => associations.value.filter(item => item.isDefault).length)
const status = computed(() => !loaded.value ? '正在读取关联状态…' : defaultCount.value ? `已默认打开 ${defaultCount.value} 种压缩格式` : '尚未设为默认打开应用')
async function refresh() {
  if (!windows) return
  try { const result = await invoke<Association[]>('archive_file_associations'); if(!Array.isArray(result))throw new Error('暂时无法读取文件关联状态，请重试。');associations.value = result; loaded.value = true }
  catch (cause) { error.value = String(cause) }
}
async function editFormats() {
  await refresh()
  const configured=associations.value.some(item=>item.registered)
  selected.value = associations.value.filter(item => item.registered || item.isDefault || !configured && ['.zip', '.7z', '.rar', '.zipx', '.tar', '.gz', '.bz2', '.xz'].includes(item.extension)).map(item => item.extension)
  formatsOpen.value = true
}
async function configure() {
  if (busy.value || !selected.value.length) return
  busy.value = true; error.value = ''
  try {
    await invoke('configure_archive_file_associations', { extensions: selected.value })
    await invoke('open_default_archive_apps')
    formatsOpen.value = false
    await refresh()
  } catch (cause) { error.value = String(cause) }
  finally { busy.value = false }
}
async function changeDefault() {
  error.value = ''
  try { await invoke('open_default_archive_apps') } catch (cause) { error.value = String(cause) }
}
onMounted(() => { void refresh(); window.addEventListener('focus', refresh) })
onUnmounted(() => window.removeEventListener('focus', refresh))
</script>

<template>
  <section class="archive-opening-settings" data-testid="archive-opening-settings" tabindex="-1">
    <div class="opening-heading"><h3>默认打开压缩包</h3><HelpHint label="系统打开压缩包说明">从 Windows 双击、按 Enter 或通过“打开方式”打开压缩包时使用下面的行为。右键明确指定的操作直接执行；软件内双击保持当前栏浏览。</HelpHint></div>
    <div v-if="windows" class="association-row"><div><strong>{{ status }}</strong><span v-if="error && !formatsOpen" role="alert">{{ error }}</span></div><button type="button" data-testid="archive-association-configure" @click="editFormats">设置默认打开方式…</button><button v-if="defaultCount" type="button" @click="changeDefault">更改 / 取消默认…</button></div>
    <p v-if="windows && loaded && !defaultCount" class="association-note">请先在 Windows 设置中将 Long解压选为默认应用；下面的选项只决定打开后的操作。</p>
    <div class="opening-fields">
      <label><span>打开后</span><ThemedSelect :model-value="app.settings.archiveOpenBehavior" :options="options" aria-label="打开压缩包后的操作" test-id="archive-open-behavior" floating @update:model-value="app.updateSettings({ archiveOpenBehavior: $event as ArchiveOpenBehavior })"/></label>
      <label><span>解压完成后</span><ThemedSelect :model-value="app.settings.archiveAfterExtract" :options="[{value:'show-results',label:'在解压中心查看结果'},{value:'open-folder',label:'打开结果文件夹'}]" aria-label="系统解压完成后的操作" test-id="archive-after-extract" floating @update:model-value="app.updateSettings({ archiveAfterExtract: $event as 'show-results' | 'open-folder' })"/></label>
    </div>
    <Modal :visible="formatsOpen" title="设置默认打开方式" icon="pi pi-box" size="md" :close-on-backdrop="!busy" :close-on-escape="!busy" @close="!busy && (formatsOpen=false)">
      <div class="association-formats" data-testid="archive-association-formats"><label v-for="item in associations" :key="item.extension" :title="item.isDefault?'当前默认使用 Long解压；取消默认请到 Windows 设置更改':''"><input v-model="selected" type="checkbox" :value="item.extension" :disabled="busy || item.isDefault"/><strong>{{ item.extension.slice(1).toUpperCase() }}</strong><span v-if="item.isDefault">已默认</span></label></div>
      <p class="association-note">默认应用需在 Windows 设置中确认。</p>
      <p v-if="error" role="alert" class="association-error">{{ error }}</p>
      <template #footer><button type="button" class="association-button" :disabled="busy" @click="formatsOpen=false">取消</button><button type="button" class="association-button primary" :disabled="busy || !selected.length" data-testid="archive-association-save" @click="configure">{{ busy ? '正在准备…' : '前往 Windows 设置' }}</button></template>
    </Modal>
  </section>
</template>

<style scoped>
.archive-opening-settings{display:grid;gap:14px;margin-top:20px;padding-top:18px;border-top:1px solid var(--border-subtle);min-width:0}.opening-heading{display:flex;align-items:center;gap:8px}.opening-heading h3{font-size:13px;font-weight:800;color:var(--text-base)}.association-row{display:flex;align-items:center;flex-wrap:wrap;gap:8px}.association-row>div{flex:1;min-width:160px}.association-row strong{display:block;font-size:12px;color:var(--text-muted);font-weight:500}.association-row button,.association-button{padding:8px 10px;border:1px solid var(--border-subtle);border-radius:9px;background:var(--bg-input);font-size:12px;color:var(--text-base)}.association-row button:hover{border-color:var(--dynamic-accent);color:var(--dynamic-accent)}.opening-fields{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}.opening-fields>label{min-width:0}.opening-fields>label>span{display:block;margin-bottom:6px;font-size:12px;color:var(--text-muted)}.association-formats{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px}.association-formats>label{display:flex;gap:7px;align-items:center;padding:10px;border:1px solid var(--border-subtle);border-radius:9px;background:var(--bg-input);font-size:12px}.association-formats>label>span{margin-left:auto;color:var(--dynamic-accent);font-size:10px}.association-note{margin-top:14px;font-size:12px;line-height:1.7;color:var(--text-muted)}.association-error,.association-row [role=alert]{display:block;margin-top:8px;font-size:12px;color:#ef4444;overflow-wrap:anywhere}.association-button.primary{background:var(--dynamic-accent);color:var(--accent-ink)}.association-button:disabled{opacity:.5}@media(max-width:800px){.opening-fields{grid-template-columns:minmax(0,1fr)}.association-formats{grid-template-columns:repeat(2,minmax(0,1fr))}}
</style>
