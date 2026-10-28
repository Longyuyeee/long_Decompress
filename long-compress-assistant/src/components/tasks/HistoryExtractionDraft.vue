<script setup lang="ts">
import {checkZpaqRetrieval} from '@/utils/zpaqRetrieval'
import {detectArchiveGroup} from '@/utils/archiveSource'
import { computed, ref, watch, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { open } from '@tauri-apps/api/dialog'
import { restoredExtractionConfig, type TaskHistoryRecord } from '@/types/taskHistory'
import { useTaskStore } from '@/stores/task'
import { generateId, extractErrorMessage } from '@/utils'
import { isDecompressArchivePath, isPotentialSplitArchivePath, isPotentialSfxPath } from '@/utils/compressionFormat'
import ComparisonPasswordField from '@/components/archive/ComparisonPasswordField.vue'
import ExtractionOutputPreview from './ExtractionOutputPreview.vue'
import ZipPasswordEncodingField from '@/components/ui/ZipPasswordEncodingField.vue'
import type { ZipPasswordEncoding } from '@/types/zipPassword'
import { supportsZipPasswordEncoding, zipPasswordEncodingForSource } from '@/utils/zipPasswordSource'

const props = defineProps<{ record: TaskHistoryRecord }>()
const store = useTaskStore()
const source = ref('')
const destination = ref('')
const extractToSubfolder = ref(true)
const password = ref('')
const zipPasswordEncoding = ref<ZipPasswordEncoding>()
const sourceFormat = computed(() => source.value === props.record.sourcePaths[0] ? props.record.format : undefined)
const restoredConfig = computed(() => restoredExtractionConfig(props.record))
const busy = ref(false)
const error = ref('')
const created = ref(false)
let disposed = false
onUnmounted(() => { disposed = true; password.value = '' })
watch(() => props.record.id, () => {
  source.value = props.record.sourcePaths[0] || ''
  destination.value = restoredConfig.value?.outputDirectory || ''
  extractToSubfolder.value = restoredConfig.value?.extractToSubfolder ?? true
  zipPasswordEncoding.value = restoredConfig.value?.comparisonSelection?.zipPasswordEncoding ?? restoredConfig.value?.zipPasswordEncoding
  password.value = ''
  error.value = ''
  created.value = false
}, { immediate: true })

async function selectPath(directory: boolean) {
  const recordId = props.record.id
  try {
    const queued = import.meta.env.VITE_DESKTOP_E2E === '1'
      ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection()
      : undefined
    const path = queued !== undefined ? queued
      : await open({ directory, multiple: false, title: directory ? '选择本次输出目录' : '选择归档或首个分卷' })
    if (!disposed && props.record.id === recordId && typeof path === 'string') {
      if (directory) destination.value = path
      else source.value = path
    }
  } catch (cause) { error.value = extractErrorMessage(cause) }
}

async function createDraft() {
  if (busy.value || created.value) return
  error.value = ''
  let path = source.value.trim()
  const outputPath = destination.value.trim()
  const confirmedSubfolder = extractToSubfolder.value
  const confirmedPassword = password.value
  const confirmedEncoding = zipPasswordEncoding.value
  const recordId = props.record.id
  if (!path || !outputPath) { error.value = '请确认来源文件并选择本次输出目录'; return }
  if (restoredConfig.value?.tarSelection && path !== props.record.sourcePaths[0]) { error.value = '请返回双栏浏览器重新选择来源和文件'; return }
  if(restoredConfig.value?.comparisonSelection){
    const selection={...restoredConfig.value.comparisonSelection,...(confirmedEncoding?{zipPasswordEncoding:confirmedEncoding}:{})}
    if(path!==selection.source.path){error.value='对比来源位置不可在这里替换，请返回资料对比重新读取';return}
    busy.value=true
    try{await invoke('comparison_validate',{request:selection,output:outputPath,password:confirmedPassword||null});if(disposed||props.record.id!==recordId)return
      store.addTask({id:generateId(),name:props.record.name,type:'decompression',sourceFiles:[path],outputPath,configurationMode:'individual',extractToSubfolder:false,recycleSourceAfterExtract:false,password:confirmedPassword||undefined,zipPasswordEncoding:confirmedEncoding,comparisonSelection:JSON.parse(JSON.stringify(selection)),selectedEntries:[...selection.paths],executionConfig:{...restoredConfig.value,outputDirectory:outputPath},retryOfTaskId:recordId,format:selection.source.kind==='zip'?'ZIP':'文件夹'});created.value=true
    }catch(cause){error.value=extractErrorMessage(cause)}finally{busy.value=false}
    return
  }
  if (!isDecompressArchivePath(path) && !isPotentialSplitArchivePath(path) && !isPotentialSfxPath(path)) {
    error.value = '请选择支持的归档文件或首个分卷'
    return
  }
  busy.value = true
  try {
    const split = await detectArchiveGroup(path,true)
    if (split?.is_split) {
      if (!split.first_part || split.is_complete === false || split.missing_parts?.length) {
        throw new Error(`分卷不完整，请补齐后重试：${split.missing_parts?.join('、') || '缺少首卷或必要分卷'}`)
      }
      path = split.first_part
    } else if (isPotentialSplitArchivePath(path) || (!isDecompressArchivePath(path) && !(isPotentialSfxPath(path) && split?.archive_format))) {
      throw new Error('无法确认分卷组，请选择归档首卷')
    }
    const file = await invoke<{ name: string; is_dir: boolean; size: number }>('get_file_info', { path })
    if (file.is_dir || file.size === 0) throw new Error('来源必须是非空归档文件；请确认下载完成')
    try {
      const directory = await invoke<{ is_dir: boolean }>('get_file_info', { path: outputPath })
      if (!directory.is_dir) throw new Error('not a directory')
    } catch {
      throw new Error('输出目录已不存在或无法访问，请重新选择本次输出目录。')
    }
    // A selection changed during the asynchronous check must not create the old draft.
    if (disposed || props.record.id !== recordId) return
    if(restoredConfig.value?.zpaqRetrieval)await checkZpaqRetrieval(path,confirmedPassword,restoredConfig.value.zpaqRetrieval,generateId())
    if(restoredConfig.value?.readerSelection) await invoke('check_image_reader_source',{filePath:path,revision:restoredConfig.value.readerSelection.revision})
    if(restoredConfig.value?.librarySourceId) {const row=await invoke<{path:string}>('library_selection',{sourceId:restoredConfig.value.librarySourceId,revision:restoredConfig.value.libraryRevision,paths:restoredConfig.value.selectedEntries});if(row.path.toLowerCase()!==path.toLowerCase())throw Error('来源位置已更新，请从资料库重新选择文件')}
    if (restoredConfig.value?.zpaqCatalogId && restoredConfig.value.zpaqSnapshot) await invoke('check_zpaq_snapshot_catalog',{filePath:path,password:confirmedPassword||null,snapshot:restoredConfig.value.zpaqSnapshot,catalogId:restoredConfig.value.zpaqCatalogId})
    if (disposed || props.record.id !== recordId) return
    const id = store.addTask({
      id: generateId(), name: restoredConfig.value?.zpaqRetrieval?props.record.name:file.name, type: 'decompression', workloadKind: 'archive',
      zpaqRetrieval:restoredConfig.value?.zpaqRetrieval,
      retryOfTaskId: recordId, zpaqSnapshot: restoredConfig.value?.zpaqSnapshot, zpaqCatalogId: restoredConfig.value?.zpaqCatalogId, selectedEntries: restoredConfig.value?.tarSelection?.paths || restoredConfig.value?.zpaqRetrieval?.paths || restoredConfig.value?.readerSelection?.paths || restoredConfig.value?.selectedEntries,
      readerSelection:restoredConfig.value?.readerSelection,
      librarySourceId:restoredConfig.value?.librarySourceId,libraryRevision:restoredConfig.value?.libraryRevision,libraryKeepStructure:restoredConfig.value?.libraryKeepStructure,libraryConflictPolicy:restoredConfig.value?.libraryConflictPolicy,
      executionConfig: restoredConfig.value ? {...restoredConfig.value,outputDirectory:outputPath,extractToSubfolder:confirmedSubfolder}:undefined,
      sourceFiles: [path], outputPath, configurationMode: 'individual',
      extractToSubfolder: confirmedSubfolder, recycleSourceAfterExtract: false,
      password: confirmedPassword || undefined,
      zipPasswordEncoding: zipPasswordEncodingForSource(path, confirmedEncoding, split?.archive_format),
      format: split?.archive_format || (restoredConfig.value?.tarSelection ? 'TAR.LZO' : undefined),
    })
    store.tasks.find(task => task.id === id)!.logs.push({
      task_id: id, severity: 'info', timestamp: new Date().toISOString(),
      message: `从历史任务 ${recordId} 创建${restoredConfig.value?.tarSelection?"TAR.LZO 所选文件":restoredConfig.value?.zpaqRetrieval?"部分取回所选文件":restoredConfig.value?.readerSelection ? "阅读所选图片" : restoredConfig.value?.librarySourceId ? "资料库所选文件" : restoredConfig.value?.zpaqCatalogId ? "所选变化" : "完整解压"}草稿；输出目录和子目录使用本次确认的设置。`,
    })
    created.value = true
    password.value = ''
  } catch (cause) {
    if (props.record.id === recordId) error.value = `无法创建草稿：${extractErrorMessage(cause)}`
  } finally { busy.value = false }
}
</script>

<template>
  <section class="detail-section" data-testid="history-extraction-draft">
    <h3>再次解压</h3>
    <p class="text-xs text-muted my-3">确认下方设置，再添加到队列。新任务不会自动开始。</p>
    <p v-if="restoredConfig" data-testid="history-config-restored" class="text-xs text-primary mb-3">已带入上次的解压设置，可直接使用或修改。</p>
    <p v-else data-testid="history-config-missing" class="text-xs text-muted mb-3">这条历史没有可恢复的设置，请选择输出目录，并确认是否创建同名子目录。</p>
    <p v-if="restoredConfig?.tarSelection" class="text-xs text-primary mb-3" data-testid="history-tar-selection">保留上次所选 {{ restoredConfig.tarSelection.paths.length }} 项、目录方式和同名策略。</p>
    <p v-if="restoredConfig?.zpaqRetrieval" class="text-xs text-primary mb-3" data-testid="history-zpaq-retrieval">保留快照 {{restoredConfig.zpaqRetrieval.snapshot.version}} 的 {{restoredConfig.zpaqRetrieval.paths.length}} 个所选文件，另存独立结果。来源变化时请返回归档诊断重新选择。</p>
    <p v-if="restoredConfig?.readerSelection" class="text-xs text-primary mb-3" data-testid="history-reader-selection">保留阅读时所选 {{ restoredConfig.readerSelection.paths.length }} 张图片、结构与同名策略；来源变化后请返回浏览中心重新选择，不扩大为整包提取。</p>
    <p v-if="restoredConfig?.comparisonSelection" class="text-xs text-primary mb-3" data-testid="history-comparison-selection">保留资料对比所选 {{ restoredConfig.comparisonSelection.paths.length }} 项、结构与同名策略。来源变化后请<a href="#/compare" class="underline">返回资料对比重新读取</a>。</p>
    <p v-if="restoredConfig?.librarySourceId" class="text-xs text-primary mb-3" data-testid="history-library-selection">保留资料库所选 {{ restoredConfig.selectedEntries?.length }} 个文件、目录方式和同名策略；来源变化时请返回资料库重新选择。<a href="#/library" class="underline">返回资料库</a></p>
    <p v-if="restoredConfig?.zpaqCatalogId" class="text-xs text-primary">保留所选 {{ restoredConfig.selectedEntries?.length }} 项与来源目录标识，不扩大为整包提取。来源变化后请重新比较。</p>
    <p v-if="restoredConfig?.zpaqSnapshot" class="text-xs text-primary mb-3" data-testid="history-zpaq-snapshot">沿用快照 {{ restoredConfig.zpaqSnapshot.version }}，不会改为最新内容；如需其他版本，请从归档浏览重新选择。</p>
    <p v-if="!restoredConfig?.tarSelection && !restoredConfig?.zpaqRetrieval && !restoredConfig?.readerSelection && !restoredConfig?.comparisonSelection && !restoredConfig?.zpaqCatalogId && !restoredConfig?.librarySourceId" class="text-xs text-muted mb-3">本次解压整个归档并保留源文件，不恢复局部选择。密码可重新填写；留空时按当前密码设置尝试，同名文件按当前冲突设置处理。</p>
    <p v-if="record.sourcePaths.length > 1" class="text-xs text-amber-500 mb-2">原记录包含多个来源，请确认此处为归档或首个分卷；本次只创建一个任务。</p>
    <fieldset :disabled="busy || created" class="space-y-2 min-w-0">
      <label class="block text-sm">来源文件
        <input v-model="source" :readonly="!!(restoredConfig?.comparisonSelection || restoredConfig?.tarSelection)" data-testid="history-draft-source" class="history-control w-full mt-1" />
      </label>
      <button v-if="!restoredConfig?.comparisonSelection && !restoredConfig?.tarSelection" class="history-control" @click="selectPath(false)">重新选择文件</button>
      <label class="block text-sm">本次输出目录
        <input v-model="destination" data-testid="history-draft-output" class="history-control w-full mt-1" placeholder="选择或输入输出目录" />
      </label>
      <button data-testid="history-draft-select-output" class="history-control" @click="selectPath(true)">选择输出目录</button>
      <label v-if="!restoredConfig?.comparisonSelection && !restoredConfig?.zpaqRetrieval" class="flex items-center gap-2 text-sm"><input v-model="extractToSubfolder" data-testid="history-draft-subfolder" type="checkbox" />解压到同名子目录</label>
      <label v-if="!restoredConfig?.comparisonSelection" class="block text-sm">本次密码（可留空）
        <input v-model="password" data-testid="history-draft-password" type="password" autocomplete="off" class="history-control w-full mt-1" />
      </label>
      <ZipPasswordEncodingField v-if="!restoredConfig?.comparisonSelection && supportsZipPasswordEncoding(source, sourceFormat)" v-model="zipPasswordEncoding" :source-path="source" :source-format="sourceFormat" :disabled="busy || created" test-id="history-draft-zip-password-encoding" />
      <ComparisonPasswordField v-if="restoredConfig?.comparisonSelection?.source.encrypted" v-model:password="password" v-model:encoding="zipPasswordEncoding" :disabled="busy||created" test-id="history-comparison-password"/>
      <ExtractionOutputPreview v-if="!restoredConfig?.zpaqRetrieval" :source="source.trim()" :output="destination.trim()" :subfolder="extractToSubfolder" @change-output="selectPath(true)" />
      <button data-testid="history-draft-create" class="history-control w-full" @click="createDraft">{{ busy ? '正在检查来源…' : '创建待执行任务' }}</button>
    </fieldset>
    <p v-if="error" role="alert" class="text-sm text-red-500 mt-2 break-words">{{ error }}</p>
    <p v-if="created" role="status" class="text-sm mt-3">草稿已创建。<a href="#/decompress" class="text-primary underline">前往解压中心确认并启动</a></p>
  </section>
</template>

<style scoped>
.history-control { border: 1px solid var(--border-subtle); border-radius: .75rem; padding: .6rem .8rem; color: var(--text-base); background: var(--bg-card); min-width: 0; }
fieldset:disabled { opacity: .65; }
button:not(:disabled):hover { border-color: var(--dynamic-accent); }
</style>
