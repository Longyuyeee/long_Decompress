<script setup lang="ts">
import { computed,ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import { useZipEditStore } from '@/stores/zipEdit'
import { useAppStore } from '@/stores/app'
const props=defineProps<{sourcePath:string}>(),store=useZipEditStore(),app=useAppStore(),busy=ref(false),error=ref('')
const session=computed(()=>store.sessionFor(props.sourcePath))
const issues=computed(()=>session.value?.drafts.filter(row=>!row.comparison&&session.value!.inputIssues[row.id])||[])
const queued=()=>import.meta.env.VITE_DESKTOP_E2E==='1'?window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection():undefined
async function locate(id?:string,current=false){busy.value=true;error.value='';try{
 const draft=id?session.value?.drafts.find(row=>row.id===id):undefined
 let path:string|undefined
 if(!current){const selection=queued(),picked=selection===undefined?await open({directory:!!draft?.isDir,multiple:false,title:draft?'重新定位编辑来源':'重新定位原 ZIP',...(!draft?{filters:[{name:'ZIP',extensions:['zip']}]}:{})}):selection;path=typeof picked==='string'?picked:Array.isArray(picked)?picked[0]:undefined;if(!path)return}
 if(id)await store.relinkInput(props.sourcePath,id,path)
 else if(path){const relocated=await store.relocateSource(props.sourcePath,path);if(relocated)app.openArchiveInBrowser(relocated,{password:store.sessionFor(relocated)?.sourcePassword,zipPasswordEncoding:store.sessionFor(relocated)?.sourceEncoding})}
 }catch(value){error.value=String(value)}finally{busy.value=false}}
</script>
<template>
 <section v-if="session&&(session.sourceIssue||issues.length)" class="min-w-0 space-y-3 rounded-xl border border-amber-500/25 bg-amber-500/5 p-3" data-testid="zip-edit-source-status">
  <strong class="text-xs text-amber-600">恢复来源需要处理 · 修改清单保留</strong>
  <div v-if="session.sourceIssue" class="min-w-0 space-y-2"><p class="break-words text-xs leading-relaxed text-muted [overflow-wrap:anywhere]">{{ session.sourceIssue }} · {{ sourcePath }}</p><div class="flex flex-wrap gap-2"><button class="source-button" :disabled="busy||session.busy" data-testid="zip-edit-relocate-archive" @click="locate()">重新定位原 ZIP</button><button class="source-button" :disabled="busy||session.busy" data-testid="zip-edit-accept-source" @click="store.refresh(sourcePath)">读取当前原包并确认</button></div></div>
  <ul class="max-h-60 space-y-3 overflow-y-auto custom-scrollbar"><li v-for="row in issues.slice(0,100)" :key="row.id" class="min-w-0 space-y-2" :data-source-issue="row.id"><p class="break-words text-xs leading-relaxed text-muted [overflow-wrap:anywhere]">{{ session.inputIssues[row.id] }} · {{ row.sourcePath }} → {{ row.targetPath }}</p><div class="flex flex-wrap gap-2"><button class="source-button" :disabled="busy||session.busy" data-testid="zip-edit-relink-input" @click="locate(row.id)">重新定位</button><button class="source-button" :disabled="busy||session.busy" data-testid="zip-edit-accept-input" @click="locate(row.id,true)">读取当前来源并确认</button></div></li></ul><p v-if="issues.length>100" class="text-xs text-muted">当前显示前 100 项，处理后继续显示其余来源。</p>
  <p v-if="error" role="alert" class="break-words text-xs text-red-500 [overflow-wrap:anywhere]">{{ error }}</p>
 </section>
</template>
<style scoped>.source-button{display:inline-flex;align-items:center;min-height:2rem;border:1px solid var(--border-subtle);border-radius:.65rem;padding:.45rem .65rem;font-size:.75rem;color:var(--dynamic-accent)}button:disabled{opacity:.45}</style>
