<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { ask, open } from '@tauri-apps/api/dialog'
import { useAppStore } from '@/stores/app'
import { useCompressionStore, type VideoCompressionItem } from '@/stores/compression'
import { useTaskStore, type Task } from '@/stores/task'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useVideoCompressionBatch } from '@/composables/useVideoCompressionBatch'
import type { VideoCompressionSettings } from '@/types/video'
import type { VideoCandidate } from '@/utils/videoCompressionWorkspace'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import Modal from '@/components/ui/Modal.vue'
import VideoCompressionSettingsPanel from './VideoCompressionSettingsPanel.vue'

const appStore = useAppStore()
const store = useCompressionStore()
const taskStore = useTaskStore()
const commands = useTauriCommands()
const videoBatch = useVideoCompressionBatch()
const inFlight = new Map<string, number>()
const replanTimers = new Map<string, ReturnType<typeof setTimeout>>()
const isRunning = ref(false)
const showGlobalSettings = ref(false)
const videoSettingsDraft = ref<VideoCompressionSettings>({ ...store.videoGlobalSettings })
const videoOutputDirectoryDraft = ref(store.videoOutputDirectory)

const formatBytes = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KiB`
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MiB`
  return `${(bytes / 1024 ** 3).toFixed(2)} GiB`
}

const formatDuration = (durationMs: number) => {
  const seconds = Math.max(0, Math.round(durationMs / 1000))
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const remainder = seconds % 60
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, '0')}:${String(remainder).padStart(2, '0')}`
    : `${minutes}:${String(remainder).padStart(2, '0')}`
}

const frameRateLabel = (item: VideoCompressionItem) => {
  const video = item.plan?.probe.primaryVideo
  if (!video) return '—'
  const fps = video.averageFrameRateMilli === null ? '未知帧率' : `${(video.averageFrameRateMilli / 1000).toFixed(3)} fps`
  return video.frameRateMode === 'variable' ? `${fps} · VFR` : `${fps} · 恒定或未确定`
}

const streamChangeLabel = (change: string) => {
  if (change.startsWith('VIDEO_PLAN_RESIZE:')) {
    const dimensions = change.match(/from (\d+x\d+) to (\d+x\d+)/)
    return dimensions ? `可见分辨率将从 ${dimensions[1]} 调整为 ${dimensions[2]}。` : '可见分辨率将按最大边界等比调整。'
  }
  if (change.startsWith('VIDEO_PLAN_ROTATION_NORMALIZED:')) {
    const degrees = change.match(/(\d+) degree/)
    return degrees ? `将应用 ${degrees[1]}° 旋转到可见像素，并移除方向歧义。` : '将旋转方向应用到可见像素。'
  }
  const labels: Array<[string, string]> = [
    ['VIDEO_PLAN_CONTAINER_CHANGE:', '输出容器将改为 MP4。'],
    ['VIDEO_PLAN_VIDEO_CODEC_CHANGE:', '主视频流将编码为 H.264。'],
    ['VIDEO_PLAN_VFR_TIMESTAMPS_PRESERVED:', '将沿用输入时间戳以保持 VFR 时序。'],
    ['VIDEO_PLAN_PRIMARY_AUDIO_CHANGE:', '主音轨将编码为 AAC。'],
    ['VIDEO_PROBE_MULTIPLE_VIDEO_STREAMS:', '仅处理默认或第一条视频流。'],
    ['VIDEO_PROBE_ADDITIONAL_AUDIO_WILL_BE_DROPPED:', '额外音轨将被移除。'],
    ['VIDEO_PROBE_SUBTITLES_WILL_BE_DROPPED:', '字幕流将被移除。'],
    ['VIDEO_PROBE_CHAPTERS_WILL_BE_DROPPED:', '章节将被移除。'],
    ['VIDEO_PROBE_ATTACHED_PICTURES_WILL_BE_DROPPED:', '封面图将被移除。'],
    ['VIDEO_PROBE_HDR_UNSUPPORTED:', '首期不支持可靠保留或映射 HDR，当前禁止编码。'],
  ]
  const match = labels.find(([prefix]) => change.startsWith(prefix))
  if (!match) return change
  return match[1]
}

const settingsFor = (item: VideoCompressionItem) => ({ ...store.getEffectiveVideoSettings(item) })

const planItem = async (item: VideoCompressionItem) => {
  const revision = item.planRevision
  if (inFlight.get(item.id) === revision || item.status !== 'planning') return
  inFlight.set(item.id, revision)
  try {
    const plan = await commands.planVideoCompression({ path: item.path, ...settingsFor(item) })
    store.completeVideoPlanning(item.id, revision, plan)
  } catch (error) {
    store.failVideoPlanning(item.id, revision, String(error))
  } finally {
    if (inFlight.get(item.id) === revision) inFlight.delete(item.id)
  }
}

watch(
  () => store.videoItems.map(item => `${item.id}:${item.status}:${item.planRevision}`).join('|'),
  () => store.videoItems.filter(item => item.status === 'planning').forEach(item => void planItem(item)),
  { immediate: true },
)

const onFilesSelected = (candidates: VideoCandidate[]) => {
  const result = store.addVideoCandidates(candidates)
  if (result.rejected.length > 0) {
    const details = result.rejected.slice(0, 2).map(item => `${item.name}：${item.reason}`).join('；')
    appStore.setError(`已拒绝 ${result.rejected.length} 个视频候选。${details}`)
  }
}

const openGlobalSettings = () => {
  videoSettingsDraft.value = { ...store.videoGlobalSettings }
  videoOutputDirectoryDraft.value = store.videoOutputDirectory
  showGlobalSettings.value = true
}
const updateGlobalSettings = (settings: VideoCompressionSettings) => {
  if (!isRunning.value) videoSettingsDraft.value = settings
}
const saveGlobalSettings = () => {
  if (isRunning.value) return
  store.updateVideoGlobalSettings({ ...videoSettingsDraft.value })
  store.videoOutputDirectory = videoOutputDirectoryDraft.value
  showGlobalSettings.value = false
}
const cancelGlobalSettings = () => {
  videoSettingsDraft.value = { ...store.videoGlobalSettings }
  videoOutputDirectoryDraft.value = store.videoOutputDirectory
  showGlobalSettings.value = false
}
const updateItemSettings = (item: VideoCompressionItem, settings: VideoCompressionSettings) => {
  if (isRunning.value) return
  store.updateVideoItemSettingsDraft(item.id, settings)
  const previous = replanTimers.get(item.id)
  if (previous) clearTimeout(previous)
  replanTimers.set(item.id, setTimeout(() => {
    replanTimers.delete(item.id)
    store.retryVideoPlanning(item.id)
  }, 180))
}
onUnmounted(() => replanTimers.forEach(timer => clearTimeout(timer)))
const toggleOverride = (item: VideoCompressionItem, enabled: boolean) => {
  if (isRunning.value) return
  if (enabled) store.enableVideoItemOverride(item.id)
  else store.disableVideoItemOverride(item.id)
}
const planningCount = computed(() => store.videoItems.filter(item => item.status === 'planning').length)
const taskForItem = (item: VideoCompressionItem): Task | undefined =>
  item.taskId ? taskStore.tasks.find(task => task.id === item.taskId) : undefined
const canRetryTask = (task?: Task) => !task || task.status === 'failed' || task.status === 'cancelled'
const runnableItems = computed(() => store.videoItems.filter(item =>
  item.status === 'ready' && item.plan?.canEncode && canRetryTask(taskForItem(item))))
const canStart = computed(() => !isRunning.value && runnableItems.value.length > 0 && planningCount.value === 0)

const chooseOutputDirectory = async () => {
  try {
    const queued = import.meta.env.VITE_DESKTOP_E2E === '1'
      ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection()
      : undefined
    const selected = queued === undefined
      ? await open({ directory: true, multiple: false, title: '选择视频输出目录' })
      : queued
    if (selected && !Array.isArray(selected)) videoOutputDirectoryDraft.value = selected
  } catch (error) {
    appStore.setError(`无法选择视频输出目录：${String(error)}`)
  }
}

const videoStatus = (item: VideoCompressionItem) => {
  const task = taskForItem(item)
  if (!task) return item.status === 'planning' ? '正在探测' : item.status === 'ready' ? '规划就绪' : '已拒绝'
  const labels: Partial<Record<Task['status'], string>> = {
    pending: '等待中', preparing: '准备中', compressing: '编码中', finalizing: '正在发布',
    cancelling: '正在取消', completed: '已完成', failed: '失败', cancelled: '已取消',
  }
  return labels[task.status] || task.status
}
const videoStatusClass = (item: VideoCompressionItem) => taskForItem(item)?.status || item.status
const formatEta = (seconds?: number) => seconds === undefined
  ? '计算中'
  : seconds < 60 ? `${Math.max(1, Math.ceil(seconds))} 秒` : `${Math.ceil(seconds / 60)} 分钟`

const confirmStreamChanges = async (items: VideoCompressionItem[]) => {
  const risky = items.filter(item => item.plan?.requiresExplicitConfirmation)
  if (risky.length === 0) return true
  const lines = risky.flatMap(item => [
    `\n${item.name}：`,
    ...item.plan!.streamChanges.map(change => `• ${streamChangeLabel(change)}`),
  ])
  if (import.meta.env.VITE_DESKTOP_E2E === '1') {
    const selected = window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopConfirmation()
    if (selected !== undefined) return selected
  }
  return ask(`以下视频存在有损流变化。请确认已阅读并接受：\n${lines.join('\n')}`, {
    title: '确认视频流变化',
    type: 'warning',
    okLabel: '确认并开始',
    cancelLabel: '取消',
  })
}

const startVideoCompression = async () => {
  const items = runnableItems.value
  if (!canStart.value || !(await confirmStreamChanges(items))) return
  isRunning.value = true
  try {
    const results = await videoBatch.runVideoBatch(
      items.map(item => ({
        id: item.id,
        name: item.name,
        path: item.path,
        plan: item.plan!,
        settings: { ...store.getEffectiveVideoSettings(item) },
      })),
      store.videoOutputDirectory || null,
      appStore.settings.preserveMarkOfWeb,
      (itemId, taskId) => store.bindVideoItemTask(itemId, taskId),
    )
    const published = results.filter(result => result.status === 'published').length
    const failed = results.filter(result => result.status === 'failed').length
    const cancelled = results.filter(result => result.status === 'cancelled').length
    const summary = `视频处理结束：${published} 个完成，${failed} 个失败，${cancelled} 个取消`
    if (failed || cancelled) appStore.setError(summary)
    else appStore.setSuccess(summary)
  } catch (error) {
    appStore.setError(`视频批量处理失败：${String(error)}`)
  } finally {
    isRunning.value = false
  }
}

const cancelVideoCompression = async () => videoBatch.cancelVideoBatch()
const playResultWithDefaultApplication = async (item: VideoCompressionItem) => {
  const path = taskForItem(item)?.outputPath
  if (!path) return
  try {
    await commands.openVideoOutputWithDefaultApplication(path)
    appStore.setSuccess('已将视频交给系统默认应用播放')
  } catch (error) {
    appStore.setError(`无法使用默认应用播放视频：${String(error)}`)
  }
}
</script>

<template>
  <section class="video-workspace" data-testid="video-compression-workspace">
    <div class="workspace-toolbar">
      <div class="toolbar-actions">
        <button type="button" class="secondary-action" :aria-expanded="showGlobalSettings" data-testid="video-toggle-global-settings" @click="openGlobalSettings"><i class="pi pi-sliders-h"></i>批量设置</button>
        <button v-if="isRunning" type="button" class="danger-action" @click="cancelVideoCompression"><i class="pi pi-stop-circle"></i>取消视频压缩</button>
        <button v-else type="button" class="primary-action" :disabled="!canStart" @click="startVideoCompression"><i class="pi pi-play-circle"></i>开始视频压缩</button>
      </div>
    </div>

    <div v-if="store.videoItems.length === 0" class="video-empty">
      <EnhancedFileDropzone
        mode="file"
        accept="mp4,mov,avi,wmv,webm,mkv,m4v"
        picker-title="选择视频文件"
        unfiltered-picker
        hint="拖入需要分析的视频"
        sub-hint="以真实容器探测结果为准，不按扩展名假定可处理"
        @files-selected="onFilesSelected"
      />
    </div>

    <div v-else class="video-list workspace-scroll-region custom-scrollbar" data-testid="video-workspace-scroll-region">
      <article v-for="item in store.videoItems" :key="item.id" class="video-card" :data-status="videoStatusClass(item)" data-testid="video-draft-card">
        <header>
          <button type="button" class="expand" :aria-expanded="item.expanded" @click="item.expanded = !item.expanded"><i :class="item.expanded ? 'pi pi-chevron-down' : 'pi pi-chevron-right'"></i></button>
          <div class="file-identity"><strong :title="item.path">{{ item.name }}</strong><small>{{ formatBytes(item.inputSize) }} · {{ item.path }}</small></div>
          <div v-if="item.plan && !item.expanded" class="video-essentials">
            <strong>{{ item.plan.probe.primaryVideo.visibleWidth }}×{{ item.plan.probe.primaryVideo.visibleHeight }} · {{ formatDuration(item.plan.probe.durationMs) }}</strong>
            <small>质量 {{ store.getEffectiveVideoSettings(item).quality }} · 预计 {{ formatBytes(item.plan.estimatedOutput.lowBytes) }}—{{ formatBytes(item.plan.estimatedOutput.highBytes) }}</small>
          </div>
          <span class="status"><i :class="taskForItem(item)?.status === 'completed' ? 'pi pi-check-circle' : item.status === 'planning' || taskForItem(item)?.status === 'compressing' ? 'pi pi-spin pi-spinner' : item.status === 'ready' ? 'pi pi-check-circle' : 'pi pi-times-circle'"></i>{{ videoStatus(item) }}</span>
          <button type="button" class="remove" title="移除" :disabled="isRunning" @click="store.removeVideoItem(item.id)"><i class="pi pi-trash"></i></button>
        </header>

        <div v-if="item.error" class="probe-error"><i class="pi pi-exclamation-triangle"></i><span>{{ item.error }}</span><button type="button" @click="store.retryVideoPlanning(item.id)">重试</button></div>

        <template v-if="item.plan && item.expanded">
          <div class="facts-grid">
            <div><span>输入事实</span><strong>{{ item.plan.probe.primaryVideo.visibleWidth }}×{{ item.plan.probe.primaryVideo.visibleHeight }}</strong><small>编码矩阵 {{ item.plan.probe.primaryVideo.encodedWidth }}×{{ item.plan.probe.primaryVideo.encodedHeight }} · 旋转 {{ item.plan.probe.primaryVideo.rotationDegrees }}°</small></div>
            <div><span>时长与帧率</span><strong>{{ formatDuration(item.plan.probe.durationMs) }}</strong><small>{{ frameRateLabel(item) }}</small></div>
            <div><span>流</span><strong>{{ item.plan.probe.primaryVideo.codec || '未知视频编码' }} · {{ item.plan.probe.audioStreams.length }} 音轨</strong><small>{{ item.plan.probe.subtitleStreams.length }} 字幕 · {{ item.plan.probe.chapterCount }} 章节 · {{ item.plan.probe.attachedPictureCount }} 封面</small></div>
            <div class="estimate"><span>预计输出 · 估算</span><strong>{{ formatBytes(item.plan.estimatedOutput.lowBytes) }}—{{ formatBytes(item.plan.estimatedOutput.highBytes) }}</strong><small>{{ item.plan.outputWidth }}×{{ item.plan.outputHeight }} · {{ (item.plan.targetVideoBitRate / 1_000_000).toFixed(2) }} Mbps 视频</small></div>
          </div>

          <div class="estimate-disclaimer" :title="item.plan.estimatedOutput.disclaimer"><i class="pi pi-info-circle"></i>估算区间会随画面复杂度、VFR 时序和实际编码器行为变化，不能作为最终大小。</div>
          <div v-if="item.plan.streamChanges.length" class="stream-changes" :class="{ blocked: !item.plan.canEncode }">
            <strong>{{ item.plan.canEncode ? '执行前流变化' : '当前阻断原因' }}</strong>
            <ul><li v-for="change in item.plan.streamChanges" :key="change">{{ streamChangeLabel(change) }}</li></ul>
            <small v-if="item.plan.requiresExplicitConfirmation"><i class="pi pi-exclamation-circle"></i> 后续执行前必须显式确认这些有损流变化。</small>
          </div>
        </template>

        <div v-if="item.expanded" class="item-settings">
          <label class="override-toggle"><input type="checkbox" :checked="Boolean(item.settings)" @change="toggleOverride(item, ($event.target as HTMLInputElement).checked)"><span>为此视频使用单项配置</span></label>
          <VideoCompressionSettingsPanel v-if="item.settings" :model-value="item.settings" @update:model-value="updateItemSettings(item, $event)" />
          <p v-else>当前跟随批量配置：质量 {{ store.getEffectiveVideoSettings(item).quality }}，{{ store.getEffectiveVideoSettings(item).maxWidth ? `最大 ${store.getEffectiveVideoSettings(item).maxWidth}×${store.getEffectiveVideoSettings(item).maxHeight}` : '保持原尺寸' }}。</p>
          <div v-if="taskForItem(item)" class="execution-facts">
            <div><span>执行阶段</span><strong>{{ taskForItem(item)!.stage || videoStatus(item) }}</strong><small v-if="taskForItem(item)!.heartbeatSecondsSinceProgress !== undefined">仍在编码 · {{ taskForItem(item)!.heartbeatSecondsSinceProgress }} 秒前收到真实进度</small></div>
            <div><span>媒体时间</span><strong>{{ formatDuration(taskForItem(item)!.currentTimeMs || 0) }}</strong><small>{{ taskForItem(item)!.speed || '等待速度样本' }} · ETA {{ formatEta(taskForItem(item)!.etaSeconds) }}</small></div>
            <div><span>{{ taskForItem(item)!.outputBytesEstimated ? '临时输出' : '最终输出' }}</span><strong>{{ formatBytes(taskForItem(item)!.outputBytes || 0) }}</strong><small>{{ taskForItem(item)!.outputToInputRatio === undefined ? '等待真实比例' : `输入的 ${(taskForItem(item)!.outputToInputRatio! * 100).toFixed(1)}%` }}</small></div>
            <button v-if="taskForItem(item)!.status === 'completed'" type="button" data-testid="video-open-default-app" @click="playResultWithDefaultApplication(item)"><i class="pi pi-play"></i>默认应用播放</button>
          </div>
        </div>
      </article>

      <EnhancedFileDropzone compact mode="file" accept="mp4,mov,avi,wmv,webm,mkv,m4v" unfiltered-picker hint="继续添加视频" @files-selected="onFilesSelected" />
    </div>

    <Modal :visible="showGlobalSettings" size="lg" title="视频批量设置" description="质量与分辨率分别控制" icon="pi pi-sliders-h" @update:visible="showGlobalSettings = $event" @close="cancelGlobalSettings">
      <div class="special-settings-dialog" :class="{ locked: isRunning }">
        <VideoCompressionSettingsPanel :model-value="videoSettingsDraft" @update:model-value="updateGlobalSettings" />
        <div class="output-directory"><div><span>输出目录</span><strong :title="videoOutputDirectoryDraft">{{ videoOutputDirectoryDraft || '与源文件同目录' }}</strong></div><button type="button" :disabled="isRunning" @click="chooseOutputDirectory"><i class="pi pi-folder-open"></i>选择目录</button></div>
      </div>
      <template #footer>
        <button type="button" class="dialog-cancel" @click="cancelGlobalSettings">取消</button>
        <button type="button" class="dialog-save" data-testid="video-save-global-settings" :disabled="isRunning" @click="saveGlobalSettings">保存设置</button>
      </template>
    </Modal>
  </section>
</template>

<style scoped>
.video-workspace { box-sizing: border-box; display: flex; width: 100%; max-width: 100%; min-width: 0; min-height: 0; flex: 1; flex-direction: column; gap: .75rem; overflow: hidden; padding: .1rem; }
.video-workspace > * { box-sizing: border-box; max-width: 100%; min-width: 0; }
.workspace-toolbar { display: flex; align-items: center; justify-content: flex-end; gap: .75rem 1rem; }
.toolbar-actions { display: flex; flex-shrink: 0; align-items: center; gap: .5rem; }.toolbar-actions button { display: flex; height: 2.75rem; align-items: center; justify-content: center; gap: .4rem; border-radius: .8rem; padding: 0 .9rem; font-size: .7rem; font-weight: 900; white-space: nowrap; transition: transform .18s ease,box-shadow .18s ease }.toolbar-actions .secondary-action { width: 7.25rem; }.toolbar-actions .primary-action,.toolbar-actions .danger-action { width: 9.5rem; }.toolbar-actions button i { width: 1rem; flex: 0 0 1rem; text-align: center; }.primary-action:hover:not(:disabled),.danger-action:hover:not(:disabled),.secondary-action:hover:not(:disabled){transform:translateY(-1px);box-shadow:0 10px 22px -17px var(--dynamic-accent)}.primary-action:active:not(:disabled),.danger-action:active:not(:disabled),.secondary-action:active:not(:disabled){transform:scale(.97)}.primary-action, .danger-action { color: white; }.primary-action { background: var(--dynamic-accent); }.danger-action { background: #ef4444; }.secondary-action{border:1px solid var(--border-subtle);background:var(--bg-input);color:var(--text-content)}.primary-action:disabled { cursor: not-allowed; opacity: .45; }
.video-card { box-sizing: border-box; max-width: 100%; min-width: 0; border: 1px solid var(--border-subtle); border-radius: 1rem; background: color-mix(in srgb, var(--bg-card) 88%, transparent); padding: .85rem; }
.special-settings-dialog{min-width:0}.locked{pointer-events:none;opacity:.68}.dialog-cancel,.dialog-save{border-radius:.75rem;padding:.65rem 1rem;font-size:.7rem;font-weight:900}.dialog-cancel{border:1px solid var(--border-subtle);background:var(--bg-input);color:var(--text-content)}.dialog-save{background:var(--dynamic-accent);color:white;box-shadow:0 10px 24px -15px var(--dynamic-accent)}.dialog-save:disabled{opacity:.45}
.settings-heading { display: flex; justify-content: space-between; gap: .75rem; margin-bottom: .65rem; color: var(--text-content); font-size: .68rem; font-weight: 900; }.settings-heading small { color: var(--text-muted); font-size: .58rem; font-weight: 650; }
.output-directory { display: flex; align-items: center; justify-content: space-between; gap: .75rem; margin-top: .7rem; border-top: 1px solid var(--border-subtle); padding-top: .65rem; }.output-directory div { min-width: 0; }.output-directory span, .output-directory strong { display: block; }.output-directory span { color: var(--text-muted); font-size: .55rem; }.output-directory strong { overflow: hidden; margin-top: .15rem; color: var(--text-content); font-size: .62rem; text-overflow: ellipsis; white-space: nowrap; }.output-directory button, .execution-facts button { flex: 0 0 auto; border: 1px solid var(--border-subtle); border-radius: .55rem; padding: .35rem .55rem; color: var(--text-content); font-size: .58rem; font-weight: 800; }
.video-empty { display:flex;min-height:0;flex:1; }.video-empty :deep(.drop-area){display:flex;min-height:13rem;width:100%;align-items:center;justify-content:center}.video-list { display: grid; width: 100%; max-width: 100%; min-width: 0; min-height:0; flex:1; grid-auto-rows:max-content; align-content:start; gap: .65rem; overflow-x:hidden;overflow-y:auto;padding-right:.25rem }.video-card { overflow: hidden; }
.video-card header { display: flex; min-width: 0; align-items: center; gap: .55rem; }.expand, .remove { flex: 0 0 auto; width: 1.8rem; height: 1.8rem; border-radius: .55rem; color: var(--text-muted); }.expand:hover, .remove:hover { background: var(--bg-input); color: var(--text-content); }
.file-identity { min-width: 0; flex: 1; }.file-identity strong, .file-identity small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }.file-identity strong { color: var(--text-content); font-size: .72rem; }.file-identity small { margin-top: .12rem; color: var(--text-muted); font-size: .55rem; }
.video-essentials{display:flex;min-width:12rem;max-width:24rem;flex:0 1 24rem;flex-direction:column;align-items:flex-end;gap:.12rem}.video-essentials strong,.video-essentials small{max-width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.video-essentials strong{color:var(--text-content);font-size:.62rem}.video-essentials small{color:var(--text-muted);font-size:.54rem}
.status { display: flex; flex: 0 0 auto; align-items: center; gap: .3rem; border-radius: 999px; background: var(--bg-input); padding: .28rem .48rem; color: var(--text-muted); font-size: .58rem; font-weight: 850; }.video-card[data-status="ready"] .status { color: #22c55e; }.video-card[data-status="rejected"] .status { color: #ef4444; }
.probe-error { display: flex; align-items: center; gap: .5rem; margin-top: .65rem; border-radius: .7rem; background: color-mix(in srgb, #ef4444 9%, transparent); padding: .6rem; color: #ef4444; font-size: .6rem; }.probe-error span { min-width: 0; flex: 1; overflow-wrap: anywhere; }.probe-error button { border: 1px solid currentColor; border-radius: .5rem; padding: .25rem .45rem; font-weight: 800; }
.facts-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: .5rem; margin-top: .7rem; }.facts-grid > div { min-width: 0; border-radius: .7rem; background: var(--bg-input); padding: .6rem; }.facts-grid span, .facts-grid strong, .facts-grid small { display: block; }.facts-grid span { color: var(--text-muted); font-size: .55rem; font-weight: 800; }.facts-grid strong { margin-top: .18rem; color: var(--text-content); font-size: .7rem; overflow-wrap: anywhere; }.facts-grid small { margin-top: .16rem; color: var(--text-muted); font-size: .53rem; line-height: 1.4; }.facts-grid .estimate { outline: 1px solid color-mix(in srgb, var(--dynamic-accent) 22%, transparent); }.facts-grid .estimate span, .facts-grid .estimate strong { color: var(--dynamic-accent); }
.estimate-disclaimer { display: flex; gap: .35rem; margin-top: .5rem; color: var(--text-muted); font-size: .55rem; font-style: italic; }
.stream-changes { margin-top: .6rem; border-left: 3px solid #f59e0b; border-radius: .45rem; background: color-mix(in srgb, #f59e0b 7%, transparent); padding: .55rem .7rem; color: var(--text-muted); font-size: .56rem; }.stream-changes.blocked { border-left-color: #ef4444; background: color-mix(in srgb, #ef4444 7%, transparent); }.stream-changes strong { color: var(--text-content); font-size: .62rem; }.stream-changes ul { display: grid; gap: .2rem; margin: .35rem 0; padding-left: 1rem; overflow-wrap: anywhere; }.stream-changes small { color: #f59e0b; font-weight: 800; }
.item-settings { display: grid; gap: .6rem; margin-top: .7rem; border-top: 1px solid var(--border-subtle); padding-top: .7rem; }.override-toggle { display: flex; align-items: center; gap: .4rem; color: var(--text-content); font-size: .62rem; font-weight: 800; }.override-toggle input { accent-color: var(--dynamic-accent); }.item-settings p { color: var(--text-muted); font-size: .58rem; }
.execution-facts { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)) auto; align-items: center; gap: .5rem; border-radius: .75rem; background: var(--bg-input); padding: .65rem; }.execution-facts span, .execution-facts strong, .execution-facts small { display: block; }.execution-facts span, .execution-facts small { color: var(--text-muted); font-size: .53rem; }.execution-facts strong { margin: .12rem 0; color: var(--text-content); font-size: .65rem; }
@media (max-width: 900px) { .facts-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
@media (max-width: 760px) { .video-essentials{display:none} }
@media (max-width: 620px) { .workspace-toolbar { align-items: flex-start; flex-direction: column; }.toolbar-actions, .toolbar-actions button { width: 100%; }.facts-grid, .execution-facts { grid-template-columns: minmax(0, 1fr); }.status { font-size: 0; }.status i { font-size: .75rem; }.settings-heading { align-items: flex-start; flex-direction: column; } }
</style>
