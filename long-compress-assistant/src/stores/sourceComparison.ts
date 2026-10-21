import { defineStore } from 'pinia'
import { computed,ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { listen,type UnlistenFn } from '@tauri-apps/api/event'
import { useTaskStore } from './task'
import { compareEntries,copyComparisonSelection,type ComparisonSide,type ComparisonSource,type ComparisonSelection,type ComparisonKind } from '@/types/sourceComparison'
interface SideState {path:string;source?:ComparisonSource;jobId:string;error:string;readCount:number}
const initial=():SideState=>({path:'',jobId:'',error:'',readCount:0})
export const useSourceComparisonStore=defineStore('sourceComparison',()=>{
 const tasks=useTaskStore(),sides=ref<Record<ComparisonSide,SideState>>({left:initial(),right:initial()}),selected=ref<Record<ComparisonSide,Set<string>>>({left:new Set(),right:new Set()}),query=ref(''),filter=ref('all'),page=ref(1),expanded=ref(''),busy=ref(false),error=ref(''),notice=ref(''),issues=ref<Partial<Record<ComparisonSide,string>>>({}),admitted=ref(0)
 const options=ref({outputDirectory:'',keepStructure:true,separateSides:true,conflictPolicy:'rename' as 'rename'|'skip'})
 const ready=computed(()=>!!sides.value.left.source&&!!sides.value.right.source&&!sides.value.left.jobId&&!sides.value.right.jobId)
 const rows=computed(()=>ready.value?compareEntries(sides.value.left.source!.entries,sides.value.right.source!.entries):[])
 const counts=computed(()=>rows.value.reduce((all,row)=>({...all,[row.kind]:(all[row.kind]||0)+1}),{} as Partial<Record<ComparisonKind,number>>))
 const filtered=computed(()=>rows.value.filter(row=>(filter.value==='all'||filter.value==='changes'&&!['candidate','directory'].includes(row.kind)||row.kind===filter.value)&&row.path.toLowerCase().includes(query.value.trim().toLowerCase())))
 const pages=computed(()=>Math.max(1,Math.ceil(filtered.value.length/50))),visible=computed(()=>filtered.value.slice((page.value-1)*50,page.value*50))
 const selectedCount=computed(()=>selected.value.left.size+selected.value.right.size)
 let unlisten:UnlistenFn|undefined,epoch=0
 async function attach(){const current=++epoch;const stop=await listen<{jobId:string;progress:{entries:number}}>('comparison-read-progress',event=>{for(const side of ['left','right'] as ComparisonSide[])if(sides.value[side].jobId===event.payload.jobId)sides.value[side].readCount=event.payload.progress.entries});if(current!==epoch)stop();else unlisten=stop}
 async function cancel(side:ComparisonSide){const id=sides.value[side].jobId;if(!id)return;sides.value[side].jobId='';sides.value[side].error='读取已停止，请重新读取这一侧';await invoke('library_cancel',{jobId:id})}
 function detach(){epoch++;unlisten?.();unlisten=undefined;for(const side of ['left','right'] as ComparisonSide[])void cancel(side)}
 async function read(side:ComparisonSide,path:string){
  if(busy.value)return;await cancel(side);const id=crypto.randomUUID();sides.value[side]={path,jobId:id,error:'',readCount:0};selected.value[side]=new Set();expanded.value='';page.value=1;issues.value={};error.value=''
  try{const source=await invoke<ComparisonSource>('comparison_read',{path,jobId:id});if(sides.value[side].jobId===id){sides.value[side].source=source;sides.value[side].path=source.source.path}}
  catch(cause){if(sides.value[side].jobId===id)sides.value[side].error=String(cause)}finally{if(sides.value[side].jobId===id)sides.value[side].jobId=''}
 }
 function editPath(side:ComparisonSide){sides.value[side].source=undefined;selected.value[side]=new Set();page.value=1;expanded.value=''}
 function toggle(side:ComparisonSide,path:string){if(busy.value||!ready.value||!sides.value[side].source?.entries.some(e=>e.path===path&&e.selectable))return;const next=new Set(selected.value[side]);if(next.has(path))next.delete(path);else{if(selectedCount.value>=2000){error.value='每批最多选择 2000 项，请先取回这一批';return}next.add(path)}selected.value[side]=next;notice.value=''}
 function clear(){if(busy.value)return;selected.value={left:new Set(),right:new Set()};issues.value={}}
 function togglePage(side:ComparisonSide){const entries=visible.value.map(row=>row[side]).filter(entry=>entry?.selectable);const all=entries.length>0&&entries.every(entry=>selected.value[side].has(entry!.path));for(const entry of entries)if(all||!selected.value[side].has(entry!.path))toggle(side,entry!.path)}
 function request(side:ComparisonSide):ComparisonSelection|null{const source=sides.value[side].source;if(!source||!selected.value[side].size)return null;return copyComparisonSelection({source:source.source,paths:[...selected.value[side]].sort(),keepStructure:options.value.keepStructure,conflictPolicy:options.value.conflictPolicy})}
 async function queue(){
  if(busy.value||!ready.value||!selectedCount.value)return;if(!options.value.outputDirectory.trim()){error.value='请先选择取回位置';return}
  busy.value=true;error.value='';issues.value={};admitted.value=0;let amount=0
  try{
   await invoke('comparison_output',{sources:[sides.value.left.source!.source,sides.value.right.source!.source],output:options.value.outputDirectory})
   for(const side of ['left','right'] as ComparisonSide[]){if(!selected.value[side].size)continue
    try{const selection=request(side);if(!selection)throw Error('所选范围无效，请缩小范围后重新勾选');const source=sides.value[side].source!,name=(side==='left'?'左侧-':'右侧-')+source.name.replace(/[<>:"/\\|?*\x00-\x1f]/g,'_').replace(/[. ]+$/,'').slice(0,80)
     const output=options.value.separateSides?options.value.outputDirectory.replace(/[\\/]+$/,'')+'/'+name:options.value.outputDirectory
     await invoke('comparison_validate',{request:selection,output})
     const duplicate=tasks.tasks.some(task=>task.status==='pending'&&task.outputPath===output&&JSON.stringify(task.comparisonSelection)===JSON.stringify(selection))
     if(!duplicate)tasks.addTask({id:crypto.randomUUID(),name:`${side==='left'?'左侧':'右侧'} · ${source.name} · 所选 ${selection.paths.length} 项`,type:'decompression',sourceFiles:[source.source.path],outputPath:output,configurationMode:'individual',format:source.source.kind==='zip'?'ZIP':'文件夹',extractToSubfolder:false,recycleSourceAfterExtract:false,selectedEntries:[...selection.paths],comparisonSelection:selection,executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:output,extractToSubfolder:false,comparisonSelection:selection}})
     admitted.value++;amount+=selection.paths.length;selected.value[side]=new Set()
    }catch(cause){issues.value[side]=String(cause).replace(/^Error:\s*/,'')}
   }
   notice.value=`已将 ${amount} 项按 ${admitted.value} 份来源加入解压中心待处理队列`+(selectedCount.value?'；剩余项请处理来源后继续':'，核对后点击开始取回')
  }catch(cause){error.value=String(cause)}finally{busy.value=false}
 }
 return {sides,selected,query,filter,page,expanded,busy,error,notice,issues,admitted,options,ready,rows,counts,filtered,pages,visible,selectedCount,attach,detach,cancel,read,toggle,clear,togglePage,request,queue,editPath}
})
