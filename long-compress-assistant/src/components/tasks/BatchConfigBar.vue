<script setup lang="ts">
import { computed, ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import SavedLocationPicker from '@/components/ui/SavedLocationPicker.vue'
import { useSavedLocations } from '@/composables/useSavedLocations'
import { useAppStore } from '@/stores/app'
import type { BatchConfigPatch } from '@/types/batchConfig'

const props = defineProps<{ count: number; mode: 'compression' | 'decompression' }>()
const emit = defineEmits<{ apply: [patch: BatchConfigPatch]; clear: [] }>()
const app = useAppStore()
const expanded = ref(false)
const format = ref('keep')
const level = ref('keep')
const subfolder = ref('keep')
const destination = ref<'keep' | 'source' | 'custom'>('keep')
const outputDirectory = ref('')
const keep = { value: 'keep', label: '保持各自设置' }
const formats = [keep, { value: 'zip', label: 'ZIP' }, { value: '7z', label: '7Z' }]
const levels = [keep, ...Array.from({ length: 10 }, (_, value) => ({ value: String(value), label: `${value}${value === 0 ? ' · 仅存储' : value === 9 ? ' · 最小体积' : value === 1 ? ' · 更快' : ''}` }))]
const subfolders = [keep, { value: 'yes', label: '创建同名子文件夹' }, { value: 'no', label: '直接放入输出目录' }]
const hasChanges = computed(() => destination.value !== 'keep' || (props.mode === 'compression'
  ? format.value !== 'keep' || level.value !== 'keep' : subfolder.value !== 'keep'))
const canApply = computed(() => hasChanges.value && (destination.value !== 'custom' || !!outputDirectory.value.trim()))
function setOutput(path: string) { outputDirectory.value = path; destination.value = 'custom' }
async function chooseOutput() {
  try {
    const path = await open({ directory: true, multiple: false, title: '选择所选任务的输出目录' })
    if (typeof path === 'string') { setOutput(path); useSavedLocations().remember(path) }
  } catch { app.setError('无法选择输出目录，请重试。') }
}
function apply() {
  if (!canApply.value) return
  const patch: BatchConfigPatch = {}
  if (destination.value !== 'keep') patch.outputDirectory = destination.value === 'source' ? '' : outputDirectory.value.trim()
  if (props.mode === 'compression') {
    if (format.value === 'zip' || format.value === '7z') patch.format = format.value
    if (level.value !== 'keep') patch.level = Number(level.value)
  } else if (subfolder.value !== 'keep') patch.extractToSubfolder = subfolder.value === 'yes'
  emit('apply', patch)
}
</script>

<template>
  <section v-if="count" class="batch-config-bar min-w-0 rounded-xl border border-primary/25 bg-primary/5 p-3" data-testid="batch-config-bar">
    <div class="flex flex-wrap items-center gap-2">
      <span class="mr-auto text-sm font-semibold text-content">已选 {{ count }} 项</span>
      <button type="button" class="batch-button bg-primary text-white" :aria-expanded="expanded" @click="expanded = !expanded">
        <i class="pi pi-sliders-h" aria-hidden="true"></i>修改所选配置<i :class="expanded ? 'pi pi-chevron-up' : 'pi pi-chevron-down'" aria-hidden="true"></i>
      </button>
      <button type="button" class="batch-button text-muted hover:bg-input" @click="emit('clear')">取消选择</button>
    </div>
    <div v-if="expanded" class="mt-3 space-y-3 border-t border-primary/15 pt-3" data-testid="batch-config-fields">
      <p class="text-xs leading-relaxed text-muted">只修改下面指定的项目，其他配置保持各自原值。</p>
      <div class="grid min-w-0 gap-3 sm:grid-cols-2">
        <label v-if="mode === 'compression'" class="min-w-0 space-y-1.5 text-xs text-muted">压缩格式
          <ThemedSelect v-model="format" :options="formats" aria-label="批量压缩格式" test-id="batch-format" floating />
        </label>
        <label v-if="mode === 'compression'" class="min-w-0 space-y-1.5 text-xs text-muted">压缩等级
          <ThemedSelect v-model="level" :options="levels" aria-label="批量压缩等级" test-id="batch-level" floating />
        </label>
        <label v-else class="min-w-0 space-y-1.5 text-xs text-muted">文件放置方式
          <ThemedSelect v-model="subfolder" :options="subfolders" aria-label="批量解压子文件夹" test-id="batch-subfolder" floating />
        </label>
      </div>
      <div class="min-w-0 space-y-2">
        <p class="text-xs text-muted">输出目录</p>
        <div class="flex flex-wrap items-center gap-2">
          <button v-for="option in [{ value: 'keep', label: '保持各自目录' }, { value: 'source', label: '各自源文件目录' }]" :key="option.value" type="button" class="batch-button border" :class="destination === option.value ? 'border-primary/40 bg-primary/10 text-primary' : 'border-subtle text-muted hover:bg-input'" :aria-pressed="destination === option.value" @click="destination = option.value as 'keep' | 'source'">{{ option.label }}</button>
          <button type="button" class="batch-button border border-subtle text-content hover:bg-input" @click="chooseOutput"><i class="pi pi-folder-open" aria-hidden="true"></i>选择目录</button>
          <SavedLocationPicker :current="destination === 'custom' ? outputDirectory : ''" @select="setOutput" />
        </div>
        <p v-if="destination === 'custom'" class="select-text break-words rounded-lg bg-input/60 px-3 py-2 text-xs text-content [overflow-wrap:anywhere]" :title="outputDirectory">{{ outputDirectory }}</p>
      </div>
      <div class="flex flex-wrap items-center justify-end gap-2">
        <button type="button" class="batch-button bg-primary font-semibold text-white disabled:opacity-40" data-testid="batch-config-apply" :disabled="!canApply" @click="apply">应用到 {{ count }} 项</button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.batch-button { display:inline-flex; min-height:2rem; align-items:center; justify-content:center; gap:.5rem; border-radius:.6rem; padding:.4rem .7rem; font-size:.75rem; transition:background .15s,opacity .15s; }
.batch-button:focus-visible { outline:2px solid var(--dynamic-accent); outline-offset:2px; }
.batch-button i { font-size:.75rem; }
</style>
