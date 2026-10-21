import { defineStore } from 'pinia'
import { computed,ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import type {ZipPasswordEncoding} from '@/types/zipPassword'
import { listen,type UnlistenFn } from '@tauri-apps/api/event'
import { useTaskStore } from './task'
import { compareEntries,copyComparisonSelection,type ComparisonSide,type ComparisonSource,type ComparisonSelection,type ComparisonKind,hasPair,type ContentVerdict,type ContentProgress } from '@/types/sourceComparison'
interface SideState {path:string;source?:ComparisonSource;jobId:string;error:string;readCount:number;password:string;encoding?:ZipPasswordEncoding;retained?:string[]}
const initial=():SideState=>({path:'',jobId:'',error:'',readCount:0,password:''})
export const useSourceComparisonStore=defineStore('sourceComparison',()=>{
 const packageSource=ref('')
 const tasks=useTaskStore(),sides=ref<Record<ComparisonSide,SideState>>({left:initial(),right:initial()}),selected=ref<Record<ComparisonSide,Set<string>>>({left:new Set(),right:new Set()}),query=ref(''),filter=ref('all'),page=ref(1),expanded=ref(''),busy=ref(false),error=ref(''),notice=ref(''),issues=ref<Partial<Record<ComparisonSide,string>>>({}),admitted=ref(0)
 const verdicts=ref<Record<string,ContentVerdict>>({}),verifyJob=ref(''),stopping=ref(false),verifyNote=ref(''),verifyProgress=ref<ContentProgress>({processedBytes:0,totalBytes:0,completed:0,total:0,path:'',side:''})
 const options=ref({outputDirectory:'',keepStructure:true,separateSides:true,conflictPolicy:'rename' as 'rename'|'skip'})
 const ready=computed(()=>!!sides.value.left.source&&!!sides.value.right.source&&!sides.value.left.jobId&&!sides.value.right.jobId)
 const rows=computed(()=>ready.value?compareEntries(sides.value.left.source!.entries,sides.value.right.source!.entries).map(row=>verdicts.value[row.path]?{...row,kind:verdicts.value[row.path]!.kind,reason:verdicts.value[row.path]!.reason}:row):[])
 const counts=computed(()=>rows.value.reduce((all,row)=>({...all,[row.kind]:(all[row.kind]||0)+1}),{} as Partial<Record<ComparisonKind,number>>))
 const filtered=computed(()=>rows.value.filter(row=>(filter.value==='all'||filter.value==='unverified'&&hasPair(row)&&(!verdicts.value[row.path]||row.kind==='unreadable')||filter.value==='changes'&&!!row.left&&!!row.right&&!['candidate','directory','same','unreadable'].includes(row.kind)||row.kind===filter.value)&&row.path.toLowerCase().includes(query.value.trim().toLowerCase())))
 const unverifiedCount=computed(()=>rows.value.filter(row=>hasPair(row)&&(!verdicts.value[row.path]||row.kind==='unreadable')).length)
 const changedCount=computed(()=>rows.value.filter(row=>row.left&&row.right&&['type','size','time','different','metadata'].includes(row.kind)).length)
 const pages=computed(()=>Math.max(1,Math.ceil(filtered.value.length/50))),visible=computed(()=>filtered.value.slice((page.value-1)*50,page.value*50))
 const selectedCount=computed(()=>selected.value.left.size+selected.value.right.size)
 let unlisteners:UnlistenFn[]=[],epoch=0
 async function attach(){
  const current=++epoch
  const stops=await Promise.all([
   listen<{jobId:string;progress:{entries:number}}>('comparison-read-progress',event=>{for(const side of ['left','right'] as ComparisonSide[])if(sides.value[side].jobId===event.payload.jobId)sides.value[side].readCount=event.payload.progress.entries}),
   listen<{jobId:string;progress:ContentProgress}>('comparison-content-progress',event=>{if(verifyJob.value===event.payload.jobId)verifyProgress.value=event.payload.progress}),
   listen<{jobId:string;entry:ContentVerdict}>('comparison-content-result',event=>{if(verifyJob.value===event.payload.jobId)verdicts.value[event.payload.entry.path]=event.payload.entry}),
  ])
  if(current!==epoch)stops.forEach(stop=>stop());else unlisteners=stops
 }
 async function stopVerification(){if(!verifyJob.value)return;stopping.value=true;await invoke('library_cancel',{jobId:verifyJob.value})}
 async function verify(paths?:string[],force=false){
  if(!ready.value||busy.value||verifyJob.value)return
  const scope=rows.value.filter(row=>hasPair(row)&&(!paths||paths.includes(row.path))&&(force||!verdicts.value[row.path]||row.kind==='unreadable')).map(row=>row.path)
  if(!scope.length){verifyNote.value='当前范围没有待核对的双方文件；已确认项可在单条展开中重新核对。';return}
  if(scope.length>2000||JSON.stringify(scope).length>60000){error.value='每次最多核对 2000 项，请缩小筛选或所选范围';return}
  const jobId=crypto.randomUUID();verifyJob.value=jobId;stopping.value=false;error.value='';verifyNote.value='正在读取双方实际内容…';verifyProgress.value={processedBytes:0,totalBytes:0,completed:0,total:scope.length,path:'',side:''}
  const left=sides.value.left.source!.source,right=sides.value.right.source!.source
  try{
   const batch=await invoke<{entries:ContentVerdict[];stopped:boolean}>('comparison_verify',{jobId,request:{left,right,paths:scope,leftCredentials:{password:sides.value.left.password,encoding:sides.value.left.encoding},rightCredentials:{password:sides.value.right.password,encoding:sides.value.right.encoding}}})
   if(verifyJob.value!==jobId)return
   for(const entry of batch.entries)verdicts.value[entry.path]=entry
   const failed=batch.entries.filter(entry=>entry.kind==='unreadable').length
   verifyNote.value=`${batch.stopped?'已停止，保留已完成项':'本次核对完成'} · ${batch.entries.length} / ${scope.length} 项`+(failed?`，${failed} 项需补密码或处理来源后继续`:'')
  }catch(cause){if(verifyJob.value===jobId){const reason=String(cause);if(reason.includes('已停止')&&!reason.includes('[comparison:changed]'))verifyNote.value='已停止，保留已完成项；可继续核对剩余项。';else{verdicts.value={};error.value=reason;verifyNote.value='本次来源未能完成确认，旧内容结论已撤销；请处理来源后重新读取。'}}}
  finally{if(verifyJob.value===jobId){verifyJob.value='';stopping.value=false}}
 }
 function verifySelected(){return verify([...new Set([...selected.value.left,...selected.value.right])])}
 function verifyFiltered(){return verify(filtered.value.map(row=>row.path))}
 async function cancel(side:ComparisonSide){const id=sides.value[side].jobId;if(!id)return;sides.value[side].jobId='';sides.value[side].error='读取已停止，请重新读取这一侧';await invoke('library_cancel',{jobId:id})}
 function detach(){epoch++;unlisteners.forEach(stop=>stop());unlisteners=[];void stopVerification();for(const side of ['left','right'] as ComparisonSide[])void cancel(side)}
 async function read(side:ComparisonSide,path:string){
  if(busy.value||verifyJob.value)return;await cancel(side);const old=sides.value[side],same=old.path.replace(/\\/g,'/').toLowerCase()===path.replace(/\\/g,'/').toLowerCase();const retained=same?[...new Set([...(old.retained||[]),...selected.value[side]])]:[];const id=crypto.randomUUID();sides.value[side]={path,jobId:id,error:'',readCount:0,password:same?old.password:'',encoding:same?old.encoding:undefined,retained};selected.value[side]=new Set();verdicts.value={};verifyNote.value='';expanded.value='';page.value=1;issues.value={};error.value=''
  try{const source=await invoke<ComparisonSource>('comparison_read',{path,jobId:id});if(sides.value[side].jobId===id){sides.value[side].source=source;sides.value[side].path=source.source.path;selected.value[side]=new Set(retained.filter(path=>source.entries.some(entry=>entry.path===path&&entry.selectable)));sides.value[side].retained=undefined}}
  catch(cause){if(sides.value[side].jobId===id)sides.value[side].error=String(cause)}finally{if(sides.value[side].jobId===id)sides.value[side].jobId=''}
 }
 function editPath(side:ComparisonSide){verdicts.value={};verifyNote.value='';sides.value[side].password='';sides.value[side].retained=undefined;sides.value[side].source=undefined;selected.value[side]=new Set();page.value=1;expanded.value=''}
 function toggle(side:ComparisonSide,path:string){if(busy.value||!!verifyJob.value||!ready.value||!sides.value[side].source?.entries.some(e=>e.path===path&&e.selectable))return;const next=new Set(selected.value[side]);if(next.has(path))next.delete(path);else{if(selectedCount.value>=2000){error.value='每批最多选择 2000 项，请先取回这一批';return}next.add(path)}selected.value[side]=next;notice.value=''}
 function clear(){if(busy.value)return;selected.value={left:new Set(),right:new Set()};issues.value={}}
 function togglePage(side:ComparisonSide){const entries=visible.value.map(row=>row[side]).filter(entry=>entry?.selectable);const all=entries.length>0&&entries.every(entry=>selected.value[side].has(entry!.path));for(const entry of entries)if(all||!selected.value[side].has(entry!.path))toggle(side,entry!.path)}
 function request(side:ComparisonSide):ComparisonSelection|null{const source=sides.value[side].source;if(!source||!selected.value[side].size)return null;return copyComparisonSelection({source:{...source.source,encrypted:source.entries.some(entry=>selected.value[side].has(entry.path)&&entry.encrypted)},zipPasswordEncoding:sides.value[side].encoding,paths:[...selected.value[side]].sort(),keepStructure:options.value.keepStructure,conflictPolicy:options.value.conflictPolicy})}
 async function queue(){
  if(busy.value||!!verifyJob.value||!ready.value||!selectedCount.value)return;if(!options.value.outputDirectory.trim()){error.value='请先选择取回位置';return}
  busy.value=true;error.value='';issues.value={};admitted.value=0;let amount=0
  try{
   await invoke('comparison_output',{sources:[sides.value.left.source!.source,sides.value.right.source!.source],output:options.value.outputDirectory})
   for(const side of ['left','right'] as ComparisonSide[]){if(!selected.value[side].size)continue
    try{const selection=request(side);if(!selection)throw Error('所选范围无效，请缩小范围后重新勾选');const source=sides.value[side].source!,name=(side==='left'?'左侧-':'右侧-')+source.name.replace(/[<>:"/\\|?*\x00-\x1f]/g,'_').replace(/[. ]+$/,'').slice(0,80)
     const output=options.value.separateSides?options.value.outputDirectory.replace(/[\\/]+$/,'')+'/'+name:options.value.outputDirectory
     await invoke('comparison_validate',{request:selection,output,password:sides.value[side].password||null})
     const duplicate=tasks.tasks.some(task=>task.status==='pending'&&task.outputPath===output&&JSON.stringify(task.comparisonSelection)===JSON.stringify(selection))
     if(!duplicate)tasks.addTask({id:crypto.randomUUID(),name:`${side==='left'?'左侧':'右侧'} · ${source.name} · 所选 ${selection.paths.length} 项`,type:'decompression',password:sides.value[side].password||undefined,zipPasswordEncoding:sides.value[side].encoding,sourceFiles:[source.source.path],outputPath:output,configurationMode:'individual',format:source.source.kind==='zip'?'ZIP':'文件夹',extractToSubfolder:false,recycleSourceAfterExtract:false,selectedEntries:[...selection.paths],comparisonSelection:selection,executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:output,extractToSubfolder:false,comparisonSelection:selection}})
     admitted.value++;amount+=selection.paths.length;selected.value[side]=new Set()
    }catch(cause){issues.value[side]=String(cause).replace(/^Error:\s*/,'')}
   }
   notice.value=`已将 ${amount} 项按 ${admitted.value} 份来源加入解压中心待处理队列`+(selectedCount.value?'；剩余项请处理来源后继续':'，核对后点击开始取回')
  }catch(cause){error.value=String(cause)}finally{busy.value=false}
 }
 return {packageSource,unverifiedCount,changedCount,verdicts,verifyJob,verifyProgress,verifyNote,stopping,verify,verifySelected,verifyFiltered,stopVerification,sides,selected,query,filter,page,expanded,busy,error,notice,issues,admitted,options,ready,rows,counts,filtered,pages,visible,selectedCount,attach,detach,cancel,read,toggle,clear,togglePage,request,queue,editPath}
})
