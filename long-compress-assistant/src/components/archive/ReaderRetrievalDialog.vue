<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import {computed,ref} from 'vue'
import {invoke} from '@tauri-apps/api/tauri'
import {open} from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import {useTaskStore} from '@/stores/task'
import {useArchiveReadingStore} from '@/stores/archiveReading'
import {copyReaderSelection,readingKey} from '@/types/archiveReading'
import type {ZipPasswordEncoding} from '@/types/zipPassword'
import type {ZpaqSnapshot} from '@/types/zpaqSnapshot'
const props=defineProps<{filePath:string;format:string;revision:string;paths:string[];outputPath:string;password:string;encoding?:ZipPasswordEncoding;snapshot?:ZpaqSnapshot;passwordRequired:boolean}>()
const emit=defineEmits<{close:[resume?:boolean];selection:[paths:string[]]}>()
const tasks=useTaskStore(),reading=useArchiveReadingStore(),paths=ref([...props.paths]),output=ref(props.outputPath),keepStructure=ref(true),conflictPolicy=ref<'rename'|'skip'>('rename'),busy=ref(false),error=ref(''),created=ref(false),page=ref(1)
const pages=computed(()=>Math.max(1,Math.ceil(paths.value.length/20))),visible=computed(()=>paths.value.slice((page.value-1)*20,page.value*20))
function rememberSelection(){emit('selection',[...paths.value]);const row=reading.find(props.filePath,props.snapshot);if(row?.revision===props.revision)reading.save({...row,selected:[...paths.value]})}
function remove(path:string){paths.value=paths.value.filter(p=>p!==path);page.value=Math.min(page.value,pages.value);rememberSelection()}
async function chooseOutput(){const queued=import.meta.env.VITE_DESKTOP_E2E==='1'?window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection():undefined;const value=queued===undefined?await open({directory:true,title:'选择所选图片的取回位置'}):queued;if(typeof value==='string')output.value=value}
async function queue(){
 const selection=copyReaderSelection({revision:props.revision,paths:paths.value,keepStructure:keepStructure.value,conflictPolicy:conflictPolicy.value})
 if(!selection||!output.value.trim()){error.value='请保留需要取回的图片，并选择输出目录。';return}
 busy.value=true;error.value=''
 try{
  await invoke('check_image_reader_source',{filePath:props.filePath,revision:props.revision})
  const duplicate=tasks.tasks.some(task=>task.status==='pending'&&readingKey(task.sourceFiles[0]||'',task.zpaqSnapshot)===readingKey(props.filePath,props.snapshot)&&task.outputPath===output.value.trim()&&JSON.stringify(task.readerSelection)===JSON.stringify(selection))
  if(!duplicate)tasks.addTask({id:crypto.randomUUID(),name:`${props.filePath.replace(/\\/g,'/').split('/').pop()} · 所选 ${selection.paths.length} 张图片`,type:'decompression',workloadKind:'archive',sourceFiles:[props.filePath],format:props.format,outputPath:output.value.trim(),configurationMode:'individual',extractToSubfolder:false,recycleSourceAfterExtract:false,selectedEntries:[...selection.paths],readerSelection:selection,password:props.password||undefined,zipPasswordEncoding:props.encoding,zpaqSnapshot:props.snapshot?{...props.snapshot}:undefined,draftPasswordRequired:props.passwordRequired&&!props.password,executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:output.value.trim(),extractToSubfolder:false,readerSelection:selection,zpaqSnapshot:props.snapshot,zipPasswordEncoding:props.encoding}})
  paths.value=[];rememberSelection();created.value=true
 }catch(cause){error.value=String(cause).replace(/^Error:\s*/,'').replace(/^.*IMAGE_READER_CHANGED\|/,'')}
 finally{busy.value=false}
}
</script>
<template>
 <Modal :visible="true" title="取回所选图片" icon="pi pi-images" size="xl" :show-footer="true" :show-close-button="!busy" :close-on-backdrop="!busy" :close-on-escape="!busy" @update:visible="!busy&&emit('close')">
  <div class="reader-retrieval" data-testid="reader-retrieval">
   <section class="retrieval-panel"><h3>只取回明确选中的图片</h3><p class="retrieval-hint">{{ paths.length }} 张 · {{ filePath.replace(/\\/g,'/').split('/').pop() }}</p>
    <div class="retrieval-paths custom-scrollbar"><div v-for="path in visible" :key="path" class="retrieval-path"><span>{{ path }}</span><button :disabled="busy" :aria-label="'移除 '+path" @click="remove(path)"><i class="pi pi-times"/></button></div><p v-if="created" class="retrieval-hint" data-testid="reader-retrieval-notice">已加入解压中心待处理队列，核对后点击开始；原包保留。</p><p v-else-if="!paths.length" class="retrieval-hint">所选已清空，可返回阅读继续挑选。</p></div>
    <div v-if="pages>1" class="retrieval-pagination"><button :disabled="page<=1" @click="page--">上一页</button><span>{{ page }} / {{ pages }}</span><button :disabled="page>=pages" @click="page++">下一页</button></div>
   </section>
   <fieldset class="retrieval-panel" :disabled="busy||created"><h3>取回设置</h3><label>输出目录<div class="retrieval-output"><input v-model="output" data-testid="reader-retrieval-output" placeholder="选择或输入保存位置"><button data-testid="reader-retrieval-choose-output" @click="chooseOutput">选择</button></div></label><label>包内目录<select v-model="keepStructure" data-testid="reader-retrieval-structure"><option :value="true">保留包内文件夹结构</option><option :value="false">只取文件，放在同一层</option></select></label><label>同名文件<select v-model="conflictPolicy" data-testid="reader-retrieval-conflict"><option value="rename">自动改名，保留两份</option><option value="skip">跳过已存在文件</option></select></label><span class="context-help"><HelpHint label="所选图片取回说明">进入原解压队列，进度、日志和结果沿用原入口。草稿与历史保留本次范围；密码不写入记录。</HelpHint></span></fieldset>
  </div>
  <p v-if="error" class="retrieval-error" role="alert" data-testid="reader-retrieval-error">{{ error }}</p>
  <template #footer><button class="retrieval-button" :disabled="busy" @click="emit('close')">返回阅读</button><RouterLink v-if="created" to="/decompress" class="retrieval-button primary" data-testid="reader-retrieval-go-queue" @click="emit('close',false)">前往解压中心</RouterLink><button v-else class="retrieval-button primary" data-testid="reader-retrieval-confirm" :disabled="busy||!paths.length" @click="queue">{{ busy?'正在加入…':'加入待处理队列' }}</button></template>
 </Modal>
</template>
<style scoped>
.reader-retrieval{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:18px}.retrieval-panel{min-width:0;border:1px solid var(--border-subtle);border-radius:16px;padding:18px;background:var(--bg-card)}h3{font-size:14px;font-weight:850}.retrieval-hint{font-size:11px;color:var(--text-muted);line-height:1.9;margin-top:10px;overflow-wrap:anywhere}.retrieval-panel label{display:block;margin-top:18px;font-size:12px;font-weight:700}.retrieval-output{display:flex;gap:8px;min-width:0;margin-top:8px}input,select,.retrieval-output button,.retrieval-button{border:1px solid var(--border-subtle);border-radius:10px;padding:10px 12px;background:var(--bg-input);color:var(--text-base);font-size:12px;min-width:0}input{flex:1;width:100%}select{width:100%;margin-top:8px}.retrieval-output button{flex-shrink:0}.retrieval-paths{max-height:250px;overflow:auto;margin-top:14px}.retrieval-path{display:flex;gap:10px;padding:9px 0;border-bottom:1px solid var(--border-subtle);font-size:11px;line-height:1.8;overflow-wrap:anywhere}.retrieval-path span{flex:1;min-width:0}.retrieval-pagination{display:flex;align-items:center;justify-content:center;gap:14px;margin-top:12px;font-size:11px;color:var(--text-muted)}.retrieval-error{margin-top:14px;color:#d97706;font-size:12px;line-height:1.8;overflow-wrap:anywhere}.retrieval-button{display:inline-flex;align-items:center;justify-content:center}.primary{background:var(--dynamic-accent);color:#fff;border-color:transparent}button:disabled{opacity:.45}fieldset:disabled{opacity:.65}@media(max-width:800px){.reader-retrieval{grid-template-columns:minmax(0,1fr);gap:12px}.retrieval-panel{padding:14px}.retrieval-paths{max-height:150px}.retrieval-panel label{margin-top:12px}}
</style>
