<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/tauri'
import { open } from '@tauri-apps/api/dialog'
import { useAppStore } from '@/stores/app'
import { useCompressionStore, type ImageCompressionItem } from '@/stores/compression'
import { estimateImageOutputRange, type ImageCandidate, type ImageCompressionSettings } from '@/utils/imageCompressionWorkspace'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import ImageCompressionSettingsPanel from './ImageCompressionSettingsPanel.vue'

const appStore = useAppStore()
const store = useCompressionStore()
const showGlobalSettings = ref(true)
const inspectionsInFlight = new Set<string>()

const formatBytes = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KiB`
  return `${(bytes / 1024 ** 2).toFixed(1)} MiB`
}

const targetFormat = (item: ImageCompressionItem) => {
  const format = store.getEffectiveImageSettings(item).outputFormat
  return format === 'keep' ? item.inputFormat : format
}

const inspectImage = async (item: ImageCompressionItem) => {
  if (inspectionsInFlight.has(item.id) || item.status !== 'inspecting') return
  inspectionsInFlight.add(item.id)
  try {
    await invoke('authorize_image_preview', { path: item.path })
  } catch (error) {
    inspectionsInFlight.delete(item.id)
    store.failImageInspection(item.id, String(error))
    return
  }
  const previewUrl = convertFileSrc(item.path)
  const image = new Image()
  image.onload = () => {
    inspectionsInFlight.delete(item.id)
    store.completeImageInspection(item.id, { width: image.naturalWidth, height: image.naturalHeight, previewUrl })
  }
  image.onerror = () => {
    inspectionsInFlight.delete(item.id)
    store.failImageInspection(item.id, '文件扩展名受支持，但实际图片无法解码')
  }
  image.src = previewUrl
}

watch(
  () => store.imageItems.map(item => `${item.id}:${item.status}`).join('|'),
  () => store.imageItems.filter(item => item.status === 'inspecting').forEach(item => void inspectImage(item)),
  { immediate: true },
)

const onFilesSelected = (candidates: ImageCandidate[]) => {
  const result = store.addImageCandidates(candidates)
  if (result.rejected.length > 0) {
    const sample = result.rejected.slice(0, 2).map(item => `${item.name}：${item.reason}`).join('；')
    appStore.setError(`已拒绝 ${result.rejected.length} 个文件。${sample}`)
  }
}

const updateGlobalSettings = (settings: ImageCompressionSettings) => {
  store.imageGlobalSettings = settings
}

const updateItemSettings = (item: ImageCompressionItem, settings: ImageCompressionSettings) => {
  store.updateImageItemSettings(item.id, settings)
}

const toggleOverride = (item: ImageCompressionItem, enabled: boolean) => {
  if (enabled) store.enableImageItemOverride(item.id)
  else store.disableImageItemOverride(item.id)
}

const chooseOutputDirectory = async (item?: ImageCompressionItem) => {
  try {
    const selected = await open({ directory: true, multiple: false, title: '选择图片输出目录' })
    if (!selected || Array.isArray(selected)) return
    if (item) {
      store.updateImageItemSettings(item.id, { ...store.getEffectiveImageSettings(item), outputDirectory: selected })
    } else {
      store.imageGlobalSettings = { ...store.imageGlobalSettings, outputDirectory: selected }
    }
  } catch (error) {
    appStore.setError(`无法选择输出目录：${String(error)}`)
  }
}

const estimateFor = (item: ImageCompressionItem) => estimateImageOutputRange(item.inputSize, store.getEffectiveImageSettings(item))
const readyItems = computed(() => store.imageItems.filter(item => item.status === 'ready'))
</script>

<template>
  <section class="image-workspace" data-testid="image-compression-workspace">
    <div class="workspace-toolbar">
      <div class="min-w-0">
        <div class="flex items-center gap-2"><i class="pi pi-images text-primary"></i><strong>图片压缩工作区</strong><span class="scope-badge">B-02 前端</span></div>
        <p>支持 JPG、PNG、WebP；GIF 与其他文件会被明确拒绝。</p>
      </div>
      <div class="toolbar-actions">
        <button type="button" class="secondary-action" @click="showGlobalSettings = !showGlobalSettings"><i class="pi pi-sliders-h"></i>批量设置</button>
        <button type="button" class="primary-action" disabled title="B-03 接入真实编码和发布事务后开放"><i class="pi pi-play-circle"></i>开始图片压缩</button>
      </div>
    </div>

    <div class="truth-boundary"><i class="pi pi-info-circle"></i><span>当前步骤只建立真实输入、配置与对比工作区；尚未生成结果文件。执行将在 B-03 验证输出后开放。</span></div>

    <div v-if="showGlobalSettings" class="global-settings-card">
      <div class="settings-heading"><span><i class="pi pi-sparkles"></i>批量全局设置</span><small>单项可在展开后覆盖</small></div>
      <ImageCompressionSettingsPanel :model-value="store.imageGlobalSettings" @update:model-value="updateGlobalSettings" />
      <div class="output-directory">
        <div class="min-w-0"><span>输出目录</span><strong :title="store.imageGlobalSettings.outputDirectory">{{ store.imageGlobalSettings.outputDirectory || '与源文件同目录' }}</strong></div>
        <button type="button" @click="chooseOutputDirectory()"><i class="pi pi-folder-open"></i>选择目录</button>
      </div>
    </div>

    <div v-if="store.imageItems.length === 0" class="image-empty">
      <EnhancedFileDropzone
        mode="file"
        accept="jpg,jpeg,png,webp"
        picker-title="选择图片文件"
        unfiltered-picker
        hint="拖入需要压缩的图片"
        sub-hint="会先读取真实尺寸和格式，不会自动开始处理"
        @files-selected="onFilesSelected"
      />
    </div>

    <div v-else class="image-list-shell">
      <div class="image-list-summary"><span>图片任务 <strong>{{ store.imageItems.length }}</strong></span><span>已读取 <strong>{{ readyItems.length }}</strong></span><button type="button" @click="store.clearImageDrafts"><i class="pi pi-trash"></i>清空</button></div>
      <div class="image-list custom-scrollbar">
        <article v-for="item in store.imageItems" :key="item.id" class="image-task" :class="{ expanded: item.expanded, rejected: item.status === 'rejected' }">
          <button type="button" class="image-row" @click="item.expanded = !item.expanded" :aria-expanded="item.expanded">
            <span class="image-name"><i class="pi pi-image"></i><span><strong :title="item.path">{{ item.name }}</strong><small :title="item.path">{{ item.path }}</small></span></span>
            <span class="image-dimensions">{{ item.width && item.height ? `${item.width} × ${item.height}` : item.status === 'rejected' ? '无法读取' : '读取中…' }}</span>
            <span class="image-size">{{ formatBytes(item.inputSize) }}</span>
            <span class="image-format">{{ targetFormat(item).toUpperCase() }}</span>
            <span class="image-status" :class="item.status"><strong>{{ item.status === 'ready' ? '待处理' : item.status === 'rejected' ? '已拒绝' : '检查中' }}</strong><small>{{ item.progress.toFixed(2) }}%</small></span>
            <span class="image-saving">—<small>实际节省</small></span>
            <span class="row-actions"><i class="pi" :class="item.expanded ? 'pi-chevron-up' : 'pi-chevron-down'"></i></span>
          </button>

          <div v-if="item.error" class="item-error"><i class="pi pi-exclamation-triangle"></i>{{ item.error }}</div>

          <div v-if="item.expanded" class="image-details">
            <div class="item-config custom-scrollbar">
              <div class="settings-heading"><span><i class="pi pi-cog"></i>单项配置</span><label><input type="checkbox" :checked="Boolean(item.settings)" @change="toggleOverride(item, ($event.target as HTMLInputElement).checked)">覆盖全局</label></div>
              <ImageCompressionSettingsPanel :model-value="store.getEffectiveImageSettings(item)" @update:model-value="updateItemSettings(item, $event)" />
              <div class="output-directory">
                <div class="min-w-0"><span>输出目录</span><strong :title="store.getEffectiveImageSettings(item).outputDirectory">{{ store.getEffectiveImageSettings(item).outputDirectory || '与源文件同目录' }}</strong></div>
                <button type="button" @click="chooseOutputDirectory(item)"><i class="pi pi-folder-open"></i>选择目录</button>
              </div>
            </div>

            <div class="comparison-panel custom-scrollbar">
              <div class="settings-heading"><span><i class="pi pi-clone"></i>原图 / 结果图对比</span><small>结果须经 B-03 真实编码验证</small></div>
              <div class="comparison-grid">
                <div class="preview-card">
                  <span>原图</span>
                  <img v-if="item.previewUrl" :src="item.previewUrl" :alt="item.name">
                  <div v-else class="preview-placeholder"><i class="pi pi-spin pi-spinner"></i>正在读取</div>
                  <small>{{ item.width || '—' }} × {{ item.height || '—' }} · {{ formatBytes(item.inputSize) }}</small>
                </div>
                <div class="preview-card result-pending">
                  <span>结果图</span>
                  <div class="preview-placeholder"><i class="pi pi-lock"></i>B-03 实际编码后显示</div>
                  <small>不会使用原图伪装结果</small>
                </div>
              </div>
              <div v-if="estimateFor(item)" class="estimate-card">
                <span><i class="pi pi-chart-bar"></i>前端参考区间（非实际结果）</span>
                <strong>{{ formatBytes(estimateFor(item)!.minimum) }} – {{ formatBytes(estimateFor(item)!.maximum) }}</strong>
                <small>仅按输入大小、压缩方式与质量给出范围；实际大小、节省比例与结果预览必须由 B-03 编码并复核后写入。</small>
              </div>
              <button type="button" class="remove-item" @click="store.removeImageItem(item.id)"><i class="pi pi-times"></i>移除此图片</button>
            </div>
          </div>
        </article>
      </div>
      <EnhancedFileDropzone compact mode="file" accept="jpg,jpeg,png,webp" picker-title="选择图片文件" unfiltered-picker hint="继续添加图片" :native-drop="false" @files-selected="onFilesSelected" />
    </div>
  </section>
</template>

<style scoped>
.image-workspace{display:flex;min-width:0;min-height:0;flex:1;flex-direction:column;gap:.75rem;overflow:hidden}
.workspace-toolbar{display:flex;align-items:center;justify-content:space-between;gap:1rem}.workspace-toolbar strong{color:var(--text-content);font-size:.95rem}.workspace-toolbar p{margin-top:.25rem;color:var(--text-muted);font-size:.72rem;font-weight:650}.scope-badge{border:1px solid color-mix(in srgb,var(--dynamic-accent) 25%,transparent);border-radius:999px;background:color-mix(in srgb,var(--dynamic-accent) 9%,transparent);padding:.18rem .5rem;color:var(--dynamic-accent);font-size:.62rem;font-weight:900}.toolbar-actions{display:flex;gap:.5rem;flex-shrink:0}.toolbar-actions button,.output-directory button{display:flex;align-items:center;justify-content:center;gap:.4rem;height:2.5rem;border-radius:.8rem;padding:0 .9rem;font-size:.72rem;font-weight:900}.secondary-action,.output-directory button{border:1px solid var(--border-subtle);background:var(--bg-input);color:var(--text-content)}.primary-action{border:0;background:var(--dynamic-accent);color:white}.primary-action:disabled{cursor:not-allowed;filter:saturate(.25);opacity:.5}.truth-boundary{display:flex;align-items:flex-start;gap:.5rem;border:1px solid color-mix(in srgb,var(--dynamic-accent) 22%,transparent);border-radius:.8rem;background:color-mix(in srgb,var(--dynamic-accent) 6%,transparent);padding:.55rem .75rem;color:var(--text-muted);font-size:.68rem;font-weight:700;line-height:1.45}.truth-boundary i{margin-top:.1rem;color:var(--dynamic-accent)}
.global-settings-card{flex-shrink:0;border:1px solid var(--border-subtle);border-radius:1rem;background:color-mix(in srgb,var(--bg-card) 82%,transparent);padding:.8rem;box-shadow:0 12px 30px -24px #000}.settings-heading{display:flex;align-items:center;justify-content:space-between;gap:.75rem;margin-bottom:.7rem;color:var(--text-content);font-size:.73rem;font-weight:900}.settings-heading span{display:flex;align-items:center;gap:.45rem}.settings-heading i{color:var(--dynamic-accent)}.settings-heading small,.settings-heading label{color:var(--text-muted);font-size:.65rem;font-weight:750}.settings-heading label{display:flex;align-items:center;gap:.35rem}.settings-heading input{accent-color:var(--dynamic-accent)}.output-directory{display:flex;align-items:center;justify-content:space-between;gap:.75rem;margin-top:.7rem;border:1px solid var(--border-subtle);border-radius:.8rem;background:color-mix(in srgb,var(--bg-input) 65%,transparent);padding:.55rem .65rem}.output-directory>div{display:flex;min-width:0;flex-direction:column;gap:.15rem}.output-directory span{color:var(--text-muted);font-size:.62rem;font-weight:800}.output-directory strong{overflow:hidden;color:var(--text-content);font-size:.72rem;text-overflow:ellipsis;white-space:nowrap}.output-directory button{height:2.1rem;flex-shrink:0}
.image-empty{display:flex;min-height:0;flex:1}.image-empty :deep(.drop-area){display:flex;min-height:15rem;width:100%;align-items:center;justify-content:center}.image-list-shell{display:flex;min-height:0;flex:1;flex-direction:column;gap:.55rem}.image-list-summary{display:flex;align-items:center;gap:1rem;color:var(--text-muted);font-size:.68rem;font-weight:750}.image-list-summary strong{color:var(--dynamic-accent)}.image-list-summary button{margin-left:auto;color:var(--text-muted)}.image-list{display:flex;min-height:0;flex:1;flex-direction:column;gap:.55rem;overflow-x:hidden;overflow-y:auto;padding-right:.25rem}.image-task{max-width:100%;min-width:0;overflow:hidden;border:1px solid var(--border-subtle);border-radius:1rem;background:color-mix(in srgb,var(--bg-card) 76%,transparent);transition:.2s ease}.image-task.expanded{border-color:color-mix(in srgb,var(--dynamic-accent) 38%,transparent);box-shadow:0 18px 34px -28px #000}.image-task.rejected{border-color:color-mix(in srgb,#ef4444 35%,transparent)}
.image-row{display:grid;width:100%;min-width:0;grid-template-columns:minmax(9rem,1.5fr) minmax(6rem,.62fr) minmax(5rem,.48fr) minmax(4.5rem,.42fr) minmax(5.2rem,.52fr) minmax(4.7rem,.46fr) 1.5rem;align-items:center;gap:.65rem;padding:.75rem;text-align:left}.image-name{display:flex;min-width:0;align-items:center;gap:.65rem}.image-name>i{display:flex;width:2rem;height:2rem;flex-shrink:0;align-items:center;justify-content:center;border:1px solid color-mix(in srgb,var(--dynamic-accent) 20%,transparent);border-radius:.65rem;background:color-mix(in srgb,var(--dynamic-accent) 8%,transparent);color:var(--dynamic-accent)}.image-name>span{display:flex;min-width:0;flex-direction:column}.image-name strong,.image-name small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.image-name strong{color:var(--text-content);font-size:.74rem}.image-name small,.image-status small,.image-saving small{color:var(--text-muted);font-size:.6rem}.image-dimensions,.image-size,.image-format{overflow:hidden;color:var(--text-muted);font-size:.68rem;font-weight:800;text-overflow:ellipsis;white-space:nowrap}.image-format{color:var(--dynamic-accent)}.image-status,.image-saving{display:flex;min-width:0;flex-direction:column;gap:.12rem;color:var(--text-content);font-size:.68rem}.image-status.ready strong{color:var(--dynamic-accent)}.image-status.rejected strong{color:#ef4444}.row-actions{color:var(--text-muted)}.item-error{display:flex;align-items:center;gap:.5rem;border-top:1px solid color-mix(in srgb,#ef4444 18%,transparent);padding:.55rem .8rem;color:#ef4444;font-size:.68rem;font-weight:800}
.image-details{display:grid;min-width:0;height:clamp(20rem,48vh,29rem);grid-template-columns:minmax(0,.9fr) minmax(0,1.1fr);overflow:hidden;border-top:1px solid var(--border-subtle)}.item-config,.comparison-panel{min-width:0;min-height:0;overflow-x:hidden;overflow-y:auto;padding:1rem}.item-config{border-right:1px solid var(--border-subtle)}.comparison-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:.65rem}.preview-card{display:flex;min-width:0;flex-direction:column;gap:.45rem;border:1px solid var(--border-subtle);border-radius:.85rem;background:var(--bg-input);padding:.55rem}.preview-card>span{color:var(--text-content);font-size:.67rem;font-weight:900}.preview-card img,.preview-placeholder{width:100%;height:8rem;border-radius:.65rem;background:color-mix(in srgb,var(--bg-base) 65%,transparent);object-fit:contain}.preview-placeholder{display:flex;align-items:center;justify-content:center;gap:.4rem;color:var(--text-muted);font-size:.65rem;font-weight:750}.preview-card small{overflow:hidden;color:var(--text-muted);font-size:.6rem;text-overflow:ellipsis;white-space:nowrap}.result-pending{border-style:dashed}.estimate-card{display:flex;flex-direction:column;gap:.35rem;margin-top:.75rem;border:1px solid color-mix(in srgb,var(--dynamic-accent) 24%,transparent);border-radius:.85rem;background:color-mix(in srgb,var(--dynamic-accent) 6%,transparent);padding:.75rem}.estimate-card span{color:var(--text-content);font-size:.67rem;font-weight:900}.estimate-card strong{color:var(--dynamic-accent);font-size:1rem}.estimate-card small{color:var(--text-muted);font-size:.62rem;font-weight:650;line-height:1.5}.remove-item{margin-top:.7rem;color:#ef4444;font-size:.67rem;font-weight:850}.remove-item i{margin-right:.35rem}
@media(max-width:900px){.image-row{grid-template-columns:minmax(8rem,1fr) minmax(5.5rem,.5fr) minmax(4.5rem,.4fr) minmax(5rem,.45fr) 1.5rem}.image-size,.image-format{display:none}.workspace-toolbar{align-items:flex-start;flex-direction:column}.toolbar-actions{width:100%}.toolbar-actions button{flex:1}}
@media(max-width:620px){.global-settings-card{max-height:16rem;overflow-y:auto}.image-row{grid-template-columns:minmax(0,1fr) minmax(4.5rem,.42fr) 1.5rem}.image-dimensions,.image-saving{display:none}.image-details{grid-template-columns:minmax(0,1fr) minmax(0,1fr)}.comparison-grid{grid-template-columns:minmax(0,1fr)}.preview-card img,.preview-placeholder{height:6rem}.item-config,.comparison-panel{padding:.7rem}}
</style>
