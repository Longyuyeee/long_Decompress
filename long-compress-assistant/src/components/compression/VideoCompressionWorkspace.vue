<script setup lang="ts">
import { computed, watch } from 'vue'
import { useAppStore } from '@/stores/app'
import { useCompressionStore, type VideoCompressionItem } from '@/stores/compression'
import { useTauriCommands } from '@/composables/useTauriCommands'
import type { VideoCompressionSettings } from '@/types/video'
import type { VideoCandidate } from '@/utils/videoCompressionWorkspace'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import VideoCompressionSettingsPanel from './VideoCompressionSettingsPanel.vue'

const appStore = useAppStore()
const store = useCompressionStore()
const commands = useTauriCommands()
const inFlight = new Map<string, number>()

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

const updateGlobalSettings = (settings: VideoCompressionSettings) => store.updateVideoGlobalSettings(settings)
const updateItemSettings = (item: VideoCompressionItem, settings: VideoCompressionSettings) => store.updateVideoItemSettings(item.id, settings)
const toggleOverride = (item: VideoCompressionItem, enabled: boolean) => {
  if (enabled) store.enableVideoItemOverride(item.id)
  else store.disableVideoItemOverride(item.id)
}
const planningCount = computed(() => store.videoItems.filter(item => item.status === 'planning').length)
const readyCount = computed(() => store.videoItems.filter(item => item.status === 'ready').length)
</script>

<template>
  <section class="video-workspace" data-testid="video-compression-workspace">
    <div class="workspace-toolbar">
      <div class="min-w-0">
        <div class="title-line"><i class="pi pi-video"></i><strong>视频压缩工作区</strong><span>探测与配置</span></div>
        <p>读取产品 ffprobe 的真实输入事实；当前节点不会创建任务或启动编码。</p>
      </div>
      <button type="button" class="execute-disabled" disabled title="C-03 完成执行、进度、取消审计后开放"><i class="pi pi-lock"></i>开始视频压缩 · C-03</button>
    </div>

    <div class="truth-boundary">
      <i class="pi pi-shield"></i>
      <span>预计大小始终是估算；字幕、多音轨、章节、封面和 HDR 变化由后端明确报告，不会静默处理。</span>
      <strong v-if="store.videoItems.length">{{ readyCount }} 就绪<span v-if="planningCount"> · {{ planningCount }} 探测中</span></strong>
    </div>

    <div class="global-settings-card">
      <div class="settings-heading"><span><i class="pi pi-sliders-h"></i>批量配置</span><small>单项可展开覆盖；修改后重新探测和规划</small></div>
      <VideoCompressionSettingsPanel :model-value="store.videoGlobalSettings" @update:model-value="updateGlobalSettings" />
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

    <div v-else class="video-list">
      <article v-for="item in store.videoItems" :key="item.id" class="video-card" :data-status="item.status" data-testid="video-draft-card">
        <header>
          <button type="button" class="expand" :aria-expanded="item.expanded" @click="item.expanded = !item.expanded"><i :class="item.expanded ? 'pi pi-chevron-down' : 'pi pi-chevron-right'"></i></button>
          <div class="file-identity"><strong :title="item.path">{{ item.name }}</strong><small>{{ formatBytes(item.inputSize) }} · {{ item.path }}</small></div>
          <span class="status"><i :class="item.status === 'planning' ? 'pi pi-spin pi-spinner' : item.status === 'ready' ? 'pi pi-check-circle' : 'pi pi-times-circle'"></i>{{ item.status === 'planning' ? '正在探测' : item.status === 'ready' ? '规划就绪' : '已拒绝' }}</span>
          <button type="button" class="remove" title="移除" @click="store.removeVideoItem(item.id)"><i class="pi pi-trash"></i></button>
        </header>

        <div v-if="item.error" class="probe-error"><i class="pi pi-exclamation-triangle"></i><span>{{ item.error }}</span><button type="button" @click="store.retryVideoPlanning(item.id)">重试</button></div>

        <template v-if="item.plan">
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
          <p v-else>当前跟随批量配置：{{ store.getEffectiveVideoSettings(item).preset }}。</p>
        </div>
      </article>

      <EnhancedFileDropzone compact mode="file" accept="mp4,mov,avi,wmv,webm,mkv,m4v" unfiltered-picker hint="继续添加视频" @files-selected="onFilesSelected" />
    </div>
  </section>
</template>

<style scoped>
.video-workspace { box-sizing: border-box; display: flex; width: 100%; max-width: 100%; min-width: 0; min-height: 0; flex: 1; flex-direction: column; gap: .75rem; overflow-x: hidden; overflow-y: auto; padding: .1rem; }
.video-workspace > * { box-sizing: border-box; max-width: 100%; min-width: 0; }
.workspace-toolbar { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: .75rem 1rem; }
.workspace-toolbar > div { min-width: 0; flex: 1 1 20rem; }
.title-line { display: flex; align-items: center; gap: .45rem; color: var(--text-content); }
.title-line i { color: var(--dynamic-accent); }
.title-line strong { font-size: .88rem; font-weight: 900; }
.title-line span { border-radius: 999px; background: color-mix(in srgb, var(--dynamic-accent) 12%, transparent); padding: .18rem .45rem; color: var(--dynamic-accent); font-size: .58rem; font-weight: 900; }
.workspace-toolbar p { margin-top: .2rem; color: var(--text-muted); font-size: .65rem; }
.execute-disabled { max-width: 100%; flex: 0 1 auto; border: 1px solid var(--border-subtle); border-radius: .75rem; background: var(--bg-input); padding: .65rem .8rem; color: var(--text-muted); font-size: .65rem; font-weight: 850; opacity: .7; }
.truth-boundary { display: flex; align-items: center; gap: .5rem; border: 1px solid color-mix(in srgb, var(--dynamic-accent) 22%, transparent); border-radius: .8rem; background: color-mix(in srgb, var(--dynamic-accent) 7%, transparent); padding: .6rem .75rem; color: var(--text-muted); font-size: .62rem; line-height: 1.45; }
.truth-boundary i, .truth-boundary strong { color: var(--dynamic-accent); }.truth-boundary strong { margin-left: auto; white-space: nowrap; }
.global-settings-card, .video-card { box-sizing: border-box; max-width: 100%; min-width: 0; border: 1px solid var(--border-subtle); border-radius: 1rem; background: color-mix(in srgb, var(--bg-card) 88%, transparent); padding: .85rem; }
.settings-heading { display: flex; justify-content: space-between; gap: .75rem; margin-bottom: .65rem; color: var(--text-content); font-size: .68rem; font-weight: 900; }.settings-heading small { color: var(--text-muted); font-size: .58rem; font-weight: 650; }
.video-empty { min-height: 14rem; }.video-list { display: grid; width: 100%; max-width: 100%; min-width: 0; gap: .65rem; }.video-card { overflow: hidden; }
.video-card header { display: flex; min-width: 0; align-items: center; gap: .55rem; }.expand, .remove { flex: 0 0 auto; width: 1.8rem; height: 1.8rem; border-radius: .55rem; color: var(--text-muted); }.expand:hover, .remove:hover { background: var(--bg-input); color: var(--text-content); }
.file-identity { min-width: 0; flex: 1; }.file-identity strong, .file-identity small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }.file-identity strong { color: var(--text-content); font-size: .72rem; }.file-identity small { margin-top: .12rem; color: var(--text-muted); font-size: .55rem; }
.status { display: flex; flex: 0 0 auto; align-items: center; gap: .3rem; border-radius: 999px; background: var(--bg-input); padding: .28rem .48rem; color: var(--text-muted); font-size: .58rem; font-weight: 850; }.video-card[data-status="ready"] .status { color: #22c55e; }.video-card[data-status="rejected"] .status { color: #ef4444; }
.probe-error { display: flex; align-items: center; gap: .5rem; margin-top: .65rem; border-radius: .7rem; background: color-mix(in srgb, #ef4444 9%, transparent); padding: .6rem; color: #ef4444; font-size: .6rem; }.probe-error span { min-width: 0; flex: 1; overflow-wrap: anywhere; }.probe-error button { border: 1px solid currentColor; border-radius: .5rem; padding: .25rem .45rem; font-weight: 800; }
.facts-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: .5rem; margin-top: .7rem; }.facts-grid > div { min-width: 0; border-radius: .7rem; background: var(--bg-input); padding: .6rem; }.facts-grid span, .facts-grid strong, .facts-grid small { display: block; }.facts-grid span { color: var(--text-muted); font-size: .55rem; font-weight: 800; }.facts-grid strong { margin-top: .18rem; color: var(--text-content); font-size: .7rem; overflow-wrap: anywhere; }.facts-grid small { margin-top: .16rem; color: var(--text-muted); font-size: .53rem; line-height: 1.4; }.facts-grid .estimate { outline: 1px solid color-mix(in srgb, var(--dynamic-accent) 22%, transparent); }.facts-grid .estimate span, .facts-grid .estimate strong { color: var(--dynamic-accent); }
.estimate-disclaimer { display: flex; gap: .35rem; margin-top: .5rem; color: var(--text-muted); font-size: .55rem; font-style: italic; }
.stream-changes { margin-top: .6rem; border-left: 3px solid #f59e0b; border-radius: .45rem; background: color-mix(in srgb, #f59e0b 7%, transparent); padding: .55rem .7rem; color: var(--text-muted); font-size: .56rem; }.stream-changes.blocked { border-left-color: #ef4444; background: color-mix(in srgb, #ef4444 7%, transparent); }.stream-changes strong { color: var(--text-content); font-size: .62rem; }.stream-changes ul { display: grid; gap: .2rem; margin: .35rem 0; padding-left: 1rem; overflow-wrap: anywhere; }.stream-changes small { color: #f59e0b; font-weight: 800; }
.item-settings { display: grid; gap: .6rem; margin-top: .7rem; border-top: 1px solid var(--border-subtle); padding-top: .7rem; }.override-toggle { display: flex; align-items: center; gap: .4rem; color: var(--text-content); font-size: .62rem; font-weight: 800; }.override-toggle input { accent-color: var(--dynamic-accent); }.item-settings p { color: var(--text-muted); font-size: .58rem; }
@media (max-width: 900px) { .facts-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
@media (max-width: 620px) { .workspace-toolbar { align-items: flex-start; flex-direction: column; }.execute-disabled { width: 100%; }.truth-boundary { align-items: flex-start; flex-wrap: wrap; }.truth-boundary strong { width: 100%; margin-left: 0; }.facts-grid { grid-template-columns: minmax(0, 1fr); }.status { font-size: 0; }.status i { font-size: .75rem; }.settings-heading { align-items: flex-start; flex-direction: column; } }
</style>
