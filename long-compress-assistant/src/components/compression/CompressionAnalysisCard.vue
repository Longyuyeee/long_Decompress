<script setup lang="ts">
import { computed } from 'vue'
import { useCompressionStore, type CompressionOptions } from '@/stores/compression'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { extractErrorMessage } from '@/utils'

const props = defineProps<{
  jobId: string
  paths: string[]
  modelValue: CompressionOptions
  disabled?: boolean
}>()
const emit = defineEmits<{ (event: 'update:modelValue', value: CompressionOptions): void }>()
const store = useCompressionStore()
const commands = useTauriCommands()
const state = computed(() => store.compressionAnalysis[props.jobId])
const result = computed(() => state.value?.result)
const stale = computed(() => state.value?.status === 'completed' && (
  state.value.format !== props.modelValue.format || state.value.level !== props.modelValue.level
))

const formatBytes = (value: number) => {
  if (value < 1024) return `${value} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let size = value / 1024
  let index = 0
  while (size >= 1024 && index < units.length - 1) { size /= 1024; index++ }
  return `${size.toFixed(size >= 100 ? 0 : size >= 10 ? 1 : 2)} ${units[index]}`
}

const confidenceLabel = (confidence?: string) => ({ low: '初步', medium: '中等', high: '较高' }[confidence || 'low'])

const analyze = async () => {
  if (props.disabled || state.value?.status === 'running') return
  const analysisId = `analysis-${Date.now()}-${Math.random().toString(36).slice(2)}`
  store.setAnalysisState(props.jobId, {
    status: 'running', analysisId, format: props.modelValue.format, level: props.modelValue.level,
  })
  try {
    const analysis = await commands.analyzeCompressionSources(
      analysisId, props.paths, props.modelValue.format, props.modelValue.level,
    )
    if (store.compressionAnalysis[props.jobId]?.analysisId !== analysisId) return
    store.setAnalysisState(props.jobId, {
      status: 'completed', analysisId, format: props.modelValue.format,
      level: props.modelValue.level, result: analysis,
    })
  } catch (error) {
    if (store.compressionAnalysis[props.jobId]?.status === 'cancelled') return
    store.setAnalysisState(props.jobId, {
      status: 'failed', analysisId, format: props.modelValue.format,
      level: props.modelValue.level, error: extractErrorMessage(error),
    })
  }
}

const cancel = async () => {
  const analysisId = state.value?.analysisId
  if (!analysisId || state.value?.status !== 'running') return
  store.setAnalysisState(props.jobId, { ...state.value, status: 'cancelled' })
  try { await commands.cancelCompressionAnalysis(analysisId) } catch { /* analysis may have just completed */ }
}

const applyRecommendation = () => {
  if (!result.value || props.disabled) return
  emit('update:modelValue', {
    ...props.modelValue,
    format: result.value.recommendedFormat as CompressionOptions['format'],
    level: result.value.recommendedLevel,
    createSolidArchive: result.value.recommendedFormat === '7z' && result.value.recommendedSolid,
  })
}
</script>

<template>
  <section data-testid="compression-analysis" class="analysis-card">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div class="min-w-0">
        <div class="flex items-center gap-2 text-sm font-black text-content"><i class="pi pi-sparkles text-primary"></i>智能压缩分析</div>
        <p class="mt-1 text-xs text-muted">仅读取最多 2 MiB 内容样本；结果不会自动修改设置</p>
      </div>
      <button v-if="state?.status === 'running'" type="button" class="analysis-button is-cancel" @click="cancel"><i class="pi pi-stop-circle"></i>取消分析</button>
      <button v-else type="button" class="analysis-button" :disabled="disabled" @click="analyze"><i class="pi pi-chart-line"></i>{{ result ? '重新分析' : '分析预计体积' }}</button>
    </div>

    <div v-if="state?.status === 'running'" class="analysis-loading"><i class="pi pi-spin pi-spinner"></i><span>正在统计文件并抽样分析…</span></div>
    <div v-else-if="state?.status === 'failed'" class="analysis-error"><i class="pi pi-exclamation-triangle"></i><span>{{ state.error }}</span></div>
    <div v-else-if="state?.status === 'cancelled'" class="mt-3 text-xs font-bold text-muted">分析已取消，不会影响压缩任务。</div>

    <div v-if="result" class="mt-4 space-y-3" :class="{ 'opacity-60': stale }">
      <div v-if="stale" class="analysis-stale"><i class="pi pi-info-circle"></i>格式或等级已经改变，请重新分析后再参考结果。</div>
      <div class="analysis-metrics">
        <div><span>源数据</span><strong>{{ formatBytes(result.totalSize) }}</strong></div>
        <div><span>预计体积</span><strong>{{ formatBytes(result.estimatedSize) }}</strong></div>
        <div><span>预计节省</span><strong>{{ Math.max(0, Math.round((1 - result.estimatedRatio) * 100)) }}%</strong></div>
        <div><span>典型耗时</span><strong>{{ result.estimatedSecondsLow }}–{{ result.estimatedSecondsHigh }} 秒</strong></div>
        <div v-if="state?.actualSize !== undefined"><span>实际体积</span><strong>{{ formatBytes(state.actualSize) }}</strong></div>
        <div v-if="state?.predictionErrorPercent !== undefined"><span>预测误差</span><strong>{{ state.predictionErrorPercent }}%</strong></div>
      </div>
      <div class="analysis-recommendation">
        <div class="min-w-0">
          <div class="flex flex-wrap items-center gap-2">
            <strong class="text-sm text-content">建议 {{ result.recommendedFormat.toUpperCase() }} · L{{ result.recommendedLevel }}</strong>
            <span class="analysis-badge">{{ confidenceLabel(result.confidence) }}置信度</span>
            <span v-if="result.recommendedSolid" class="analysis-badge">固实压缩</span>
          </div>
          <ul class="mt-2 space-y-1 text-xs text-muted leading-5"><li v-for="reason in result.reasons" :key="reason">· {{ reason }}</li></ul>
          <p class="mt-2 text-xs text-dim">已抽样 {{ formatBytes(result.sampledBytes) }} / {{ result.sampledFiles }} 个文件；低收益内容 {{ result.lowValueFileCount }} 个。</p>
        </div>
        <button type="button" class="analysis-apply" :disabled="disabled || stale" @click="applyRecommendation">采用建议</button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.analysis-card { min-width: 0; border: 1px solid color-mix(in srgb, var(--dynamic-accent) 24%, var(--border-subtle)); border-radius: 1rem; padding: .9rem; background: linear-gradient(145deg, color-mix(in srgb, var(--dynamic-accent) 7%, var(--bg-card)), color-mix(in srgb, var(--bg-input) 55%, transparent)); overflow: hidden; }
.analysis-button, .analysis-apply { min-height: 2.25rem; padding: 0 .8rem; display: inline-flex; align-items: center; justify-content: center; gap: .4rem; border-radius: .7rem; background: var(--dynamic-accent); color: white; font-size: .7rem; font-weight: 900; }
.analysis-button:disabled, .analysis-apply:disabled { opacity: .45; cursor: not-allowed; }
.analysis-button.is-cancel { background: color-mix(in srgb, #ef4444 75%, var(--bg-card)); }
.analysis-loading, .analysis-error, .analysis-stale { margin-top: .75rem; display: flex; align-items: center; gap: .5rem; border-radius: .7rem; padding: .65rem .75rem; font-size: .72rem; font-weight: 800; }
.analysis-loading { color: var(--dynamic-accent); background: color-mix(in srgb, var(--dynamic-accent) 9%, transparent); }
.analysis-error { color: #f87171; background: color-mix(in srgb, #ef4444 10%, transparent); }
.analysis-stale { color: #f59e0b; background: color-mix(in srgb, #f59e0b 10%, transparent); }
.analysis-metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: .5rem; }
.analysis-metrics > div { min-width: 0; border-radius: .7rem; padding: .65rem; background: color-mix(in srgb, var(--bg-input) 62%, transparent); }
.analysis-metrics span, .analysis-metrics strong { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.analysis-metrics span { font-size: .62rem; color: var(--text-muted); font-weight: 800; }
.analysis-metrics strong { margin-top: .2rem; color: var(--text-content); font-size: .8rem; }
.analysis-recommendation { display: flex; align-items: flex-start; justify-content: space-between; gap: .8rem; border-top: 1px solid var(--border-subtle); padding-top: .75rem; }
.analysis-badge { border-radius: 999px; padding: .18rem .45rem; background: color-mix(in srgb, var(--dynamic-accent) 12%, transparent); color: var(--dynamic-accent); font-size: .6rem; font-weight: 900; }
.analysis-apply { flex-shrink: 0; }
@media (max-width: 850px) { .analysis-metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); } .analysis-recommendation { flex-direction: column; } .analysis-apply { width: 100%; } }
</style>
