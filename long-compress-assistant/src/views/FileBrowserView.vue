<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useCompressionStore } from '@/stores/compression'
import DualPaneFileManager, { type LocalFileEntry } from '@/components/file-manager/DualPaneFileManager.vue'
import ArchiveBrowserView from '@/views/ArchiveBrowserView.vue'

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
  <DualPaneFileManager v-if="mode === 'files'" @open-archive="openArchive" @compress="compress" @extract="extract" />
  <div v-else class="h-full min-w-0 relative">
    <button class="return-files" type="button" data-testid="return-to-file-manager" @click="mode = 'files'"><i class="pi pi-arrow-left"></i><span>返回双栏文件浏览器</span></button>
    <ArchiveBrowserView />
  </div>
</template>

<style scoped>
.return-files{position:absolute;z-index:40;top:1.55rem;right:12.5rem;height:2.85rem;padding:0 .85rem;display:flex;align-items:center;gap:.45rem;border:1px solid var(--border-subtle);border-radius:.9rem;background:var(--bg-input);color:var(--text-muted);font-size:.68rem;font-weight:850}.return-files:hover{border-color:var(--dynamic-accent);color:var(--dynamic-accent)}
</style>
