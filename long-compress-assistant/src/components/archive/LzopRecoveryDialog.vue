<script setup lang="ts">
import {computed,onMounted,onUnmounted,ref} from 'vue'
import {invoke} from '@tauri-apps/api/tauri'
import {listen} from '@tauri-apps/api/event'
import {open} from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import ResultActions from '@/components/tasks/ResultActions.vue'
import {parentDirectory} from '@/utils/resultLocation'
import {archiveInspectionMessageForDisplay} from '@/utils/taskFailure'
import type {ZipPasswordEncoding} from '@/types/zipPassword'
import type {TaskHistoryRecord} from '@/types/taskHistory'
import type {LzopRecoveryReport,LzopRecoveryOutcome} from '@/types/lzopRecovery'
const props=defineProps<{path:string;encoding?:ZipPasswordEncoding;initial?:LzopRecoveryReport;originTaskId?:string}>()
const emit=defineEmits<{close:[]}>()
const report=ref<LzopRecoveryReport|undefined>(props.initial),result=ref<LzopRecoveryOutcome>(),selected=ref(new Set<number>(props.initial?.entries.map(e=>e.id)||[]))
const query=ref(''),page=ref(0),output=ref(parentDirectory(props.path)),busy=ref(false),cancelling=ref(false),error=ref(''),operation=ref(''),phase=ref('checking'),bytes=ref(0),total=ref(0)
const history=ref<TaskHistoryRecord>(),historyError=ref('')
let unlisten:(()=>void)|undefined;let disposed=false
const filtered=computed(()=>report.value?.entries.filter(e=>e.name.toLowerCase().includes(query.value.trim().toLowerCase()))||[])
const pages=computed(()=>Math.max(1,Math.ceil(filtered.value.length/50)))
const visibleEntries=computed(()=>filtered.value.slice(Math.min(page.value,pages.value-1)*50,(Math.min(page.value,pages.value-1)+1)*50))
const filename=computed(()=>props.path.split(/[\\/]/).pop())
const readable=(s:string)=>archiveInspectionMessageForDisplay(s)
function toggle(id:number){const next=new Set(selected.value);next.has(id)?next.delete(id):next.add(id);selected.value=next}
function toggleAll(){const next=new Set(selected.value);const all=filtered.value.every(e=>next.has(e.id));for(const e of filtered.value)all?next.delete(e.id):next.add(e.id);selected.value=next}
function begin(){operation.value=`lzop-recovery-${crypto.randomUUID()}`;busy.value=true;cancelling.value=false;error.value='';bytes.value=total.value=0;phase.value='checking'}
async function scan(){begin();try{report.value=await invoke<LzopRecoveryReport>('scan_lzop_recovery',{operationId:operation.value,filePath:props.path,zipPasswordEncoding:props.encoding||null});selected.value=new Set(report.value.entries.map(e=>e.id));page.value=0}catch(e){if(!/cancel|已取消/i.test(String(e)))error.value=readable(String(e))}finally{busy.value=false;operation.value=''}}
async function chooseOutput(){const queued=import.meta.env.VITE_DESKTOP_E2E==='1'?window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection():undefined;const dir=queued===undefined?await open({directory:true,multiple:false,title:'选择部分取回保存目录'}):queued;if(typeof dir==='string')output.value=dir}
async function saveHistory(){if(!history.value)return;try{await invoke('save_task_history',{record:history.value});historyError.value=''}catch{historyError.value='历史记录未能保存，取回结果仍保留。'}}
async function recover(){if(!report.value||!selected.value.size||!output.value.trim()||busy.value)return;const checked=report.value;begin();const started=new Date()
 let status:TaskHistoryRecord['status']='completed',failure=''
 try{result.value=await invoke<LzopRecoveryOutcome>('recover_lzop_members',{operationId:operation.value,filePath:props.path,outputDirectory:output.value.trim(),revision:checked.revision,selected:[...selected.value],zipPasswordEncoding:props.encoding||null})}
 catch(e){failure=readable(String(e));status=/cancel|已取消/i.test(failure)?'cancelled':'failed';error.value=status==='cancelled'?'已停止取回，本次暂存已清理。':failure}
 finally{const end=new Date();history.value={id:operation.value,name:`LZOP 部分取回 · ${filename.value}`,taskType:'decompression',workloadKind:'archive',status,sourcePaths:[props.path],outputPath:result.value?.outputDirectory||output.value.trim(),format:'LZOP',retryOfTaskId:props.originTaskId||null,startedAt:started.toISOString(),completedAt:end.toISOString(),durationMs:end.getTime()-started.getTime(),processedBytes:result.value?.recoveredBytes||0,totalBytes:result.value?.recoveredBytes||0,errorMessage:failure||null,executionConfig:{schemaVersion:1,kind:'archive-recovery',outputDirectory:result.value?.outputDirectory||output.value.trim(),recoveredFiles:result.value?.recoveredFiles||0,stopReason:checked.stopReason,revision:checked.revision},logs:[{timestamp:end.toISOString(),severity:status==='completed'?'warning':'info',message:result.value?`部分取回 ${result.value.recoveredFiles} 个完整文件，后续内容未确认。${checked.stopReason}`:failure}]};await saveHistory();busy.value=false;operation.value=''}
}
async function cancel(){if(!operation.value||cancelling.value)return;cancelling.value=true;try{await invoke('cancel_zip_repair',{repairId:operation.value})}catch{} }
onMounted(async()=>{const stop=await listen<{operationId:string;phase:string;bytes:number;total:number}>('lzop-recovery-progress',e=>{if(e.payload.operationId===operation.value){phase.value=e.payload.phase;bytes.value=e.payload.bytes;total.value=e.payload.total}});if(disposed){stop();return}unlisten=stop;if(!report.value)await scan()})
onUnmounted(()=>{disposed=true;unlisten?.();if(operation.value)void cancel()})
</script>
<template>
 <Modal :visible="true" title="LZOP 部分取回" icon="pi pi-folder-open" size="lg" :show-close-button="!busy" :close-on-backdrop="!busy" :close-on-escape="!busy" @close="emit('close')">
  <div class="lzop-recovery" data-testid="lzop-recovery-dialog">
   <div class="source-line"><i class="pi pi-box"/><strong :title="path">{{filename}}</strong><HelpHint label="LZOP 部分取回范围">只取回从首卷起连续存在、完整结束且通过内容校验的成员。第一个损坏、截断或无校验成员处停止，后续清单未知；格式没有总成员数或总卷数。结果单独保存，原件保留。</HelpHint></div>
   <template v-if="result">
    <div class="partial-result" data-testid="lzop-recovery-result"><strong>已部分取回 {{result.recoveredFiles}} 个文件</strong><span>后续内容未确认</span></div>
    <ResultActions :path="result.outputDirectory" directory show-path />
    <details><summary>查看已取回文件</summary><ul class="recovery-list"><li v-for="entry in result.entries" :key="entry.id">{{entry.name}}</li></ul></details>
   </template>
   <template v-else-if="report">
    <div class="scope-line"><strong data-testid="lzop-recovery-count">{{report.entries.length}} 个完整文件可取回</strong><button class="recovery-button" :disabled="busy" data-testid="lzop-recovery-recheck" @click="scan">重新检查</button></div>
    <details class="stop-detail"><summary>查看停止原因<small v-if="report.ignoredVolumes"> · 后续 {{report.ignoredVolumes}} 卷未拼接</small></summary><p>{{readable(report.stopReason)}}</p><p v-if="report.stoppedMember">停止于：{{report.stoppedMember}}</p><p>位置：{{report.stoppedOffset}} 字节；后续清单未知。</p></details>
    <div v-if="report.entries.length&&!report.complete" class="members" data-testid="lzop-recovery-members">
     <div class="members-toolbar"><input v-model="query" class="recovery-input" placeholder="筛选文件名" aria-label="筛选可取回文件" @input="page=0"/><button class="recovery-button" :disabled="busy" @click="toggleAll">{{filtered.length&&filtered.every(e=>selected.has(e.id))?'取消所列':'选择所列'}}</button></div>
     <div class="member-list custom-scrollbar"><label v-for="entry in visibleEntries" :key="entry.id" class="member" :data-recovery-member="entry.id"><input type="checkbox" :checked="selected.has(entry.id)" :disabled="busy" @change="toggle(entry.id)"/><span :title="entry.name">{{entry.name}}</span><small>{{entry.bytes.toLocaleString()}} B</small></label></div>
     <div class="members-footer"><span>已选 {{selected.size}} 项</span><template v-if="pages>1"><button class="recovery-button" :disabled="page<=0||busy" @click="page--">上一页</button><span>{{page+1}} / {{pages}}</span><button class="recovery-button" :disabled="page>=pages-1||busy" @click="page++">下一页</button></template></div>
    </div>
    <p v-else class="empty-message">{{report.complete?'现有内容完整，请返回正常解压。':'没有可确认完好的完整成员，请补齐原包后重试。'}}</p>
    <label v-if="report.entries.length&&!report.complete" class="destination">保存到<HelpHint label="部分取回保存位置">在此目录中新建独立的“部分取回”文件夹，所选文件直接保存其中。同名旧文件不会被覆盖。</HelpHint><div><input v-model="output" :disabled="busy" class="recovery-input" data-testid="lzop-recovery-output"/><button class="recovery-button" :disabled="busy" aria-label="选择部分取回目录" @click="chooseOutput"><i class="pi pi-folder-open"/></button></div></label>
   </template>
   <div v-if="busy" class="recovery-progress" data-testid="lzop-recovery-progress"><span>{{phase==='checking'?'正在核对完整成员…':'正在取回所选文件…'}}</span><progress :value="total?bytes:undefined" :max="total||1"/><small v-if="total">{{Math.min(100,Math.round(bytes/total*100))}}%</small></div>
   <p v-if="error" role="alert" data-testid="lzop-recovery-error" class="text-red-500">{{error}}</p>
   <p v-if="historyError" class="text-amber-600">{{historyError}} <button class="text-primary" data-testid="lzop-recovery-save-history" @click="saveHistory">重试保存记录</button></p>
  </div>
  <template #footer><button class="recovery-button" :disabled="busy" @click="emit('close')">关闭</button><button v-if="busy" class="recovery-button" :disabled="cancelling" data-testid="lzop-recovery-cancel" @click="cancel">{{cancelling?'正在停止…':'停止'}}</button><button v-else-if="!result" class="recovery-button primary" :disabled="!report||report.complete||!selected.size||!output.trim()" data-testid="lzop-recovery-confirm" @click="recover">取回所选{{selected.size?`（${selected.size}）`:''}}</button></template>
 </Modal>
</template>
<style scoped>
.lzop-recovery{display:grid;gap:.85rem;min-width:0;font-size:.75rem}.source-line,.scope-line,.members-toolbar,.members-footer{display:flex;align-items:center;gap:.6rem;min-width:0}.source-line>strong{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.source-line>i{color:var(--dynamic-accent)}.scope-line{justify-content:space-between}.recovery-button{flex-shrink:0;border:1px solid var(--border-subtle);border-radius:.65rem;background:var(--bg-input);color:var(--text-content);font-size:.75rem;font-weight:700;padding:.55rem .75rem}.recovery-button.primary{background:var(--dynamic-accent);border-color:transparent;color:white}.recovery-button:disabled{opacity:.4;cursor:default}.recovery-input{width:100%;min-width:0;padding:.6rem .7rem;border:1px solid var(--border-subtle);border-radius:.65rem;background:var(--bg-input);color:var(--text-content)}.recovery-input:focus-visible,button:focus-visible{outline:2px solid var(--dynamic-accent);outline-offset:2px}.stop-detail{border:1px solid var(--border-subtle);border-radius:.7rem;padding:.65rem .8rem;color:var(--text-muted)}summary{cursor:pointer;color:var(--dynamic-accent);font-weight:700}details p{margin-top:.5rem;overflow-wrap:anywhere}details small{font-size:inherit}.members{border:1px solid var(--border-subtle);border-radius:.8rem;overflow:hidden;min-width:0}.members-toolbar,.members-footer{padding:.55rem .65rem;background:var(--bg-input)}.members-footer>span:first-child{margin-right:auto}.member-list{max-height:14rem;overflow:auto}.member{display:flex;align-items:center;gap:.65rem;padding:.65rem .8rem;border-top:1px solid var(--border-subtle);cursor:pointer;min-width:0}.member>span{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1;color:var(--text-content)}.member small{flex-shrink:0;color:var(--text-muted)}.destination{display:flex;flex-wrap:wrap;align-items:center;gap:.5rem;color:var(--text-muted)}.destination>div{display:flex;gap:.5rem;flex-basis:100%;min-width:0}.recovery-progress{display:flex;align-items:center;gap:.5rem;flex-wrap:wrap;color:var(--dynamic-accent)}progress{width:100%;accent-color:var(--dynamic-accent)}.partial-result{display:flex;flex-wrap:wrap;gap:.5rem;padding:.8rem;border:1px solid #d9770640;border-radius:.8rem;background:#d977060b;color:#b45309}.partial-result>span{margin-left:auto}.recovery-list{max-height:14rem;overflow:auto;line-height:1.8;padding:.6rem;overflow-wrap:anywhere}.empty-message{color:var(--text-muted)}
</style>
