<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import { computed, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/api/dialog'
import { useRouter } from 'vue-router'
import Modal from '@/components/ui/Modal.vue'
import { useTaskStore } from '@/stores/task'
import { archiveSourceStem, detectArchiveGroup } from '@/utils/archiveSource'
import { formatFileSize } from '@/utils'
import { snapshotDate, type ZpaqHistory, type ZpaqSnapshot } from '@/types/zpaqSnapshot'
import { defaultDiffSelection, diffLabels, mergeVerifiedComparison, selectedDiffGroups, selectionKey, type DiffKind, type ZpaqComparison } from '@/types/zpaqDiff'
const props=defineProps<{visible:boolean;path:string;password:string;history:ZpaqHistory;outputDirectory:string}>()
const emit=defineEmits<{ 'update:visible':[boolean] }>()
const router=useRouter(),tasks=useTaskStore()
let generation=0
const visible=computed({get:()=>props.visible,set:value=>emit('update:visible',value)})
const from=ref(1),to=ref(2),result=ref<ZpaqComparison|null>(null),selected=ref(new Set<string>())
const query=ref(''),filter=ref('differences'),destination=ref(''),error=ref(''),notice=ref(''),request=ref(''),creating=ref(false),created=ref(0)
const page=ref(1)
const progress=ref<{phase:string;processedBytes:number;totalBytes:number}|null>(null)
const busy=computed(()=>!!request.value||creating.value)
const groups=computed(()=>result.value?selectedDiffGroups(result.value,selected.value):[])
const candidates=computed(()=>result.value?.entries.filter(e=>e.kind==='candidate'&&(selected.value.has(selectionKey('before',e.path))||selected.value.has(selectionKey('after',e.path)))).map(e=>e.path)||[])
const rows=computed(()=>result.value?.entries.filter(e=>(filter.value==='all'||filter.value==='differences'&&e.kind!=='unchanged'||e.kind===filter.value)&&e.path.toLowerCase().includes(query.value.toLowerCase()))||[])
const pageCount=computed(()=>Math.max(1,Math.ceil(rows.value.length/200)))
const pageRows=computed(()=>rows.value.slice((page.value-1)*200,page.value*200))
watch([query,filter,result],()=>{page.value=1})
const phaseText=computed(()=>({catalog:'读取两版目录', 'before-decode':'临时提取旧版所选候选','before-hash':'核对旧版内容','after-decode':'临时提取新版所选候选','after-hash':'核对新版内容',complete:'比较完成'}[progress.value?.phase||'catalog']))
function initialize(){
  generation++
  const versions=props.history.snapshots,at=Math.max(1,versions.findIndex(s=>s.version===props.history.selected.version))
  from.value=versions[at-1]?.version||versions[0].version;to.value=versions[at]?.version||versions.at(-1)!.version
  result.value=null;selected.value=new Set();error.value=notice.value='';created.value=0;destination.value=props.outputDirectory;query.value='';filter.value='differences'
}
async function stop(){const id=request.value;request.value='';if(id){notice.value='已停止本次比较；没有创建或执行提取任务。';try{await invoke('cancel_archive_browse',{browseId:id})}catch(cause){error.value='停止请求失败：'+String(cause)}}}
watch(()=>props.visible,value=>{if(value)initialize();else {generation++;void stop()}},{immediate:true})
watch(()=>[props.path,props.password],()=>{void stop();if(props.visible)initialize()})
onUnmounted(()=>{void stop()})
function clear(){result.value=null;selected.value=new Set();created.value=0;notice.value=error.value=''}
function toggle(side:'before'|'after',path:string){const key=selectionKey(side,path);const next=new Set(selected.value);next.has(key)?next.delete(key):next.add(key);selected.value=next}
async function compare(exact=false){
  if(busy.value)return
  const before=props.history.snapshots.find(s=>s.version===from.value),after=props.history.snapshots.find(s=>s.version===to.value)
  if(!before||!after||before.version>=after.version){error.value='请选择较早和较新的两个不同快照。';return}
  if(exact&&!candidates.value.length)return
  const id=crypto.randomUUID(),paths=exact?[...candidates.value]:[],prior=exact?result.value:null
  error.value=notice.value='';created.value=0;request.value=id;progress.value={phase:'catalog',processedBytes:0,totalBytes:0};let unlisten:(()=>void)|undefined
  try{
    unlisten=await listen<{requestId:string;phase:string;processedBytes:number;totalBytes:number}>('zpaq-comparison-progress',({payload})=>{if(payload.requestId===request.value)progress.value=payload})
    if(request.value!==id)return
    const next=await invoke<ZpaqComparison>('compare_zpaq_snapshots',{filePath:props.path,password:props.password||null,browseId:id,before:{version:before.version,date:before.date},after:{version:after.version,date:after.date},verifyPaths:paths,expectedRevision:prior?.revision||null})
    if(request.value!==id)return
    result.value=mergeVerifiedComparison(prior,next)
    if(!exact){page.value=1;selected.value=defaultDiffSelection({...next,entries:rows.value.slice(0,200)})}
    notice.value=exact?'所选候选已按实际内容核对。':'目录比较完成，默认选择第一页变化；候选可按需精确核对。'
  }catch(cause){if(request.value===id){if(exact)clear();error.value=String(cause)}}
  finally{unlisten?.();if(request.value===id)request.value=''}
}
async function chooseOutput(){try{
  const queued=import.meta.env.VITE_DESKTOP_E2E==='1'?window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection():undefined
  const chosen=queued===undefined?await open({directory:true,multiple:false,title:'选择变化文件的输出目录'}):queued
  if(typeof chosen==='string')destination.value=chosen
}catch(cause){error.value=String(cause)}}
async function createDrafts(){
  if(busy.value||!result.value||!groups.value.length||created.value)return
  error.value='';creating.value=true
  const token=generation,sourcePath=props.path,sourcePassword=props.password,outputDirectory=destination.value.trim(),prepared=[...groups.value]
  try{
    const group=await detectArchiveGroup(sourcePath),source=group?.is_split?group.first_part:sourcePath
    if(!source||group?.is_split&&(!group.is_complete||group.missing_parts?.length))throw Error('分卷不完整，请补齐后重新比较。')
    const directory=await invoke<{is_dir:boolean}>('get_file_info',{path:outputDirectory});if(!directory.is_dir)throw Error('请选择已存在的输出目录。')
    for(const g of prepared){
      if(g.paths.length>2000||JSON.stringify(g.paths).length>60000||g.paths.some(p=>p.length>32768))throw Error('所选路径过多或过长，请分批创建任务。')
      await invoke('check_zpaq_snapshot_catalog',{filePath:source,password:sourcePassword||null,snapshot:g.snapshot,catalogId:g.catalogId})
    }
    if(!props.visible||token!==generation)return
    const stamp=Date.now()+'_'+crypto.randomUUID().slice(0,6)
    for(const g of prepared){
      const outputPath=outputDirectory.replace(/[\\/]$/,'')+`/ZPAQ变化_第${g.snapshot.version}版_${stamp}`
      tasks.addTask({id:crypto.randomUUID(),name:`${archiveSourceStem(source)} · 变化提取（快照 ${g.snapshot.version}）`,type:'decompression',workloadKind:'archive',sourceFiles:[source],outputPath,format:'ZPAQ',configurationMode:'individual',extractToSubfolder:false,recycleSourceAfterExtract:false,password:sourcePassword||undefined,zpaqSnapshot:{...g.snapshot},zpaqCatalogId:g.catalogId,selectedEntries:[...g.paths],executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:outputPath,extractToSubfolder:false,zpaqSnapshot:{...g.snapshot},zpaqCatalogId:g.catalogId,selectedEntries:[...g.paths]}})
    }
    created.value=prepared.length;notice.value=`已创建 ${prepared.length} 个待处理任务，按版本分别保存；请到解压中心核对后开始。`
  }catch(cause){error.value=String(cause)}finally{creating.value=false}
}
function review(){visible.value=false;void router.push('/decompress')}
const date=(snapshot:ZpaqSnapshot)=>`第 ${snapshot.version} 版 · ${snapshotDate(snapshot.date)}`
</script>
<template>
  <Modal v-model:visible="visible" title="ZPAQ 快照对比" icon="pi pi-clone" size="xl">
    <div class="min-w-0 space-y-4" data-testid="zpaq-diff">
      <p class="break-all text-xs text-muted">{{ path }}</p>
      <div class="grid min-w-0 grid-cols-1 gap-3 sm:grid-cols-2">
        <label class="min-w-0 text-xs font-semibold">较早版本<select v-model.number="from" class="diff-input mt-1 w-full" :disabled="busy" data-testid="zpaq-diff-before" @change="clear"><option v-for="s in history.snapshots" :key="s.version" :value="s.version">{{ date(s) }}</option></select></label>
        <label class="min-w-0 text-xs font-semibold">较新版本<select v-model.number="to" class="diff-input mt-1 w-full" :disabled="busy" data-testid="zpaq-diff-after" @change="clear"><option v-for="s in history.snapshots" :key="s.version" :value="s.version">{{ date(s) }}</option></select></label>
      </div>
      <div class="flex min-w-0 flex-wrap items-center gap-2"><button class="diff-button bg-primary text-white" type="button" data-testid="zpaq-diff-compare" :disabled="busy" @click="compare()">比较目录</button><button v-if="request" class="diff-button" type="button" data-testid="zpaq-diff-stop" @click="stop">停止比较</button><span v-if="request" class="min-w-0 text-xs text-primary">{{ phaseText }}<template v-if="progress?.phase.endsWith('hash')"> · {{ formatFileSize(progress.processedBytes) }} / {{ formatFileSize(progress.totalBytes) }}</template></span></div>
      <span class="context-help"><HelpHint label="快照核对说明">先比较目录和内容片段引用，引用变化标为“内容候选”，不当作逐字节结论。精确核对会依次临时提取两版所选候选，计算 SHA256，支持停止并清理临时文件。原包不改动。</HelpHint></span>
      <p v-if="error" role="alert" class="break-words rounded-lg bg-red-500/5 p-2 text-xs text-red-500">{{ error }}</p>
      <p v-if="notice" role="status" class="break-words text-xs text-primary" data-testid="zpaq-diff-notice">{{ notice }}</p>
      <template v-if="result">
        <div class="flex flex-wrap gap-2" data-testid="zpaq-diff-counts"><button v-for="(label,kind) in diffLabels" :key="kind" class="diff-chip" :class="{'border-primary text-primary':filter===kind}" :disabled="busy" type="button" @click="filter=kind">{{ label }} {{ result.counts[kind]||0 }}</button></div>
        <div class="flex min-w-0 flex-wrap gap-2"><input v-model="query" class="diff-input min-w-0 flex-1" placeholder="搜索归档内路径" data-testid="zpaq-diff-search" /><select v-model="filter" class="diff-input" data-testid="zpaq-diff-filter"><option value="differences">仅差异</option><option value="all">全部条目</option><option v-for="(label,kind) in diffLabels" :key="kind" :value="kind">{{ label }}</option></select></div>
        <div class="flex flex-wrap items-center gap-2 text-xs"><span class="mr-auto text-muted">已选 {{ selected.size }} 项版本内容 · {{ rows.length }} 条结果</span><button class="text-primary" type="button" :disabled="busy" @click="selected=defaultDiffSelection({...result,entries:pageRows})">选择本页变化</button><button class="text-muted" type="button" :disabled="busy" @click="selected=new Set()">清空选择</button></div>
        <div v-if="pageCount>1" class="flex flex-wrap items-center gap-2 text-xs"><button class="diff-button" :disabled="busy||page<=1" @click="page--">上一页</button><span>第 {{ page }} / {{ pageCount }} 页 · 每页200条</span><button class="diff-button" :disabled="busy||page>=pageCount" @click="page++">下一页</button></div>
        <div class="min-w-0 divide-y divide-subtle overflow-hidden rounded-xl border border-subtle" data-testid="zpaq-diff-entries">
          <article v-for="row in pageRows" :key="row.path" class="grid min-w-0 grid-cols-1 gap-3 p-3 sm:grid-cols-[minmax(0,1fr)_minmax(8rem,.5fr)_minmax(8rem,.5fr)]" :data-diff-path="row.path" :data-diff-kind="row.kind">
            <div class="min-w-0 text-xs"><p class="break-all font-semibold text-content"><i :class="row.after?.directory||row.before?.directory?'pi pi-folder':'pi pi-file'"></i> {{ row.path }}</p><p class="mt-1 font-semibold" :class="row.kind==='removed'?'text-amber-600':'text-primary'">{{ diffLabels[row.kind] }} <span v-if="row.verified">· 已核对</span></p><p class="mt-1 break-words leading-relaxed text-muted">{{ row.reason }}</p></div>
            <label v-for="side in (['after','before'] as const)" :key="side" class="min-w-0 rounded-lg bg-input/40 p-2 text-xs"><span class="flex items-center gap-2"><input type="checkbox" :checked="selected.has(selectionKey(side,row.path))" :disabled="busy||!row[side]?.selectable" :data-diff-side="side" :aria-label="`${side==='after'?'新版':'旧版'} ${row.path}`" @change="toggle(side,row.path)" /><strong>{{ side==='after'?'新版':'旧版' }} · 第 {{ result[side].version }} 版</strong></span><span v-if="row[side]" class="mt-1 block text-muted">{{ row[side]!.directory ? row[side]!.selectable?'空目录':'目录（请选择具体文件）' : formatFileSize(row[side]!.size) }}</span><span v-if="row[side]" class="mt-1 block break-words text-[10px] text-muted">{{ snapshotDate(row[side]!.date) }}</span><span v-else class="mt-1 block text-muted">此版本不存在</span></label>
          </article>
          <p v-if="!rows.length" class="p-5 text-center text-xs text-muted">没有符合筛选的条目。</p>
        </div>
        <button type="button" class="diff-button" data-testid="zpaq-diff-verify" :disabled="busy||!candidates.length" @click="compare(true)">精确核对所选候选（{{ candidates.length }}）</button>
        <div class="rounded-xl border border-subtle p-3 text-xs"><p class="font-semibold">按来源版本分别创建待处理任务</p><p class="mt-1 leading-relaxed text-muted">新版保存新增/变化内容，旧版可取回移除内容；两版使用独立子目录，保留路径和冲突处理。非空目录请勾选具体文件，避免带出未选内容。</p><div class="mt-2 flex min-w-0 flex-wrap items-center gap-2"><input v-model="destination" :disabled="busy" class="diff-input min-w-0 flex-1" data-testid="zpaq-diff-output" placeholder="选择已有输出目录" /><button class="diff-button" type="button" :disabled="busy" @click="chooseOutput">选择目录</button></div><p v-for="g in groups" :key="g.side" class="mt-2 break-words text-muted">第 {{ g.snapshot.version }} 版 · {{ g.paths.length }} 项 → 独立版本子目录</p></div>
      </template>
    </div>
    <template #footer><button class="diff-button" type="button" @click="visible=false">关闭</button><button v-if="created" class="diff-button bg-primary text-white" type="button" data-testid="zpaq-diff-review" @click="review">查看待处理任务</button><button v-else class="diff-button bg-primary text-white" type="button" data-testid="zpaq-diff-create" :disabled="busy||!groups.length||!destination.trim()" @click="createDrafts">{{ creating?'正在创建…':`创建待处理任务（${groups.length}）` }}</button></template>
  </Modal>
</template>
<style scoped>
.diff-input{border:1px solid var(--border-subtle);border-radius:.65rem;padding:.6rem .7rem;background:var(--bg-input);font-size:.75rem;color:var(--text-content);max-width:100%}
.diff-button,.diff-chip{display:inline-flex;align-items:center;justify-content:center;border:1px solid var(--border-subtle);border-radius:.65rem;padding:.5rem .75rem;font-size:.75rem;font-weight:600;transition:background .15s}
.diff-chip{border-radius:999px;padding:.35rem .65rem;background:var(--bg-input)}
.diff-button:disabled,.diff-chip:disabled{opacity:.4;cursor:default}.diff-button:focus-visible,.diff-input:focus-visible{outline:2px solid var(--dynamic-accent);outline-offset:2px}
</style>
