import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useArchiveLibraryStore } from './archiveLibrary'
import { useTaskStore } from './task'
import { isZipPasswordEncoding } from '@/types/zipPassword'
import { libraryHitKey, type LibraryHit, type LibrarySource } from '@/types/archiveLibrary'

export interface RetrievalOptions { outputDirectory:string; keepStructure:boolean; separateSources:boolean; conflictPolicy:'rename'|'skip' }
export const useLibraryRetrievalStore = defineStore('libraryRetrieval', () => {
  const library=useArchiveLibraryStore(),tasks=useTaskStore()
  const selected=ref<Record<string,LibraryHit>>({}),busy=ref(false),error=ref(''),notice=ref('')
  const issues=ref<Record<string,string>>({}), admitted=ref(0)
  const options=ref<RetrievalOptions>({outputDirectory:'',keepStructure:true,separateSources:true,conflictPolicy:'rename'})
  const items=computed(()=>Object.values(selected.value))
  const groups=computed(()=>[...new Set(items.value.map(hit=>hit.sourceId))].map(id=>({id,name:library.sources.find(row=>row.id===id)?.name||items.value.find(hit=>hit.sourceId===id)!.sourceName,hits:items.value.filter(hit=>hit.sourceId===id)})))
  const has=(hit:LibraryHit)=>!!selected.value[libraryHitKey(hit)]
  function toggle(hit:LibraryHit) {
    if(busy.value)return
    const key=libraryHitKey(hit)
    if(selected.value[key]){delete selected.value[key];return}
    if(items.value.length>=2000){error.value='每批最多选择 2000 个文件，请先取回这批再继续';return}
    if(!items.value.length){notice.value='';admitted.value=0}
    selected.value[key]={...hit,revision:hit.revision||library.sources.find(row=>row.id===hit.sourceId)?.revision||''}
  }
  function clear(){if(!busy.value){selected.value={};issues.value={}}}
  function togglePage(hits:LibraryHit[]){const all=hits.every(has);for(const hit of hits)if(all||!has(hit))toggle(hit)}
  async function checkGroup(group:typeof groups.value[number]):Promise<LibrarySource>{
    const revisions=new Set(group.hits.map(hit=>hit.revision))
    if(revisions.size!==1)throw Error('此来源含不同索引的选择，请清除此来源后重新勾选')
    const source=await invoke<LibrarySource>('library_selection',{sourceId:group.id,revision:group.hits[0].revision||'',paths:group.hits.map(hit=>hit.path)})
    library.sources=library.sources.map(row=>row.id===source.id?source:row)
    if(source.encrypted&&!library.credentials[source.id]?.password)throw Error('需要来源密码，请填写或从保险箱选择后继续')
    return source
  }
  async function inspect(){
    if(busy.value)return
    busy.value=true;issues.value={};error.value=''
    try {for(const group of groups.value){try{await checkGroup(group)}catch(reason){issues.value[group.id]=String(reason).replace(/^Error:\s*/,'')}}}
    finally{busy.value=false}
  }
  async function queue(){
    if(busy.value||!items.value.length)return
    if(!options.value.outputDirectory.trim()){error.value='请先选择取回位置';return}
    busy.value=true;error.value='';issues.value={};admitted.value=0;let files=0
    try {
      for(const group of [...groups.value]){
        try {
          const source=await checkGroup(group)
          const revision=group.hits[0].revision||'',paths=group.hits.map(hit=>hit.path)
          if(JSON.stringify(paths).length>60000)throw Error('这些文件路径过长，请减少所选文件后分批取回')
          const duplicate=tasks.tasks.find(task=>task.status==='pending'&&task.librarySourceId===source.id&&task.libraryRevision===revision&&JSON.stringify([...(task.selectedEntries||[])].sort())===JSON.stringify([...paths].sort()))
          if(!duplicate){
            const credential=library.credentials[source.id],encoding=credential?.encoding||(isZipPasswordEncoding(source.sourceEncoding)?source.sourceEncoding:undefined)
            const folder=source.name.replace(/\.[^.]+$/,'').replace(/[<>:"/\\|?*\x00-\x1f]/g,'_').replace(/[. ]+$/,'').slice(0,80)||'资料'
            const output=options.value.separateSources?options.value.outputDirectory.replace(/[\\/]+$/,'')+'/'+folder+'-'+source.id.slice(0,8):options.value.outputDirectory
            tasks.addTask({id:crypto.randomUUID(),name:`${source.name} · 所选 ${paths.length} 个文件`,type:'decompression',sourceFiles:[source.path],format:source.format,outputPath:output,configurationMode:'individual',extractToSubfolder:false,recycleSourceAfterExtract:false,selectedEntries:paths,librarySourceId:source.id,libraryRevision:revision,libraryKeepStructure:options.value.keepStructure,libraryConflictPolicy:options.value.conflictPolicy,password:credential?.password,zipPasswordEncoding:encoding,draftPasswordRequired:source.encrypted&&!credential?.password})
          }
          admitted.value++;files+=paths.length;for(const hit of group.hits)delete selected.value[libraryHitKey(hit)]
        } catch(reason){issues.value[group.id]=String(reason).replace(/^Error:\s*/,'')}
      }
      notice.value=`已将 ${files} 个文件按 ${admitted.value} 份来源加入解压中心待处理队列`+(items.value.length?`；剩余 ${items.value.length} 个文件请处理来源后继续`:'，核对后点击开始即可')
    }finally{busy.value=false}
  }
  return{selected,busy,error,notice,issues,admitted,options,items,groups,has,toggle,togglePage,clear,inspect,queue}
})
