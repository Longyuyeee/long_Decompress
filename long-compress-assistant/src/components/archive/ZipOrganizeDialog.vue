<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import { computed, reactive, ref, watch } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import { useZipEditStore } from '@/stores/zipEdit'
import { previewZipStructure, zipBatchTargets, zipPathKey } from '@/utils/zipWorkspace'
import { formatFileSize } from '@/utils'
import type { ZipStructureOperation } from '@/types/zipEdit'
const props = defineProps<{ visible: boolean; sourcePath: string; initialPaths: string[]; initialMode: 'rename' | 'delete' }>()
const emit = defineEmits<{ 'update:visible': [value: boolean]; review: []; add: [directory: string, folder: boolean] }>()
const store = useZipEditStore(), query = ref(''), directory = ref(''), page = ref(1), selected = ref(new Set<string>())
const mode = ref<'rename' | 'delete'>('rename'), singlePath = ref(''), notice = ref('')
const rule = reactive({ prefix: '', suffix: '', search: '', replacement: '', destination: '' })
const session = computed(() => store.sessionFor(props.sourcePath))
const plan = computed(() => session.value ? store.plan(session.value) : null)
const nodes = computed(() => plan.value?.nodes || [])
const picked = computed(() => nodes.value.filter(node => selected.value.has(node.id)))
const roots = computed(() => picked.value.filter(node => !picked.value.some(other => other !== node && other.isDir && node.path.startsWith(other.path))))
const parent = (path: string) => { const clean = path.replace(/\/$/,''); const at = clean.lastIndexOf('/'); return at < 0 ? '' : clean.slice(0,at+1) }
const filtered = computed(() => nodes.value.filter(node => query.value.trim() ? node.path.toLowerCase().includes(query.value.trim().toLowerCase()) : parent(node.path) === directory.value).sort((a,b) => Number(b.isDir)-Number(a.isDir)||a.path.localeCompare(b.path)))
const pageCount = computed(() => Math.max(1,Math.ceil(filtered.value.length/100)))
const visibleNodes = computed(() => filtered.value.slice((page.value-1)*100,page.value*100))
const operation = computed<ZipStructureOperation>(() => ({ id: 'preview', order: 0, kind: mode.value, targets: mode.value === 'delete' ? roots.value.map(node => ({ id: node.id, path: null })) : roots.value.length === 1 ? [{ id: roots.value[0].id, path: singlePath.value }] : zipBatchTargets(roots.value,rule) }))
const preview = computed(() => previewZipStructure(nodes.value,operation.value))
const selectedAll = computed(() => visibleNodes.value.length > 0 && visibleNodes.value.every(node => selected.value.has(node.id)))
watch([query,directory],() => { page.value=1 })
watch(pageCount,count => { page.value=Math.min(page.value,count) })
watch(() => roots.value.map(node => node.id + ':' + node.path).join('|'),() => { singlePath.value = roots.value.length === 1 ? roots.value[0].path.replace(/\/$/,'') : '' })
watch(() => props.visible,visible => {
  if (!visible) return
  query.value=''; directory.value=''; page.value=1; notice.value=''; mode.value=props.initialMode
  Object.assign(rule,{prefix:'',suffix:'',search:'',replacement:'',destination:''})
  selected.value = new Set(nodes.value.filter(node => props.initialPaths.some(path => zipPathKey(path) === zipPathKey(node.originalPath || node.originalAlias || node.path))).map(node => node.id))
  if (props.initialPaths.length && !selected.value.size) notice.value='所选原条目已被替换或删除，请在当前清单重新选择。'
  if (roots.value.length === 1) singlePath.value=roots.value[0].path.replace(/\/$/,'')
}, { immediate: true })
watch(nodes,() => {
  const available=new Set(nodes.value.map(node=>node.id));selected.value=new Set([...selected.value].filter(id=>available.has(id)))
  if (directory.value && !nodes.value.some(node=>node.isDir&&node.path===directory.value)) directory.value=''
})
function toggle(id: string) { const value=new Set(selected.value);value.has(id)?value.delete(id):value.add(id);selected.value=value;notice.value='' }
function toggleAll() { const value=new Set(selected.value);for(const node of visibleNodes.value)selectedAll.value?value.delete(node.id):value.add(node.id);selected.value=value }
function apply() {
  if (preview.value.issue || !session.value || session.value.busy) return
  const count=preview.value.affected.length
  if (store.organize(props.sourcePath,mode.value,operation.value.targets)) { selected.value=new Set(); notice.value=`已加入待保存清单，影响 ${count} 项；另存后生效。` }
}
const locked = computed(() => session.value?.busy || store.preparing || store.opening)
</script>
<template>
  <Modal description="这里展示本次编辑后的目录结构。删除、改名和移动先进入待保存清单，原包保持不变。" description-as-help :visible="visible" @update:visible="emit('update:visible',$event)" title="整理 ZIP 内容" icon="pi pi-sitemap" size="xl" cancel-text="关闭整理窗口">

    <div class="grid min-w-0 gap-4 lg:grid-cols-2" data-testid="zip-organize-content">
      <section class="min-w-0 space-y-3" data-testid="zip-organize-tree">
        <div class="flex flex-wrap items-center gap-2"><strong class="mr-auto text-sm">编辑后的内容 · {{ nodes.length }} 项</strong><button class="organize-button" :disabled="locked" data-testid="zip-organize-add" @click="emit('add',directory,false)">添加文件</button></div>
        <label class="block"><span class="sr-only">搜索包内路径</span><input v-model="query" class="organize-input w-full" placeholder="搜索名称或包内路径" data-testid="zip-organize-search" /></label>
        <nav class="flex min-w-0 flex-wrap items-center gap-2 text-xs" aria-label="编辑目录"><button class="text-primary" data-testid="zip-organize-root" @click="directory='';query=''">包内根目录</button><template v-if="directory"><span>/</span><span class="min-w-0 flex-1 break-words [overflow-wrap:anywhere]">{{ directory }}</span><button class="organize-button" @click="directory=parent(directory)">上一级</button></template></nav>
        <div class="rounded-xl border border-subtle bg-input/20">
          <label class="flex items-center gap-2 border-b border-subtle p-3 text-xs text-muted"><input type="checkbox" :checked="selectedAll" data-testid="zip-organize-select-all" @change="toggleAll" />选择本页<span class="ml-auto">已选 {{ picked.length }} 项</span></label>
          <ul class="max-h-72 overflow-y-auto custom-scrollbar">
            <li v-for="node in visibleNodes" :key="node.id" class="flex min-w-0 items-start gap-2 border-b border-subtle/40 p-3 last:border-0" :data-organize-path="node.path">
              <input type="checkbox" class="mt-1 shrink-0" :checked="selected.has(node.id)" :aria-label="`选择 ${node.path}`" :disabled="locked" @change="toggle(node.id)" />
              <button class="min-w-0 flex-1 text-left" :aria-disabled="!node.isDir" @click="node.isDir ? (directory=node.path,query='') : toggle(node.id)" @dblclick="directory=node.isDir?node.path:directory"><span class="block break-words text-xs leading-relaxed [overflow-wrap:anywhere]"><i :class="node.isDir?'pi pi-folder text-primary':'pi pi-file text-muted'" class="mr-2"></i>{{ node.path }}</span><small class="mt-1 block text-muted">{{ node.isDir?'目录 · 连同子项整理':formatFileSize(node.size) }}{{ node.input?' · 新增/替换内容':'' }}</small></button>
            </li>
            <li v-if="!visibleNodes.length" class="p-5 text-xs text-muted">{{ query?'没有匹配条目':'当前目录为空' }}</li>
          </ul>
        </div>
        <div v-if="pageCount>1" class="flex justify-between text-xs"><button :disabled="page<=1" @click="page--">上一页</button><span>{{ page }} / {{ pageCount }}</span><button :disabled="page>=pageCount" @click="page++">下一页</button></div>
      </section>
      <section class="min-w-0 space-y-3" data-testid="zip-organize-rules">
        <div class="flex flex-wrap gap-2"><button class="organize-button" :class="mode==='rename'?'text-primary border-primary/40':''" data-testid="zip-organize-mode-rename" @click="mode='rename'">重命名 / 移动</button><button class="organize-button" :class="mode==='delete'?'text-red-500 border-red-500/40':''" data-testid="zip-organize-mode-delete" @click="mode='delete'">删除所选</button></div>
        <p v-if="!picked.length" class="rounded-xl border border-dashed border-subtle p-4 text-xs leading-relaxed text-muted">先在左侧选择文件或目录。可以多选；选择目录会连同子项一起整理。</p>
        <template v-else>
          <p class="text-xs text-muted">已选 {{ picked.length }} 项 · {{ roots.length }} 个独立位置{{ picked.length>roots.length?'（子项随目录处理）':'' }}</p>
          <fieldset v-if="mode==='rename'" :disabled="locked" class="space-y-3 rounded-xl border border-subtle bg-input/20 p-3">
            <label v-if="roots.length===1" class="block text-xs text-muted">新包内路径<input v-model="singlePath" class="organize-input mt-2 w-full" data-testid="zip-organize-single-path" /><span class="mt-2 block leading-relaxed">修改名称，或填写“目录/名称”移动位置；原有子项跟随目录。</span></label>
            <template v-else>
              <div class="grid gap-3 sm:grid-cols-2"><label class="text-xs text-muted">名称前缀<input v-model="rule.prefix" class="organize-input mt-2 w-full" data-testid="zip-organize-prefix" /></label><label class="text-xs text-muted">名称后缀<input v-model="rule.suffix" class="organize-input mt-2 w-full" data-testid="zip-organize-suffix" /></label></div>
              <div class="grid gap-3 sm:grid-cols-2"><label class="text-xs text-muted">替换名称中的文字<input v-model="rule.search" class="organize-input mt-2 w-full" data-testid="zip-organize-find" /></label><label class="text-xs text-muted">替换为<input v-model="rule.replacement" class="organize-input mt-2 w-full" data-testid="zip-organize-replacement" /></label></div>
              <label class="block text-xs text-muted">移入包内目录（可选）<input v-model="rule.destination" class="organize-input mt-2 w-full" data-testid="zip-organize-destination" placeholder="留空时保留各自目录" /></label><span class="context-help"><HelpHint label="批量改名说明">批量规则处理名称，文件扩展名保留；目录连同子项移动，不覆盖已有内容。</HelpHint></span>
            </template>
          </fieldset>
          <p v-else class="rounded-xl bg-red-500/5 p-3 text-xs leading-relaxed text-red-500">所选位置及其子项将从新 ZIP 移除。当前只加入清单，保存前仍可撤销。</p>
          <div class="rounded-xl border border-subtle p-3" data-testid="zip-organize-preview"><strong class="text-xs">{{ mode==='delete'?'移除范围':'修改前 → 修改后' }} · 影响 {{ preview.affected.length }} 项</strong><ol class="mt-3 max-h-56 space-y-3 overflow-y-auto custom-scrollbar"><li v-for="row in preview.affected.slice(0,100)" :key="row.before" class="min-w-0 text-xs leading-relaxed"><span class="block break-words text-muted [overflow-wrap:anywhere]">{{ row.before }}</span><span class="block break-words [overflow-wrap:anywhere]" :class="row.after===null?'text-red-500':'text-primary'">→ {{ row.after??'从新包移除' }}</span></li></ol><p v-if="preview.affected.length>100" class="mt-2 text-xs text-muted">前 100 项预览，实际将处理全部 {{ preview.affected.length }} 项。</p></div>
          <p v-if="preview.issue" role="alert" class="break-words rounded-xl bg-amber-500/10 p-3 text-xs text-amber-600 [overflow-wrap:anywhere]" data-testid="zip-organize-issue">{{ preview.issue }}</p>
          <button class="organize-button bg-primary text-white" :disabled="locked||!!preview.issue" data-testid="zip-organize-apply" @click="apply">加入待保存清单</button>
        </template>
        <p v-if="notice" role="status" class="rounded-xl bg-primary/5 p-3 text-xs leading-relaxed text-primary" data-testid="zip-organize-notice">{{ notice }}</p>
      </section>
    </div>
    <template #footer><div class="flex w-full flex-wrap items-center justify-end gap-2"><button class="organize-button" @click="emit('update:visible',false)">返回原包浏览</button><button class="organize-button text-primary" data-testid="zip-organize-review" @click="emit('update:visible',false);emit('review')">复核并另存{{ plan?.changeCount?`（${plan.changeCount} 项）`:'' }}</button></div></template>
  </Modal>
</template>
<style scoped>
.organize-button{display:inline-flex;align-items:center;justify-content:center;min-height:2rem;border:1px solid var(--border-subtle);border-radius:.65rem;padding:.45rem .7rem;font-size:.75rem;transition:background .15s,color .15s}
.organize-input{min-width:0;border:1px solid var(--border-subtle);border-radius:.65rem;background:var(--bg-input);padding:.55rem .65rem;font-size:.75rem}
button:disabled{opacity:.45;cursor:default}button:focus-visible,input:focus-visible{outline:2px solid var(--dynamic-accent);outline-offset:2px}input[type=checkbox]{accent-color:var(--dynamic-accent)}
</style>
