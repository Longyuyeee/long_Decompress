<script setup lang="ts">
import {computed,ref,watch} from 'vue'
import {invoke} from '@tauri-apps/api/tauri'
import Modal from '@/components/ui/Modal.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import {snapshotDate} from '@/types/zpaqSnapshot'
import type {ZpaqRetrievalSelection,ZpaqRetrievalReceipt} from '@/types/zpaqRetrieval'
const props=defineProps<{path:string;selection:ZpaqRetrievalSelection}>()
const visible=ref(false),busy=ref(false),error=ref(''),report=ref<ZpaqRetrievalReceipt>(),filter=ref('all'),query=ref(''),page=ref(1)
const label=(state:string)=>({retrieved:'已取回',affected:'受影响，未取回',unchecked:'未检查，未取回',not_selected:'未选择'}[state]||state)
const counts=computed(()=>Object.fromEntries(['retrieved','affected','unchecked','not_selected'].map(s=>[s,report.value?.entries.filter(e=>e.state===s).length||0])))
const matches=computed(()=>report.value?.entries.filter(e=>(filter.value==='all'||e.state===filter.value)&&e.path.toLowerCase().includes(query.value.trim().toLowerCase()))||[])
const pages=computed(()=>Math.max(1,Math.ceil(matches.value.length/20))),rows=computed(()=>matches.value.slice((page.value-1)*20,page.value*20))
watch([filter,query],()=>page.value=1)
async function show(){visible.value=true;busy.value=true;error.value='';report.value=undefined;query.value='';filter.value='all';page.value=1;try{report.value=await invoke<ZpaqRetrievalReceipt>('read_zpaq_retrieval_result',{path:props.path,selection:props.selection})}catch(e){error.value=String(e)}finally{busy.value=false}}
</script>
<template>
 <button type="button" class="result-button border border-subtle text-primary" data-testid="zpaq-retrieval-result-open" @click.stop="show"><i class="pi pi-list"/>部分取回说明</button>
 <Modal :visible="visible" title="部分取回说明" icon="pi pi-list" size="lg" @close="visible=false">
  <div class="grid min-w-0 gap-3 text-xs" data-testid="zpaq-retrieval-result">
   <p v-if="busy" class="text-muted">正在读取完成记录…</p><p v-else-if="error" role="alert" class="break-all text-red-500">{{error}}</p>
   <template v-else-if="report">
    <div class="flex flex-wrap items-center gap-2 text-content"><strong>快照 {{report.snapshot.version}}</strong><span class="text-muted">{{snapshotDate(report.snapshot.date)}}</span><HelpHint label="取回结果范围">这是本次完成时的独立记录。只发布所选且完整校验通过的文件；受影响、未检查及未选择内容均未取回，不代表整个归档已经修复。原包保留。</HelpHint></div>
    <div class="flex flex-wrap gap-2 text-muted" data-testid="zpaq-retrieval-counts"><strong class="text-green-600">{{counts.retrieved}} 已取回</strong><span>{{counts.affected}} 受影响 · {{counts.unchecked}} 未检查 · {{counts.not_selected}} 未选择</span></div>
    <div class="flex flex-wrap gap-2"><input v-model="query" class="min-w-0 flex-1 rounded-lg border border-subtle bg-input p-2 text-content" placeholder="搜索文件名或路径" aria-label="搜索取回结果"/><select v-model="filter" class="rounded-lg border border-subtle bg-input p-2 text-content" data-testid="zpaq-retrieval-result-filter" aria-label="取回状态"><option value="all">全部结果</option><option value="retrieved">已取回</option><option value="affected">受影响</option><option value="unchecked">未检查</option><option value="not_selected">未选择</option></select></div>
    <div role="list" class="rounded-xl border border-subtle overflow-hidden" data-testid="zpaq-retrieval-result-list"><article v-for="row in rows" :key="row.path" role="listitem" :data-state="row.state" class="flex min-w-0 flex-wrap items-center gap-2 border-b border-subtle last:border-b-0 p-3"><strong class="min-w-0 flex-1 break-all text-content">{{row.path}}</strong><span class="text-muted">{{row.size.toLocaleString()}} bytes</span><span :class="row.state==='retrieved'?'text-green-600':'text-muted'">{{label(row.state)}}</span></article><p v-if="!rows.length" class="p-5 text-center text-muted">没有符合条件的文件</p></div>
   </template>
  </div>
  <template #footer><div class="flex w-full flex-wrap items-center justify-end gap-2 text-xs"><span v-if="report" class="mr-auto text-muted">{{matches.length}} 项 · {{page}} / {{pages}} 页</span><button v-if="report" :disabled="page===1" class="receipt-button" aria-label="上一页" @click="page--"><i class="pi pi-angle-left"/></button><button v-if="report" :disabled="page===pages" class="receipt-button" aria-label="下一页" @click="page++"><i class="pi pi-angle-right"/></button><button class="receipt-button" @click="visible=false">关闭</button></div></template>
 </Modal>
</template>
<style scoped>
.result-button{display:inline-flex;min-height:2rem;align-items:center;gap:.4rem;border-radius:.6rem;padding:.4rem .7rem;font-size:.75rem;transition:background .15s}.result-button:focus-visible{outline:2px solid var(--dynamic-accent);outline-offset:2px}
.receipt-button{padding:.6rem .8rem;border:1px solid var(--border-subtle);border-radius:.65rem;background:var(--bg-input);color:var(--text-content);font-weight:700}.receipt-button:disabled{opacity:.35}
</style>
