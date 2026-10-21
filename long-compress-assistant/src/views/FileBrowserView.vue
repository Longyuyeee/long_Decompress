<script setup lang="ts">
import { ref, watch } from 'vue'
import {useArchiveReadingStore} from '@/stores/archiveReading'
import type {ArchiveReadingRecord} from '@/types/archiveReading'
import Modal from '@/components/ui/Modal.vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useCompressionStore } from '@/stores/compression'
import DualPaneFileManager, { type LocalFileEntry } from '@/components/file-manager/DualPaneFileManager.vue'
import ArchiveBrowserView from '@/views/ArchiveBrowserView.vue'

const reading=useArchiveReadingStore(),readingVisible=ref(false)
const continueReading=(row:ArchiveReadingRecord)=>{readingVisible.value=false;appStore.openArchiveInBrowser(row.filePath,{focusEntryPath:row.path,entryAction:'read',resumeReading:true,zpaqSnapshot:row.snapshot,zipPasswordEncoding:row.encoding});mode.value='archive'}
const router = useRouter()
const appStore = useAppStore()
const compressionStore = useCompressionStore()
const mode = ref<'files' | 'archive'>(appStore.pendingArchiveBrowserPath ? 'archive' : 'files')

watch(() => appStore.pendingArchiveBrowserPath, path => { if (path) mode.value = 'archive' })

const openArchive = (path: string) => {
  appStore.openArchiveInBrowser(path)
  mode.value = 'archive'
}

const compress = (entries: LocalFileEntry[], destination: string) => {
  const baseName = entries.length === 1 ? entries[0].name.replace(/\.[^/.]+$/, '') : 'archive'
  compressionStore.addQuickPack(entries.map(entry => ({
    name: entry.name, path: entry.path, size: entry.size, type: entry.isDir ? 'folder' : 'file', isDirectory: entry.isDir,
  })), baseName || 'archive', destination)
  void router.push('/compress')
  appStore.setSuccess(`已将 ${entries.length} 项送入压缩任务，输出目录为另一栏`)
}

const extract = (paths: string[], destination: string) => {
  appStore.enqueueContextAction({ action: 'context-extract-other-pane', files: paths, outputPath: destination })
  void router.push('/decompress')
}
</script>

<template>
  <DualPaneFileManager v-if="mode === 'files'" @open-archive="openArchive" @compress="compress" @extract="extract"><template #header-actions><button v-if="reading.records.length" class="continue-reading" data-testid="reading-history-open" @click="readingVisible=true"><i class="pi pi-bookmark"/>继续阅读</button></template></DualPaneFileManager>
  <Modal :visible="readingVisible" title="继续上次阅读" icon="pi pi-bookmark" size="lg" :show-footer="false" @update:visible="readingVisible=$event"><div class="reading-shelf" data-testid="file-manager-reading-history"><p>仅保留本机阅读位置与明确所选图片，密码需要时重新填写。</p><article v-for="row in reading.records" :key="row.filePath+String(row.snapshot?.version||'')"><button class="reading-open" data-testid="reading-recent-open" :title="row.filePath" @click="continueReading(row)"><strong>{{ row.filePath.replace(/\\/g,'/').split('/').pop() }}<small v-if="row.snapshot"> · 快照 {{ row.snapshot.version }}</small></strong><span>{{ row.path }}</span><small>{{ row.selected.length?'已选 '+row.selected.length+' 张 · ':'' }}{{ row.scope==='archive'?'整包图片':'当前目录' }}</small></button><button class="reading-remove" aria-label="移除此阅读记录" @click="reading.remove(row)"><i class="pi pi-times"/></button></article><p v-if="!reading.records.length">阅读记录已清空，可以直接打开新的压缩包。</p></div></Modal>
  <div v-if="mode === 'archive'" class="h-full min-w-0 relative">
    <ArchiveBrowserView>
      <template #header-actions>
        <button class="return-files" type="button" data-testid="return-to-file-manager" @click="mode = 'files'"><i class="pi pi-arrow-left"></i><span>返回双栏文件浏览器</span></button>
      </template>
    </ArchiveBrowserView>
  </div>
</template>

<style scoped>
.return-files{min-height:2.85rem;padding:.5rem .85rem;display:flex;align-items:center;gap:.45rem;border:1px solid var(--border-subtle);border-radius:.9rem;background:var(--bg-input);color:var(--text-muted);font-size:.68rem;font-weight:850}.return-files:hover{border-color:var(--dynamic-accent);color:var(--dynamic-accent)}
.continue-reading{display:inline-flex;align-items:center;justify-content:center;gap:7px;padding:9px 13px;border:1px solid var(--dynamic-accent);border-radius:11px;color:var(--dynamic-accent);font-size:12px;font-weight:800;white-space:nowrap;background:var(--bg-card)}.reading-shelf{display:flex;flex-direction:column;gap:10px}.reading-shelf>p{font-size:12px;line-height:1.8;color:var(--text-muted);margin-bottom:4px}.reading-shelf article{display:flex;align-items:center;gap:12px;border:1px solid var(--border-subtle);border-radius:13px;padding:13px 15px;background:var(--bg-card)}.reading-open{min-width:0;flex:1;text-align:left;font-size:13px}.reading-open strong,.reading-open span{display:block;overflow-wrap:anywhere;line-height:1.7}.reading-open span{font-size:12px;color:var(--text-muted);margin-top:3px}.reading-open small{font-size:10px;color:var(--dynamic-accent)}.reading-open:hover strong{color:var(--dynamic-accent)}.reading-remove{font-size:11px;padding:6px;color:var(--text-muted)}
</style>
