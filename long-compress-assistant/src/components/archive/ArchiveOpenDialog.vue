<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'
import { useRouter } from 'vue-router'
import Modal from '@/components/ui/Modal.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import { useAppStore } from '@/stores/app'
import { archiveOpenOptions, type ArchiveOpenAction } from '@/utils/archiveOpening'
import { archiveSourceDirectory } from '@/utils/zpaqMultipart'

const app = useAppStore(), router = useRouter()
const busy = ref(false), remember = ref(false), error = ref('')
const destination = ref(''), subfolder = ref('')
let previewRevision = 0
const visible = computed(() => app.settingsLoaded && app.pendingArchiveOpenFiles.length > 0 && app.settings.archiveOpenBehavior === 'ask')
const name = computed(() => app.pendingArchiveOpenFiles[0]?.split(/[\\/]/).pop() || '')
watch(() => app.pendingArchiveOpenFiles[0], async path => {
  const revision = ++previewRevision
  destination.value = ''; subfolder.value = ''; remember.value = false; error.value = ''
  if (!path) return
  try {
    const parent = await archiveSourceDirectory(path)
    const target = await invoke<string>('preview_extraction_output', { filePath: path, outputPath: parent, createSubdirectory: true })
    if (revision === previewRevision) { destination.value = parent; subfolder.value = target }
  } catch { /* The task reports source errors; choosing a destination remains available. */ }
}, { immediate:true })
function cancel() { if (!busy.value) app.pendingArchiveOpenFiles = [] }
async function choose(action: ArchiveOpenAction) {
  if (busy.value || !app.pendingArchiveOpenFiles.length) return
  busy.value = true; error.value = ''
  const files = [...app.pendingArchiveOpenFiles]
  try {
    let outputPath: string | undefined
    if (action === 'choose-folder') {
      const selected = await open({ directory: true, multiple: false, title: '选择解压文件夹', defaultPath: destination.value || undefined })
      if (!selected || Array.isArray(selected)) {
        // A cancelled picker must not be reopened automatically by the saved action.
        if (app.settings.archiveOpenBehavior !== 'ask') app.pendingArchiveOpenFiles = app.pendingArchiveOpenFiles.filter(path => !files.includes(path))
        return
      }
      outputPath = selected
    }
    if (remember.value && app.settings.archiveOpenBehavior === 'ask') app.updateSettings({ archiveOpenBehavior: action })
    app.pendingArchiveOpenFiles = app.pendingArchiveOpenFiles.filter(path => !files.includes(path))
    if (action === 'browse') {
      app.openArchiveInBrowser(files[0]); await router.push('/browser')
      if (files.length > 1) app.setSuccess(`已打开第一个压缩包，其余 ${files.length - 1} 个可从双栏浏览器继续打开。`)
    } else {
      app.enqueueContextAction({
        action: action === 'here' ? 'context-extract-here' : 'context-extract-to', files, outputPath,
        extractToSubfolder: action === 'subfolder' || (action === 'choose-folder' && files.length > 1),
        openResultFolder: app.settings.archiveAfterExtract === 'open-folder',
      })
      await router.push('/decompress')
    }
  } catch (cause) {
    error.value = String(cause)
    if (app.settings.archiveOpenBehavior !== 'ask') { app.pendingArchiveOpenFiles = app.pendingArchiveOpenFiles.filter(path => !files.includes(path)); app.setError(error.value) }
  } finally { busy.value = false }
}
watch([() => app.pendingArchiveOpenFiles.length, () => app.settingsLoaded, () => app.settings.archiveOpenBehavior, busy], () => {
  if (!busy.value && app.settingsLoaded && app.pendingArchiveOpenFiles.length && app.settings.archiveOpenBehavior !== 'ask') void choose(app.settings.archiveOpenBehavior)
}, { immediate: true })
</script>

<template>
  <Modal :visible="visible" title="打开压缩包" icon="pi pi-box" size="sm" :show-footer="false" :close-on-backdrop="!busy" :close-on-escape="!busy" :show-close-button="!busy" @close="cancel">
    <div class="archive-open-dialog" data-testid="archive-open-dialog" :aria-busy="busy">
      <div class="open-source"><strong :title="name">{{ name }}</strong><span v-if="app.pendingArchiveOpenFiles.length > 1">等 {{ app.pendingArchiveOpenFiles.length }} 个压缩包</span></div>
      <button v-for="option in archiveOpenOptions" :key="option.value" type="button" class="open-choice" :disabled="busy" :data-testid="`archive-open-${option.value}`" @click="choose(option.value)">
        <i :class="option.icon" aria-hidden="true"/><span><strong>{{ option.label }}</strong><small v-if="option.value === 'here' && destination" :title="destination">{{ destination }}</small><small v-if="option.value === 'subfolder' && subfolder" :title="subfolder">{{ subfolder }}</small></span><i class="pi pi-angle-right" aria-hidden="true"/>
      </button>
      <div class="remember-choice"><label><input v-model="remember" type="checkbox" :disabled="busy" data-testid="archive-open-remember"/>记住我的选择</label><HelpHint label="默认打开方式说明">仅用于从 Windows 打开压缩包。可在设置 → 行为中恢复“每次询问”；软件内双击仍在当前栏浏览。多包选择同一个输出目录时，各自放入同名文件夹。</HelpHint></div>
      <p v-if="error" role="alert">{{ error }}</p>
    </div>
  </Modal>
</template>

<style scoped>
.archive-open-dialog{display:grid;gap:8px;min-width:0}.open-source{display:flex;align-items:center;gap:8px;min-width:0;margin-bottom:6px;font-size:12px;color:var(--text-muted)}.open-source strong{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--text-base)}.open-source>span{flex-shrink:0}.open-choice{display:flex;width:100%;align-items:center;gap:12px;padding:12px;border:1px solid var(--border-subtle);border-radius:12px;background:var(--bg-input);text-align:left;transition:background .15s,border-color .15s}.open-choice:hover:not(:disabled),.open-choice:focus-visible{border-color:var(--dynamic-accent);background:color-mix(in srgb,var(--dynamic-accent) 8%,var(--bg-input));outline:none}.open-choice>i:first-child{display:grid;place-items:center;width:34px;height:34px;flex-shrink:0;border-radius:9px;background:color-mix(in srgb,var(--dynamic-accent) 10%,transparent);color:var(--dynamic-accent)}.open-choice>i:last-child{margin-left:auto;color:var(--text-muted)}.open-choice>span{min-width:0}.open-choice strong{display:block;font-size:13px;color:var(--text-base)}.open-choice small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;margin-top:4px;color:var(--text-muted);font-size:11px}.open-choice:disabled{opacity:.55}.remember-choice{display:flex;align-items:center;gap:8px;margin-top:8px;font-size:12px;color:var(--text-muted)}.remember-choice label{display:flex;align-items:center;gap:7px}.archive-open-dialog>p{overflow-wrap:anywhere;font-size:12px;color:#ef4444}
</style>
