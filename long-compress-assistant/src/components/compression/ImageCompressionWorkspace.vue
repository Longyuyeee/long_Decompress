<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/tauri'
import { open } from '@tauri-apps/api/dialog'
import { useAppStore } from '@/stores/app'
import { useCompressionStore, type ImageCompressionItem } from '@/stores/compression'
import { useTaskStore, type Task } from '@/stores/task'
import { useImageCompressionBatch } from '@/composables/useImageCompressionBatch'
import {
  estimateImageOutputRange,
  resolveImageCompressionMode,
  type ImageBatchItemResult,
  type ImageCandidate,
  type ImageCompressionSettings,
} from '@/utils/imageCompressionWorkspace'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import Modal from '@/components/ui/Modal.vue'
import ImageCompressionSettingsPanel from './ImageCompressionSettingsPanel.vue'

const appStore = useAppStore()
const store = useCompressionStore()
const taskStore = useTaskStore()
const imageBatch = useImageCompressionBatch()
const showGlobalSettings = ref(false)
const imageSettingsDraft = ref<ImageCompressionSettings>({ ...store.imageGlobalSettings })
const isRunning = ref(false)
const batchSettled = ref(0)
const batchTotal = ref(0)
const batchPercentage = ref(0)
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

const openGlobalSettings = () => {
  imageSettingsDraft.value = { ...store.imageGlobalSettings }
  showGlobalSettings.value = true
}
const updateGlobalSettings = (settings: ImageCompressionSettings) => { imageSettingsDraft.value = settings }
const saveGlobalSettings = () => {
  store.imageGlobalSettings = { ...imageSettingsDraft.value }
  showGlobalSettings.value = false
}
const cancelGlobalSettings = () => {
  imageSettingsDraft.value = { ...store.imageGlobalSettings }
  showGlobalSettings.value = false
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
      imageSettingsDraft.value = { ...imageSettingsDraft.value, outputDirectory: selected }
    }
  } catch (error) {
    appStore.setError(`无法选择输出目录：${String(error)}`)
  }
}

const estimateFor = (item: ImageCompressionItem) => {
  const settings = store.getEffectiveImageSettings(item)
  return estimateImageOutputRange(item.inputSize, {
    ...settings,
    mode: resolveImageCompressionMode(item.inputFormat, settings),
  })
}
const readyItems = computed(() => store.imageItems.filter(item => item.status === 'ready'))
const taskForItem = (item: ImageCompressionItem): Task | undefined =>
  item.taskId ? taskStore.tasks.find(task => task.id === item.taskId) : undefined
const canRetryTask = (task?: Task) => !task
  || task.status === 'failed'
  || task.status === 'cancelled'
  || (task.status === 'completed' && !task.metrics)
const runnableItems = computed(() => readyItems.value.filter(item => canRetryTask(taskForItem(item))))
const canStart = computed(() => !isRunning.value && runnableItems.value.length > 0)

const imageStatus = (item: ImageCompressionItem) => {
  if (item.status === 'rejected') return '已拒绝'
  if (item.status === 'inspecting') return '检查中'
  const task = taskForItem(item)
  if (!task) return '待处理'
  if (task.status === 'completed') return task.metrics ? '已完成' : '已跳过'
  const labels: Partial<Record<Task['status'], string>> = {
    pending: '等待中',
    preparing: '准备中',
    running: '处理中',
    compressing: '压缩中',
    finalizing: '正在收尾',
    cancelling: '正在取消',
    failed: '失败',
    cancelled: '已取消',
  }
  return labels[task.status] || task.status
}

const imageStatusClass = (item: ImageCompressionItem) => taskForItem(item)?.status || item.status
const resultFacts = (item: ImageCompressionItem) => taskForItem(item)?.metrics?.media?.image?.output
const actualSaving = (item: ImageCompressionItem) => {
  const metrics = taskForItem(item)?.metrics
  if (!metrics) return null
  return metrics.inputBytes - metrics.outputBytes
}
const savingLabel = (item: ImageCompressionItem) => {
  const saving = actualSaving(item)
  if (saving === null) return '—'
  if (saving < 0) return `增加 ${formatBytes(Math.abs(saving))}`
  return `节省 ${formatBytes(saving)}`
}

const authorizeResultPreview = async (result: ImageBatchItemResult) => {
  if (result.status !== 'published' && result.status !== 'kept-source-because-output-was-not-smaller') return
  const item = store.imageItems.find(candidate => candidate.id === result.itemId)
  const task = item ? taskForItem(item) : undefined
  if (!item || !task?.outputPath) return
  try {
    await invoke('authorize_image_preview', { path: task.outputPath })
    store.setImageResultPreview(item.id, convertFileSrc(task.outputPath))
  } catch (error) {
    appStore.setError(`结果文件已生成，但无法加载预览：${String(error)}`)
  }
}

const startImageCompression = async () => {
  const items = runnableItems.value
  if (items.length === 0 || isRunning.value) return
  isRunning.value = true
  batchSettled.value = 0
  batchTotal.value = items.length
  batchPercentage.value = 0
  const previews: Promise<void>[] = []
  try {
    const results = await imageBatch.runImageBatch(
      items.map(item => ({
        id: item.id,
        name: item.name,
        path: item.path,
        inputFormat: item.inputFormat,
        settings: { ...store.getEffectiveImageSettings(item) },
      })),
      progress => {
        batchSettled.value = progress.settled
        batchTotal.value = progress.total
        batchPercentage.value = progress.percentage
        previews.push(authorizeResultPreview(progress.result))
      },
      undefined,
      (itemId, taskId) => store.bindImageItemTask(itemId, taskId),
    )
    await Promise.all(previews)
    const completed = results.filter(result => result.status === 'published'
      || result.status === 'kept-source-because-output-was-not-smaller').length
    const skipped = results.filter(result => result.status === 'skipped').length
    const failed = results.filter(result => result.status === 'failed').length
    const cancelled = results.filter(result => result.status === 'cancelled').length
    const summary = `图片处理完成：${completed} 个结果，${skipped} 个跳过，${failed} 个失败，${cancelled} 个取消`
    if (failed > 0 || cancelled > 0) appStore.setError(summary)
    else appStore.setSuccess(summary)
  } catch (error) {
    appStore.setError(`图片批量处理失败：${String(error)}`)
  } finally {
    isRunning.value = false
  }
}

const cancelImageCompression = async () => {
  await imageBatch.cancelImageBatch()
}

const openResultLocation = async (item: ImageCompressionItem) => {
  const path = taskForItem(item)?.outputPath
  if (!path) return
  try {
    await invoke('open_in_explorer', { path })
  } catch (error) {
    appStore.setError(`无法打开结果位置：${String(error)}`)
  }
}
</script>

<template>
  <section class="image-workspace" data-testid="image-compression-workspace">
    <div class="workspace-toolbar">
      <div class="toolbar-actions">
        <button type="button" class="secondary-action" :aria-expanded="showGlobalSettings" data-testid="image-toggle-global-settings" @click="openGlobalSettings"><i class="pi pi-sliders-h"></i>批量设置</button>
        <button v-if="isRunning" type="button" class="danger-action" @click="cancelImageCompression"><i class="pi pi-stop-circle"></i>取消图片压缩</button>
        <button v-else type="button" class="primary-action" :disabled="!canStart" :title="canStart ? '开始处理待处理或可重试图片' : '请先添加并读取可处理图片'" @click="startImageCompression"><i class="pi pi-play-circle"></i>开始图片压缩</button>
      </div>
    </div>

    <div v-if="batchTotal" class="batch-progress"><i class="pi pi-spin pi-spinner"></i><span>已完成 {{ batchSettled }}/{{ batchTotal }}</span><strong>{{ batchPercentage.toFixed(2) }}%</strong></div>

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
      <div class="image-list-summary"><span>图片任务 <strong>{{ store.imageItems.length }}</strong></span><span>已读取 <strong>{{ readyItems.length }}</strong></span><button type="button" :disabled="isRunning" @click="store.clearImageDrafts"><i class="pi pi-trash"></i>清空</button></div>
      <div class="image-list custom-scrollbar">
        <article v-for="item in store.imageItems" :key="item.id" class="image-task" :class="{ expanded: item.expanded, rejected: item.status === 'rejected' }">
          <button type="button" class="image-row" @click="item.expanded = !item.expanded" :aria-expanded="item.expanded">
            <span class="image-name"><i class="pi pi-image"></i><span><strong :title="item.path">{{ item.name }}</strong><small :title="item.path">{{ item.path }}</small></span></span>
            <span class="image-dimensions">{{ resultFacts(item) ? `${resultFacts(item)!.visibleWidth} × ${resultFacts(item)!.visibleHeight}` : item.width && item.height ? `${item.width} × ${item.height}` : item.status === 'rejected' ? '无法读取' : '读取中…' }}</span>
            <span class="image-size">{{ taskForItem(item)?.metrics ? formatBytes(taskForItem(item)!.metrics!.outputBytes) : formatBytes(item.inputSize) }}</span>
            <span class="image-format">{{ (resultFacts(item)?.format || targetFormat(item)).toUpperCase() }}</span>
            <span class="image-status" :class="imageStatusClass(item)"><strong>{{ imageStatus(item) }}</strong><small>{{ taskForItem(item)?.logs.at(-1)?.message || (item.status === 'ready' ? '等待真实执行' : `${item.progress.toFixed(2)}%`) }}</small></span>
            <span class="image-saving"><strong>{{ savingLabel(item) }}</strong><small>实际字节差</small></span>
            <span class="row-actions"><i class="pi" :class="item.expanded ? 'pi-chevron-up' : 'pi-chevron-down'"></i></span>
          </button>

          <div v-if="item.error" class="item-error"><i class="pi pi-exclamation-triangle"></i>{{ item.error }}</div>

          <div v-if="item.expanded" class="image-details">
            <div class="item-config custom-scrollbar" :class="{ locked: isRunning || Boolean(taskForItem(item)?.metrics) }">
              <div class="settings-heading"><span><i class="pi pi-cog"></i>单项配置</span><label><input type="checkbox" :checked="Boolean(item.settings)" @change="toggleOverride(item, ($event.target as HTMLInputElement).checked)">覆盖全局</label></div>
              <ImageCompressionSettingsPanel :model-value="store.getEffectiveImageSettings(item)" @update:model-value="updateItemSettings(item, $event)" />
              <div class="output-directory">
                <div class="min-w-0"><span>输出目录</span><strong :title="store.getEffectiveImageSettings(item).outputDirectory">{{ store.getEffectiveImageSettings(item).outputDirectory || '与源文件同目录' }}</strong></div>
                <button type="button" @click="chooseOutputDirectory(item)"><i class="pi pi-folder-open"></i>选择目录</button>
              </div>
            </div>

            <div class="comparison-panel custom-scrollbar">
              <div class="settings-heading"><span><i class="pi pi-clone"></i>原图 / 结果图对比</span><small>结果来自已验证发布文件</small></div>
              <div class="comparison-grid">
                <div class="preview-card">
                  <span>原图</span>
                  <img v-if="item.previewUrl" :src="item.previewUrl" :alt="item.name">
                  <div v-else class="preview-placeholder"><i class="pi pi-spin pi-spinner"></i>正在读取</div>
                  <small>{{ item.width || '—' }} × {{ item.height || '—' }} · {{ formatBytes(item.inputSize) }}</small>
                </div>
                <div class="preview-card" :class="item.resultPreviewUrl ? 'result-ready' : 'result-pending'">
                  <span>结果图</span>
                  <img v-if="item.resultPreviewUrl" :src="item.resultPreviewUrl" :alt="`${item.name} 压缩结果`">
                  <div v-else class="preview-placeholder"><i class="pi" :class="taskForItem(item)?.status === 'failed' ? 'pi-exclamation-triangle' : taskForItem(item)?.status === 'cancelled' ? 'pi-ban' : 'pi-hourglass'"></i>{{ taskForItem(item)?.error || (taskForItem(item)?.status === 'cancelled' ? '任务已取消，未生成结果' : taskForItem(item)?.status === 'completed' ? '已跳过，未生成结果' : '等待真实结果') }}</div>
                  <small v-if="taskForItem(item)?.metrics">{{ resultFacts(item)?.visibleWidth }} × {{ resultFacts(item)?.visibleHeight }} · {{ formatBytes(taskForItem(item)!.metrics!.outputBytes) }}</small>
                  <small v-else>不会使用原图或预计值伪装结果</small>
                  <small v-if="taskForItem(item)?.metrics" class="result-path" :title="taskForItem(item)?.outputPath">{{ taskForItem(item)?.outputPath }}</small>
                  <button v-if="taskForItem(item)?.metrics" type="button" class="open-result" @click="openResultLocation(item)"><i class="pi pi-folder-open"></i>打开结果位置</button>
                </div>
              </div>
              <div v-if="estimateFor(item) && !taskForItem(item)?.metrics" class="estimate-card">
                <span><i class="pi pi-chart-bar"></i>前端参考区间（非实际结果）</span>
                <strong>{{ formatBytes(estimateFor(item)!.minimum) }} – {{ formatBytes(estimateFor(item)!.maximum) }}</strong>
                <small>仅按输入大小、压缩方式与质量给出范围；任务完成后由后端真实字节、尺寸和发布路径替代。</small>
              </div>
              <button type="button" class="remove-item" :disabled="isRunning" @click="store.removeImageItem(item.id)"><i class="pi pi-times"></i>移除此图片</button>
            </div>
          </div>
        </article>
      </div>
      <EnhancedFileDropzone compact mode="file" accept="jpg,jpeg,png,webp" picker-title="选择图片文件" unfiltered-picker hint="继续添加图片" :native-drop="false" @files-selected="onFilesSelected" />
    </div>

    <Modal :visible="showGlobalSettings" size="xl" title="图片批量设置" description="应用到尚未单独覆盖的图片" icon="pi pi-sliders-h" @update:visible="showGlobalSettings = $event" @close="cancelGlobalSettings">
      <div class="special-settings-dialog" :class="{ locked: isRunning }">
        <ImageCompressionSettingsPanel :model-value="imageSettingsDraft" @update:model-value="updateGlobalSettings" />
        <div class="output-directory">
          <div class="min-w-0"><span>输出目录</span><strong :title="imageSettingsDraft.outputDirectory">{{ imageSettingsDraft.outputDirectory || '与源文件同目录' }}</strong></div>
          <button type="button" @click="chooseOutputDirectory()"><i class="pi pi-folder-open"></i>选择目录</button>
        </div>
      </div>
      <template #footer>
        <button type="button" class="dialog-cancel" @click="cancelGlobalSettings">取消</button>
        <button type="button" class="dialog-save" data-testid="image-save-global-settings" @click="saveGlobalSettings">保存设置</button>
      </template>
    </Modal>
  </section>
</template>

<style scoped>
.image-workspace{display:flex;min-width:0;min-height:0;flex:1;flex-direction:column;gap:.75rem;overflow:hidden}
.workspace-toolbar{display:flex;align-items:center;justify-content:flex-end;gap:1rem}.toolbar-actions{display:flex;gap:.5rem;flex-shrink:0}.toolbar-actions button,.output-directory button{display:flex;align-items:center;justify-content:center;gap:.4rem;height:2.5rem;border-radius:.8rem;padding:0 .9rem;font-size:.72rem;font-weight:900;transition:transform .18s ease,border-color .18s ease,box-shadow .18s ease}.toolbar-actions button:hover:not(:disabled){transform:translateY(-1px);box-shadow:0 10px 22px -17px var(--dynamic-accent)}.toolbar-actions button:active:not(:disabled){transform:scale(.97)}.secondary-action,.output-directory button{border:1px solid var(--border-subtle);background:var(--bg-input);color:var(--text-content)}.primary-action{border:0;background:var(--dynamic-accent);color:white}.primary-action:disabled{cursor:not-allowed;filter:saturate(.25);opacity:.5}.batch-progress{display:flex;align-items:center;gap:.5rem;border-radius:.8rem;background:color-mix(in srgb,var(--dynamic-accent) 8%,transparent);padding:.55rem .75rem;color:var(--text-muted);font-size:.68rem;font-weight:800}.batch-progress i,.batch-progress strong{color:var(--dynamic-accent)}.batch-progress strong{margin-left:auto}
.special-settings-dialog{min-width:0}.settings-heading{display:flex;align-items:center;justify-content:space-between;gap:.75rem;margin-bottom:.7rem;color:var(--text-content);font-size:.73rem;font-weight:900}.settings-heading span{display:flex;align-items:center;gap:.45rem}.settings-heading i{color:var(--dynamic-accent)}.settings-heading small,.settings-heading label{color:var(--text-muted);font-size:.65rem;font-weight:750}.settings-heading label{display:flex;align-items:center;gap:.35rem}.settings-heading input{accent-color:var(--dynamic-accent)}.output-directory{display:flex;align-items:center;justify-content:space-between;gap:.75rem;margin-top:.7rem;border:1px solid var(--border-subtle);border-radius:.8rem;background:color-mix(in srgb,var(--bg-input) 65%,transparent);padding:.55rem .65rem}.output-directory>div{display:flex;min-width:0;flex-direction:column;gap:.15rem}.output-directory span{color:var(--text-muted);font-size:.62rem;font-weight:800}.output-directory strong{overflow:hidden;color:var(--text-content);font-size:.72rem;text-overflow:ellipsis;white-space:nowrap}.output-directory button{height:2.1rem;flex-shrink:0}.dialog-cancel,.dialog-save{border-radius:.75rem;padding:.65rem 1rem;font-size:.7rem;font-weight:900}.dialog-cancel{border:1px solid var(--border-subtle);background:var(--bg-input);color:var(--text-content)}.dialog-save{background:var(--dynamic-accent);color:white;box-shadow:0 10px 24px -15px var(--dynamic-accent)}
.image-empty{display:flex;min-height:0;flex:1}.image-empty :deep(.drop-area){display:flex;min-height:15rem;width:100%;align-items:center;justify-content:center}.image-list-shell{display:flex;min-height:0;flex:1;flex-direction:column;gap:.55rem}.image-list-summary{display:flex;align-items:center;gap:1rem;color:var(--text-muted);font-size:.68rem;font-weight:750}.image-list-summary strong{color:var(--dynamic-accent)}.image-list-summary button{margin-left:auto;color:var(--text-muted)}.image-list{display:flex;min-height:0;flex:1;flex-direction:column;gap:.55rem;overflow-x:hidden;overflow-y:auto;padding-right:.25rem}.image-task{max-width:100%;min-width:0;overflow:hidden;border:1px solid var(--border-subtle);border-radius:1rem;background:color-mix(in srgb,var(--bg-card) 76%,transparent);transition:.2s ease}.image-task.expanded{border-color:color-mix(in srgb,var(--dynamic-accent) 38%,transparent);box-shadow:0 18px 34px -28px #000}.image-task.rejected{border-color:color-mix(in srgb,#ef4444 35%,transparent)}
.image-row{display:grid;width:100%;min-width:0;grid-template-columns:minmax(9rem,1.5fr) minmax(6rem,.62fr) minmax(5rem,.48fr) minmax(4.5rem,.42fr) minmax(5.2rem,.52fr) minmax(4.7rem,.46fr) 1.5rem;align-items:center;gap:.65rem;padding:.75rem;text-align:left}.image-name{display:flex;min-width:0;align-items:center;gap:.65rem}.image-name>i{display:flex;width:2rem;height:2rem;flex-shrink:0;align-items:center;justify-content:center;border:1px solid color-mix(in srgb,var(--dynamic-accent) 20%,transparent);border-radius:.65rem;background:color-mix(in srgb,var(--dynamic-accent) 8%,transparent);color:var(--dynamic-accent)}.image-name>span{display:flex;min-width:0;flex-direction:column}.image-name strong,.image-name small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.image-name strong{color:var(--text-content);font-size:.74rem}.image-name small,.image-status small,.image-saving small{color:var(--text-muted);font-size:.6rem}.image-dimensions,.image-size,.image-format{overflow:hidden;color:var(--text-muted);font-size:.68rem;font-weight:800;text-overflow:ellipsis;white-space:nowrap}.image-format{color:var(--dynamic-accent)}.image-status,.image-saving{display:flex;min-width:0;flex-direction:column;gap:.12rem;color:var(--text-content);font-size:.68rem}.image-status.ready strong{color:var(--dynamic-accent)}.image-status.rejected strong{color:#ef4444}.row-actions{color:var(--text-muted)}.item-error{display:flex;align-items:center;gap:.5rem;border-top:1px solid color-mix(in srgb,#ef4444 18%,transparent);padding:.55rem .8rem;color:#ef4444;font-size:.68rem;font-weight:800}
.image-details{display:grid;min-width:0;height:clamp(20rem,48vh,29rem);grid-template-columns:minmax(0,.9fr) minmax(0,1.1fr);overflow:hidden;border-top:1px solid var(--border-subtle)}.item-config,.comparison-panel{min-width:0;min-height:0;overflow-x:hidden;overflow-y:auto;padding:1rem}.item-config{border-right:1px solid var(--border-subtle)}.comparison-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:.65rem}.preview-card{display:flex;min-width:0;flex-direction:column;gap:.45rem;border:1px solid var(--border-subtle);border-radius:.85rem;background:var(--bg-input);padding:.55rem}.preview-card>span{color:var(--text-content);font-size:.67rem;font-weight:900}.preview-card img,.preview-placeholder{width:100%;height:8rem;border-radius:.65rem;background:color-mix(in srgb,var(--bg-base) 65%,transparent);object-fit:contain}.preview-placeholder{display:flex;align-items:center;justify-content:center;gap:.4rem;color:var(--text-muted);font-size:.65rem;font-weight:750}.preview-card small{overflow:hidden;color:var(--text-muted);font-size:.6rem;text-overflow:ellipsis;white-space:nowrap}.result-pending{border-style:dashed}.estimate-card{display:flex;flex-direction:column;gap:.35rem;margin-top:.75rem;border:1px solid color-mix(in srgb,var(--dynamic-accent) 24%,transparent);border-radius:.85rem;background:color-mix(in srgb,var(--dynamic-accent) 6%,transparent);padding:.75rem}.estimate-card span{color:var(--text-content);font-size:.67rem;font-weight:900}.estimate-card strong{color:var(--dynamic-accent);font-size:1rem}.estimate-card small{color:var(--text-muted);font-size:.62rem;font-weight:650;line-height:1.5}.remove-item{margin-top:.7rem;color:#ef4444;font-size:.67rem;font-weight:850}.remove-item i{margin-right:.35rem}
.danger-action{border:1px solid color-mix(in srgb,#ef4444 45%,transparent);background:color-mix(in srgb,#ef4444 14%,var(--bg-input));color:#ef4444}.locked{pointer-events:none;opacity:.68}.image-status.completed strong{color:var(--dynamic-accent)}.image-status.failed strong{color:#ef4444}.image-status.cancelled strong{color:#f59e0b}.image-status small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.image-saving strong{font-size:.68rem}.result-ready{border-color:color-mix(in srgb,var(--dynamic-accent) 35%,transparent)}.result-path{direction:ltr;text-align:left}.open-result{display:flex;align-items:center;justify-content:center;gap:.35rem;border:1px solid var(--border-subtle);border-radius:.55rem;background:var(--bg-card);padding:.35rem;color:var(--text-content);font-size:.62rem;font-weight:850}
@media(max-width:900px){.image-row{grid-template-columns:minmax(8rem,1fr) minmax(5.5rem,.5fr) minmax(4.5rem,.4fr) minmax(5rem,.45fr) 1.5rem}.image-size,.image-format{display:none}.workspace-toolbar{align-items:flex-start;flex-direction:column}.toolbar-actions{width:100%}.toolbar-actions button{flex:1}}
@media(max-width:620px){.image-row{grid-template-columns:minmax(0,1fr) minmax(4.5rem,.42fr) 1.5rem}.image-dimensions,.image-saving{display:none}.image-details{grid-template-columns:minmax(0,1fr) minmax(0,1fr)}.comparison-grid{grid-template-columns:minmax(0,1fr)}.preview-card img,.preview-placeholder{height:6rem}.item-config,.comparison-panel{padding:.7rem}}
</style>
