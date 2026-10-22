<script setup lang="ts">
import {computed,onMounted,ref} from 'vue'
import {open} from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import {useTaskStore} from '@/stores/task'
import {archiveSourceDirectory} from '@/utils/zpaqMultipart'
import {copyZpaqRetrieval} from '@/types/zpaqRetrieval'
import {snapshotDate} from '@/types/zpaqSnapshot'
import type {ZpaqBlockReport} from '@/types/zpaqBlocks'
const props=defineProps<{path:string;password:string;encrypted:boolean;report:ZpaqBlockReport;paths:string[]}>()
const emit=defineEmits<{close:[]}>(),tasks=useTaskStore()
const output=ref(''),error=ref(''),created=ref(false),busy=ref(false)
const omitted=computed(()=>({affected:props.report.entries.filter(e=>e.state==='affected').length,unchecked:props.report.entries.filter(e=>e.state==='unchecked').length,notSelected:props.report.entries.filter(e=>e.state==='verified'&&!props.paths.includes(e.path)).length}))
onMounted(async()=>{try{output.value=await archiveSourceDirectory(props.path)}catch(e){error.value=String(e)}})
async function choose(){const queued=import.meta.env.VITE_DESKTOP_E2E==='1'?window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection():undefined;const value=queued===undefined?await open({directory:true,title:'选择部分取回位置'}):queued;if(typeof value==='string')output.value=value}
function queue(){
 if(busy.value||created.value||!output.value.trim())return
 const selection=copyZpaqRetrieval({snapshot:props.report.snapshot,revision:props.report.revision,paths:props.paths})
 if(!selection){error.value='请选择 1～2000 个已检查通过的文件，长路径清单可分批取回';return}
 if(selection.paths.some(p=>!props.report.entries.some(e=>e.path===p&&e.state==='verified'))){error.value='所选内容尚未检查通过，请重新诊断';return}
 const parent=output.value.trim(),id=crypto.randomUUID()
 tasks.addTask({id,name:`${props.path.replace(/\\/g,'/').split('/').pop()||'ZPAQ'} · 部分取回 ${selection.paths.length} 个文件`,type:'decompression',workloadKind:'archive',sourceFiles:[props.path],format:'ZPAQ',outputPath:parent,configurationMode:'individual',extractToSubfolder:false,recycleSourceAfterExtract:false,zpaqSnapshot:{...selection.snapshot},selectedEntries:[...selection.paths],zpaqRetrieval:selection,password:props.password||undefined,draftPasswordRequired:props.encrypted&&!props.password,executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:parent,extractToSubfolder:false,zpaqSnapshot:{...selection.snapshot},zpaqRetrieval:selection}})
 created.value=true
}
</script>
<template>
 <Modal :visible="true" title="取回已校验文件" icon="pi pi-download" size="md" @close="emit('close')">
  <div class="grid min-w-0 gap-4 text-sm" data-testid="zpaq-retrieval-dialog">
   <div class="rounded-xl border border-subtle bg-input p-3 flex flex-wrap gap-x-4 gap-y-1"><strong class="text-content">快照 {{report.snapshot.version}} · 所选 {{paths.length}} 个文件</strong><span class="text-xs text-muted">{{snapshotDate(report.snapshot.date)}}</span></div>
   <div class="flex flex-wrap items-center gap-2 text-xs text-muted"><strong>本次不取回</strong><span>{{omitted.affected}} 受影响 · {{omitted.unchecked}} 未检查 · {{omitted.notSelected}} 未选择</span><HelpHint label="部分取回范围">只取回你勾选的已校验文件。执行时重新核对来源与检查结果，完整校验后再发布；其他文件不会混入。原包保留，每次创建独立结果目录和取回说明。</HelpHint></div>
   <details class="rounded-xl border border-subtle p-3 text-xs"><summary class="cursor-pointer font-bold text-content">查看所选清单</summary><ul class="mt-2 max-h-40 overflow-auto space-y-1 text-muted"><li v-for="path in paths" :key="path" class="break-all">{{path}}</li></ul></details>
   <label class="grid gap-2 text-xs font-bold text-muted">取回位置<div class="flex min-w-0 gap-2"><input v-model="output" :disabled="created" data-testid="zpaq-retrieval-output" class="min-w-0 flex-1 rounded-xl border border-subtle bg-input p-3 text-content"/><button :disabled="created" class="rounded-xl border border-subtle px-3" aria-label="选择取回位置" @click="choose"><i class="pi pi-folder-open"/></button></div><span class="font-normal">在此创建独立结果目录，保留原相对路径</span></label>
   <p v-if="error" role="alert" class="text-xs text-red-500 break-all">{{error}}</p>
   <p v-if="created" role="status" class="text-xs text-primary" data-testid="zpaq-retrieval-created">已加入待处理队列，可核对后开始取回。</p>
  </div>
  <template #footer><button class="retrieval-button" @click="emit('close')">关闭</button><RouterLink v-if="created" class="retrieval-button primary" to="/decompress" data-testid="zpaq-retrieval-go-queue" @click="emit('close')">前往解压中心</RouterLink><button v-else class="retrieval-button primary" :disabled="!output.trim()||busy" data-testid="zpaq-retrieval-confirm" @click="queue">加入待处理队列</button></template>
 </Modal>
</template>
<style scoped>
.retrieval-button{padding:.65rem .9rem;border:1px solid var(--border-subtle);border-radius:.7rem;background:var(--bg-input);color:var(--text-content);font-size:.8rem;font-weight:700}.retrieval-button.primary{background:var(--dynamic-accent);border-color:transparent;color:white}.retrieval-button:disabled{opacity:.4}
</style>
