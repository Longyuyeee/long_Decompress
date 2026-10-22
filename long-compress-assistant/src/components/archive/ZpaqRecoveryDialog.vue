<script setup lang="ts">
import {computed,ref,watch} from 'vue'
import {open} from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import {useTauriCommands,type ArchiveBrowseResult} from '@/composables/useTauriCommands'
import {useTaskStore} from '@/stores/task'
import {snapshotDate,type ZpaqHistory} from '@/types/zpaqSnapshot'

const props=defineProps<{path:string;password:string;history:ZpaqHistory;outputPath:string}>()
const emit=defineEmits<{close:[]}>()
const commands=useTauriCommands(),tasks=useTaskStore()
const version=ref(props.history.selected.version),output=ref(props.outputPath),directory=ref<ArchiveBrowseResult>(),error=ref(''),busy=ref(false),created=ref(false)
const snapshot=computed(()=>props.history.snapshots.find(s=>s.version===version.value)!)
let sequence=0
watch(version,async()=>{
 const current=++sequence;directory.value=undefined;error.value=''
 try {const value=await commands.browseArchive(props.path,props.password,undefined,undefined,{version:snapshot.value.version,date:snapshot.value.date});if(current===sequence)directory.value=value}
 catch(e){if(current===sequence)error.value=String(e)}
},{immediate:true})
async function chooseOutput(){
 const queued=import.meta.env.VITE_DESKTOP_E2E==='1'?window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection():undefined
 const value=queued===undefined?await open({directory:true,title:'选择完整快照的取回位置'}):queued
 if(typeof value==='string')output.value=value
}
function queue(){
 if(!directory.value||!snapshot.value||!output.value.trim()||busy.value)return
 busy.value=true
 const selected={version:snapshot.value.version,date:snapshot.value.date}
 const name=props.path.replace(/\\/g,'/').split('/').pop()
 tasks.addTask({id:crypto.randomUUID(),name:`${name} · 取回快照 ${selected.version}（不含中断更新）`,type:'decompression',workloadKind:'archive',sourceFiles:[props.path],format:'ZPAQ',outputPath:output.value.trim(),configurationMode:'individual',extractToSubfolder:true,recycleSourceAfterExtract:false,password:props.password||undefined,zpaqSnapshot:selected,
  executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:output.value.trim(),extractToSubfolder:true,zpaqSnapshot:selected}})
 created.value=true;busy.value=false
}
</script>
<template>
 <Modal :visible="true" title="取回完整快照" icon="pi pi-history" size="md" :close-on-backdrop="!busy" @close="emit('close')">
  <div class="recovery-fields" data-testid="zpaq-recovery-dialog">
   <div class="recovery-notice"><i class="pi pi-exclamation-triangle"/><span>尾部更新未完成<HelpHint label="中断范围">只取回所选完整快照。中断事务的新增、修改和删除不包含在结果中，具体条目范围无法确定；原件保留，取回时仍会校验文件内容。</HelpHint></span></div>
   <label>取回版本<select v-model="version" :disabled="busy||created" data-testid="zpaq-recovery-version"><option v-for="item in [...history.snapshots].reverse()" :key="item.version" :value="item.version">第 {{item.version}} 次更新 · {{snapshotDate(item.date)}}</option></select></label>
   <div class="recovery-scope"><span>范围</span><strong>{{directory?`${directory.totalFiles} 个文件 · ${directory.totalDirectories} 个文件夹`:'正在读取…'}}</strong><small>所选快照全部内容 · 独立子目录</small></div>
   <label>取回位置<div class="recovery-output"><input v-model="output" :disabled="busy||created" placeholder="选择输出目录" data-testid="zpaq-recovery-output"/><button :disabled="busy||created" aria-label="选择取回位置" @click="chooseOutput"><i class="pi pi-folder-open"/></button></div></label>
   <p v-if="error" role="alert" class="text-red-400 break-words" data-testid="zpaq-recovery-error">{{error}}</p>
  </div>
  <template #footer><button class="recovery-button" :disabled="busy" @click="emit('close')">关闭</button><RouterLink v-if="created" class="recovery-button primary" to="/decompress" data-testid="zpaq-recovery-go-queue" @click="emit('close')">前往解压中心</RouterLink><button v-else class="recovery-button primary" :disabled="busy||!directory||!output.trim()" data-testid="zpaq-recovery-confirm" @click="queue">加入待处理队列</button></template>
 </Modal>
</template>
<style scoped>
.recovery-fields{display:grid;gap:1rem;min-width:0;font-size:.8rem}.recovery-notice{display:flex;align-items:center;gap:.65rem;padding:.8rem 1rem;background:color-mix(in srgb,#f59e0b 9%,var(--bg-card));border:1px solid color-mix(in srgb,#f59e0b 35%,var(--border-subtle));border-radius:.8rem;color:var(--text-content)}.recovery-notice>i{color:#f59e0b}.recovery-notice span{display:flex;align-items:center;gap:.5rem}.recovery-fields label{display:grid;gap:.5rem;color:var(--text-muted);font-weight:700}.recovery-fields select,.recovery-fields input{width:100%;min-width:0;background:var(--bg-input);color:var(--text-content);border:1px solid var(--border-subtle);border-radius:.7rem;padding:.7rem;outline:none}.recovery-fields select:focus,.recovery-fields input:focus{border-color:var(--dynamic-accent)}.recovery-scope{display:flex;flex-wrap:wrap;gap:.4rem .8rem;padding:.7rem;border-radius:.7rem;background:var(--bg-input)}.recovery-scope>span,.recovery-scope small{color:var(--text-muted)}.recovery-scope small{flex-basis:100%}.recovery-output{display:flex;gap:.5rem;min-width:0}.recovery-output button,.recovery-button{border:1px solid var(--border-subtle);border-radius:.7rem;padding:.65rem .9rem;background:var(--bg-input);color:var(--text-content);font-weight:700}.recovery-button.primary{background:var(--dynamic-accent);color:white;border-color:transparent}.recovery-button:disabled{opacity:.4}.recovery-output button{flex-shrink:0}
</style>
