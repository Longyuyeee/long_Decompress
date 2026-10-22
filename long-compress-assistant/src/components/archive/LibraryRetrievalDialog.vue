<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import { onMounted } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import { useLibraryRetrievalStore } from '@/stores/libraryRetrieval'
import { libraryHitKey } from '@/types/archiveLibrary'
const emit=defineEmits<{close:[];fix:[id:string];queue:[]}>()
const retrieval=useLibraryRetrievalStore()
onMounted(()=>void retrieval.inspect())
async function chooseDirectory(){const queued=import.meta.env.VITE_DESKTOP_E2E==='1'?window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection():undefined;const path=queued===undefined?await open({directory:true,title:'选择批量取回位置'}):queued;if(typeof path==='string')retrieval.options.outputDirectory=path}
</script>
<template>
  <Modal :visible="true" title="批量取回所选文件" icon="pi pi-download" size="xl" :show-footer="true" :show-close-button="!retrieval.busy" :close-on-backdrop="!retrieval.busy" :close-on-escape="!retrieval.busy" @update:visible="!retrieval.busy && emit('close')">
    <div data-testid="library-retrieval" class="retrieval-grid">
      <section class="retrieval-card"><h3>本次选择 <span>{{ retrieval.items.length }} 个文件 · {{ retrieval.groups.length }} 份来源</span><HelpHint label="取回范围说明">只取回下列文件。来源需处理时保留选择，可先加入其他可用部分。</HelpHint></h3>
        <div class="retrieval-list custom-scrollbar"><article v-for="group in retrieval.groups" :key="group.id" :data-retrieval-source="group.name" class="retrieval-source"><strong class="break-all">{{ group.name }}</strong><p class="retrieval-note">{{ group.hits.length }} 个文件</p><p v-if="retrieval.issues[group.id]" class="retrieval-issue" role="status">{{ retrieval.issues[group.id] }} <button :disabled="retrieval.busy" class="text-primary underline" @click="emit('fix',group.id)">处理来源</button></p><p v-else class="retrieval-ready">{{ retrieval.busy ? '正在确认来源…' : '可加入队列' }}</p><details><summary>查看所选路径</summary><div v-for="hit in group.hits" :key="libraryHitKey(hit)" class="retrieval-path"><span>{{ hit.path }}</span><button :disabled="retrieval.busy" aria-label="移除此文件" @click="retrieval.toggle(hit)"><i class="pi pi-times" /></button></div></details></article><p v-if="!retrieval.items.length" class="retrieval-note">本批所选文件已加入队列。前往解压中心核对并开始。</p></div>
      </section>
      <section class="retrieval-card"><h3>取回到哪里<HelpHint label="取回队列说明">按来源加入原解压中心的待处理队列，核对后再开始。可继续查看进度、日志和结果，原包保留。</HelpHint></h3><fieldset :disabled="retrieval.busy" class="space-y-4 mt-4 min-w-0"><label class="block text-xs font-bold">输出目录<div class="flex gap-2 mt-2 min-w-0"><input v-model="retrieval.options.outputDirectory" class="retrieval-control min-w-0 flex-1" data-testid="library-retrieval-output" placeholder="选择或输入保存位置"><button class="retrieval-control shrink-0" data-testid="library-retrieval-choose-output" @click="chooseDirectory">选择</button></div></label>
        <label class="block text-xs font-bold">来源之间<select v-model="retrieval.options.separateSources" class="retrieval-control w-full mt-2" data-testid="library-retrieval-separate"><option :value="true">每份来源独立子目录（推荐）</option><option :value="false">合并到同一输出目录</option></select></label>
        <label class="block text-xs font-bold">包内目录<select v-model="retrieval.options.keepStructure" class="retrieval-control w-full mt-2" data-testid="library-retrieval-structure"><option :value="true">保留包内文件夹结构</option><option :value="false">只取文件，放在同一层</option></select></label>
        <label class="block text-xs font-bold">同名文件<select v-model="retrieval.options.conflictPolicy" class="retrieval-control w-full mt-2" data-testid="library-retrieval-conflict"><option value="rename">自动改名，保留两份</option><option value="skip">跳过已存在文件</option></select></label>
      </fieldset></section>
    </div>
    <p v-if="retrieval.error" role="alert" class="retrieval-issue mt-3">{{ retrieval.error }}</p><p v-if="retrieval.notice" data-testid="library-retrieval-notice" role="status" class="retrieval-note mt-3">{{ retrieval.notice }}</p>
    <template #footer><button class="retrieval-control" :disabled="retrieval.busy" @click="emit('close')">返回</button><button v-if="retrieval.admitted" class="retrieval-control text-primary" data-testid="library-retrieval-go-queue" :disabled="retrieval.busy" @click="emit('queue')">前往解压中心</button><button class="retrieval-control primary" data-testid="library-retrieval-confirm" :disabled="retrieval.busy || !retrieval.items.length || retrieval.groups.every(group=>!!retrieval.issues[group.id])" @click="retrieval.queue">{{ retrieval.busy ? '正在准备…' : '将可用部分加入队列' }}</button></template>
  </Modal>
</template>
<style scoped>
.retrieval-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:18px}.retrieval-card{min-width:0;border:1px solid var(--border-subtle);border-radius:16px;padding:18px;background:var(--bg-card)}h3{font-weight:900;font-size:14px}h3 span{display:block;font-size:11px;color:var(--dynamic-accent);margin-top:8px}.retrieval-note{font-size:11px;color:var(--text-muted);line-height:1.9;overflow-wrap:anywhere}.retrieval-list{max-height:260px;overflow:auto;margin-top:14px}.retrieval-source{border:1px solid var(--border-subtle);border-radius:12px;padding:12px;margin-bottom:10px;font-size:12px}.retrieval-ready{color:#16a34a;font-size:11px;margin:8px 0}.retrieval-issue{font-size:11px;color:#d97706;line-height:1.8;overflow-wrap:anywhere}.retrieval-control{border:1px solid var(--border-subtle);border-radius:10px;padding:10px 12px;background:var(--bg-input);font-size:12px;min-width:0}.retrieval-control.primary{background:var(--dynamic-accent);color:white}.retrieval-control:disabled{opacity:.5}.retrieval-path{display:flex;gap:8px;font-size:11px;padding:6px 0;overflow-wrap:anywhere}.retrieval-path span{flex:1;min-width:0}summary{cursor:pointer;color:var(--text-muted);font-size:11px;margin-top:8px}@media(max-width:800px){.retrieval-grid{grid-template-columns:minmax(0,1fr)}.retrieval-card{padding:14px}.retrieval-list{max-height:180px}}

.retrieval-card>h3{display:flex;align-items:center;gap:8px;flex-wrap:wrap}.retrieval-card>h3>span{display:inline;margin:0}
</style>
