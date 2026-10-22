<script setup lang="ts">
import {computed,ref,watch} from 'vue'
import Modal from '@/components/ui/Modal.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import {snapshotDate} from '@/types/zpaqSnapshot'
import {zpaqCheckCounts,type ZpaqBlockReport,type ZpaqCheckedFile,type ZpaqBlock} from '@/types/zpaqBlocks'
const props=defineProps<{report:ZpaqBlockReport}>()
const visible=ref(false),tab=ref<'files'|'blocks'>('files'),filter=ref('all'),query=ref(''),page=ref(1),scope=ref<number[]>([])
const counts=computed(()=>zpaqCheckCounts(props.report))
const category=(state:string)=>['affected','failed'].includes(state)?'issues':state
const label=(state:string)=>({verified:'检查通过',failed:'校验失败',affected:'受影响',unchecked:'未检查'}[state]||state)
const filtered=computed(()=>{
 const rows=tab.value==='files'?props.report.entries:props.report.blocks
 const term=query.value.trim().toLowerCase()
 return rows.filter(row=>(filter.value==='all'||category(row.state)===filter.value)
  &&(!scope.value.length||('path'in row?row.blocks.some(id=>scope.value.includes(id)):scope.value.includes(row.id)))
  &&(!term||('path'in row?row.path:`数据块 #${row.id}`).toLowerCase().includes(term)))
})
const pages=computed(()=>Math.max(1,Math.ceil(filtered.value.length/20))),rows=computed(()=>filtered.value.slice((page.value-1)*20,page.value*20))
watch([query,filter,tab],()=>{page.value=1})
watch(()=>props.report,()=>{visible.value=false;scope.value=[];query.value='';page.value=1})
function show(){tab.value='files';filter.value=counts.value.affected?'issues':counts.value.unchecked?'unchecked':'all';scope.value=[];query.value='';page.value=1;visible.value=true}
function linkedBlocks(file:ZpaqCheckedFile){scope.value=[...file.blocks];filter.value='all';query.value='';tab.value='blocks';page.value=1}
function linkedFiles(block:ZpaqBlock){scope.value=[block.id];filter.value='all';query.value='';tab.value='files';page.value=1}
function clearScope(){scope.value=[];page.value=1}
</script>
<template>
 <section class="block-summary" data-testid="zpaq-block-summary">
  <div><strong><i class="pi pi-sitemap"/>快照 {{report.snapshot.version}} · 文件检查</strong><span><b class="verified">{{counts.verified}}</b> 通过 · <b class="affected">{{counts.affected}}</b> 受影响 · <b>{{counts.unchecked}}</b> 未检查</span></div>
  <button class="block-button" data-testid="zpaq-block-details-open" @click="show">查看文件影响<i class="pi pi-angle-right"/></button>
 </section>
 <Modal :visible="visible" title="快照文件检查" icon="pi pi-sitemap" size="lg" @close="visible=false">
  <div class="block-details" data-testid="zpaq-block-details">
   <div class="block-scope"><strong>快照 {{report.snapshot.version}}</strong><span>{{snapshotDate(report.snapshot.date)}}</span><HelpHint label="文件检查范围">只检查所选快照引用的数据块，每个块读取一次。检查通过代表关联块及片段内容校验通过；共享块失败仅说明关联文件受影响，不能断定每个文件都已损坏。超出解码内存范围的内容保持未检查，本版未执行部分取回。</HelpHint></div>
   <div class="block-tools"><div class="block-tabs" role="tablist" aria-label="检查内容"><button role="tab" :aria-selected="tab==='files'" :class="{active:tab==='files'}" data-testid="zpaq-block-files-tab" @click="tab='files'">文件 {{report.entries.length}}</button><button role="tab" :aria-selected="tab==='blocks'" :class="{active:tab==='blocks'}" data-testid="zpaq-block-blocks-tab" @click="tab='blocks'">数据块 {{report.blocks.length}}</button></div><select v-model="filter" aria-label="检查状态" data-testid="zpaq-block-filter"><option value="all">全部状态</option><option value="issues">{{tab==='files'?'受影响':'校验失败'}}</option><option value="verified">检查通过</option><option value="unchecked">未检查</option></select></div>
   <input v-model="query" class="block-search" :placeholder="tab==='files'?'搜索文件名或路径':'搜索数据块编号'" aria-label="搜索检查结果" data-testid="zpaq-block-query"/>
   <button v-if="scope.length" class="block-linked" data-testid="zpaq-block-clear-scope" @click="clearScope">{{scope.length}} 个数据块的关联范围 <i class="pi pi-times"/></button>
   <div class="block-list" role="list" data-testid="zpaq-block-list">
    <article v-for="row in rows" :key="'path'in row?row.path:row.id" role="listitem" :data-state="row.state" class="block-row">
     <div class="block-row-copy"><strong :title="'path'in row?row.path:undefined">{{'path'in row?row.path:`数据块 #${row.id}`}}</strong><span v-if="!('path'in row)">偏移 {{row.offset.toLocaleString()}} · {{row.compressedSize.toLocaleString()}} bytes</span><span v-else>{{row.size.toLocaleString()}} bytes</span></div>
     <span class="block-state" :class="row.state">{{label(row.state)}}</span>
     <button v-if="'path'in row&&row.blocks.length" class="block-link" data-testid="zpaq-file-blocks" @click="linkedBlocks(row)">{{row.blocks.length}} 个关联块</button><button v-else-if="!('path'in row)" class="block-link" data-testid="zpaq-block-files" @click="linkedFiles(row)">{{row.files}} 个关联文件</button>
    </article>
    <p v-if="!rows.length" class="block-empty">没有符合条件的结果</p>
   </div>
  </div>
  <template #footer><div class="block-pagination"><span>{{filtered.length}} 项 · {{page}} / {{pages}} 页</span><button class="block-button" :disabled="page===1" aria-label="上一页" @click="page--"><i class="pi pi-angle-left"/></button><button class="block-button" :disabled="page===pages" aria-label="下一页" data-testid="zpaq-block-next-page" @click="page++"><i class="pi pi-angle-right"/></button><button class="block-button" @click="visible=false">关闭</button></div></template>
 </Modal>
</template>
<style scoped>
.block-summary{display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:.75rem;padding:.85rem 1rem;border:1px solid var(--border-subtle);border-radius:1rem;background:var(--bg-input);margin-top:1rem}.block-summary>div{display:grid;gap:.35rem;font-size:.78rem;min-width:0}.block-summary strong{display:flex;align-items:center;gap:.5rem;color:var(--text-content)}.block-summary span,.block-scope>span{color:var(--text-muted);font-size:.72rem}.verified{color:#16a34a}.affected,.failed{color:#d97706}.block-button,.block-tools select{border:1px solid var(--border-subtle);border-radius:.7rem;background:var(--bg-input);color:var(--text-content);font-size:.75rem;font-weight:700;padding:.6rem .8rem;display:inline-flex;align-items:center;gap:.4rem}.block-button:disabled{opacity:.35}.block-details{display:grid;gap:.8rem;min-width:0;font-size:.78rem}.block-scope{display:flex;flex-wrap:wrap;align-items:center;gap:.4rem .7rem;color:var(--text-content)}.block-tools{display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:.6rem}.block-tabs{display:inline-flex;gap:.2rem;padding:.25rem;border-radius:.8rem;background:var(--bg-input)}.block-tabs button{padding:.5rem .7rem;border-radius:.6rem;color:var(--text-muted);font-weight:700}.block-tabs button.active{background:var(--bg-card);color:var(--dynamic-accent)}.block-search{width:100%;min-width:0;padding:.7rem .8rem;border:1px solid var(--border-subtle);border-radius:.8rem;background:var(--bg-input);color:var(--text-content);outline:none}.block-search:focus{border-color:var(--dynamic-accent)}.block-linked{justify-self:start;font-size:.72rem;color:var(--dynamic-accent);display:flex;align-items:center;gap:.5rem}.block-list{min-width:0;border:1px solid var(--border-subtle);border-radius:.8rem;overflow:hidden}.block-row{display:flex;align-items:center;flex-wrap:wrap;gap:.5rem .8rem;padding:.7rem .8rem;border-bottom:1px solid var(--border-subtle)}.block-row:last-child{border-bottom:0}.block-row-copy{flex:1 1 12rem;min-width:0;display:grid;gap:.25rem}.block-row-copy strong{overflow-wrap:anywhere;color:var(--text-content);font-size:.75rem}.block-row-copy>span{font-size:.67rem;color:var(--text-muted)}.block-state{font-size:.68rem;font-weight:700;padding:.2rem .4rem;border-radius:.35rem;background:var(--bg-input);white-space:nowrap}.block-link{color:var(--dynamic-accent);font-size:.7rem}.block-pagination{display:flex;flex-wrap:wrap;align-items:center;justify-content:flex-end;gap:.4rem;width:100%}.block-pagination>span{margin-right:auto;color:var(--text-muted);font-size:.7rem}.block-empty{padding:1.5rem;text-align:center;color:var(--text-muted)}
</style>
