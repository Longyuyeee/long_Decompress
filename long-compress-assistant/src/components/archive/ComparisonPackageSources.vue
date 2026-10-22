<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import {computed,ref,watchEffect} from 'vue'
import {open} from '@tauri-apps/api/dialog'
import {useZipEditStore} from '@/stores/zipEdit'
import ComparisonPasswordField from './ComparisonPasswordField.vue'
const props=defineProps<{sourcePath:string}>(),store=useZipEditStore(),busy=ref(''),error=ref('')
const session=computed(()=>store.sessionFor(props.sourcePath))
const sources=computed(()=>[...new Map((session.value?.drafts||[]).flatMap(row=>row.comparison?[[row.comparison.source.path,row.comparison.source] as const]:[])).values()])
watchEffect(()=>{if(!session.value||!sources.value.length)return;session.value.comparisonPasswords??={};for(const source of sources.value)session.value.comparisonPasswords[source.path]??={password:'',encoding:session.value.drafts.find(row=>row.comparison?.source.path===source.path)?.comparison?.encoding}})
watchEffect(()=>{if(!session.value)return;for(const row of session.value.drafts)if(row.comparison&&session.value.comparisonPasswords?.[row.comparison.source.path])row.comparison.encoding=session.value.comparisonPasswords[row.comparison.source.path]!.encoding})
async function accept(origin:string,locate=false){
 if(busy.value)return;busy.value=origin;error.value=''
 try{let path:string|undefined
  if(locate){const queued=import.meta.env.VITE_DESKTOP_E2E==='1'?window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection():undefined,picked=queued===undefined?await open({directory:sources.value.find(s=>s.path===origin)?.kind==='directory',title:'重新定位这份组包来源',multiple:false}):queued;path=typeof picked==='string'?picked:undefined;if(!path)return}
  await store.acceptComparisonSource(props.sourcePath,origin,path)
 }catch(cause){error.value=String(cause)}finally{busy.value=''}
}
</script>
<template>
 <section v-if="sources.length&&session" class="space-y-3 min-w-0" data-testid="comparison-package-sources">
  <strong class="block text-xs text-content">组包来源 · {{ sources.length }} 份</strong>
  <span class="context-help"><HelpHint label="草稿来源说明">按草稿引用读取，不保存解密文件。来源变化后重新读取并确认；不会加入新出现的文件。重启后请补充密码。</HelpHint></span>
  <article v-for="(source,index) in sources" :key="source.path" class="min-w-0 space-y-3 rounded-xl border border-subtle bg-input/20 p-3" :data-package-source="source.path">
   <div class="flex min-w-0 items-start gap-2"><i :class="source.kind==='zip'?'pi pi-box':'pi pi-folder'" class="mt-1 shrink-0 text-primary"/><strong class="min-w-0 break-words text-xs [overflow-wrap:anywhere]" :title="source.path">{{ source.path.replace(/\\/g,'/').split('/').pop() }}</strong><span class="ml-auto shrink-0 text-xs text-muted">{{ session.drafts.filter(row=>row.comparison?.source.path===source.path).length }} 项</span></div><details class="min-w-0 text-xs text-muted"><summary class="cursor-pointer">查看来源路径</summary><p class="mt-2 break-words leading-relaxed [overflow-wrap:anywhere]">{{ source.path }}</p></details>
   <p v-if="session.drafts.some(row=>row.comparison?.source.path===source.path&&session!.inputIssues[row.id])" class="text-xs text-amber-600">此来源已变化、缺失或不可访问，请处理后继续。</p>
   <ComparisonPasswordField v-if="source.kind==='zip'&&source.encrypted&&session.comparisonPasswords?.[source.path]" v-model:password="session.comparisonPasswords[source.path]!.password" v-model:encoding="session.comparisonPasswords[source.path]!.encoding" :disabled="session.busy||!!busy" :test-id="'package-source-'+index"/>
   <div class="flex flex-wrap gap-2"><button class="source-button" :disabled="session.busy||!!busy" data-testid="package-source-accept" @click="accept(source.path)">重新读取并确认来源</button><button class="source-button" :disabled="session.busy||!!busy" data-testid="package-source-relocate" @click="accept(source.path,true)">重新定位</button></div>
  </article>
  <p v-if="error" role="alert" class="break-words text-xs text-amber-600 [overflow-wrap:anywhere]">{{ error }}</p>
 </section>
</template>
<style scoped>.source-button{border:1px solid var(--border-subtle);border-radius:8px;padding:8px;font-size:11px;color:var(--dynamic-accent)}button:disabled{opacity:.45}</style>
