<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useSavedLocations } from '@/composables/useSavedLocations'
const props = withDefaults(defineProps<{
  paneId: string; path: string; crumbs: Array<{name:string;path:string}>; folders: number; files: number;
  canBack?: boolean; canForward?: boolean; canUp?: boolean; loading?: boolean; selectionMode?: boolean;
  archive?: boolean; toolsOpen?: boolean; filterActive?:boolean; locations?: Array<{name:string;path:string;kind:string}>;
  saved?: ReturnType<typeof useSavedLocations>;
}>(), { locations: () => [] })
const emit = defineEmits<{ back: []; forward: []; up: []; refresh: []; create: []; selection: []; tools: [event:MouseEvent]; navigate: [path:string]; breadcrumb: [index:number] }>()
const root = ref<HTMLElement>(), draft = ref(props.path), locationsOpen = ref(false)
const saved = props.saved || useSavedLocations()
watch(() => props.path, path => { draft.value = path; locationsOpen.value = false })
function submit(path = draft.value) { locationsOpen.value = false; emit('navigate', path.trim().replace(/^['"]|['"]$/g, '')) }
function dismiss(event: Event) { if (!root.value?.contains(event.target as Node)) locationsOpen.value = false }
onMounted(() => document.addEventListener('pointerdown', dismiss))
onUnmounted(() => document.removeEventListener('pointerdown', dismiss))
</script>
<template>
  <div ref="root" class="pane-chrome" :data-testid="`file-pane-navigation-${paneId}`">
    <nav class="pane-nav" :aria-label="`${paneId==='left'?'左':'右'}栏导航`">
      <button type="button" :disabled="!canBack || loading" title="后退" aria-label="后退" @click="emit('back')"><i class="pi pi-arrow-left"/></button>
      <button type="button" :disabled="!canForward || loading" title="前进" aria-label="前进" @click="emit('forward')"><i class="pi pi-arrow-right"/></button>
      <button type="button" :disabled="!canUp || loading" title="上一级" aria-label="上一级" :data-testid="archive ? 'archive-nav-up' : undefined" @click="emit('up')"><i class="pi pi-arrow-up"/></button>
      <div class="path-combobox" :class="{open:locationsOpen}" @pointerdown.stop>
        <i :class="archive ? 'pi pi-box' : 'pi pi-folder'"/>
        <input v-model="draft" type="text" autocomplete="off" spellcheck="false" title="输入或粘贴路径，按 Enter 打开" :aria-label="`${paneId==='left'?'左':'右'}栏路径`" :data-testid="`file-manager-path-input-${paneId}`" :aria-expanded="locationsOpen" aria-haspopup="listbox" @focus="locationsOpen=true" @keydown.enter.prevent="submit()" @keydown.esc.prevent="draft=path;locationsOpen=false"/>
        <button type="button" class="location-toggle" :aria-label="locationsOpen?'收起常用位置':'展开常用位置'" @click="locationsOpen=!locationsOpen"><i class="pi pi-chevron-down"/></button>
        <div v-if="locationsOpen" class="location-menu custom-scrollbar" role="listbox" :aria-label="`${paneId==='left'?'左':'右'}栏常用位置`">
          <strong>书签</strong><button v-for="location in saved.bookmarks.value" :key="location" role="option" :aria-selected="location===path" :data-testid="`saved-location-${paneId}`" @click="submit(location)"><i class="pi pi-star-fill"/><span>{{ location }}</span></button>
          <strong>最近使用</strong><button v-for="location in saved.recent.value" :key="location" role="option" :aria-selected="location===path" @click="submit(location)"><i class="pi pi-history"/><span>{{ location }}</span></button>
          <strong>常用位置</strong><button v-for="location in locations" :key="location.path" role="option" :aria-selected="location.path===path" @click="submit(location.path)"><i :class="location.kind==='drive'?'pi pi-database':'pi pi-folder'"/><span><b>{{ location.name }}</b><small>{{ location.path }}</small></span></button>
        </div>
      </div>
      <button type="button" :disabled="!path || archive" :title="archive?'可在资料库收藏此压缩包':saved.isBookmarked(path)?'取消收藏当前目录':'收藏当前目录'" :aria-pressed="!archive && saved.isBookmarked(path)" :data-testid="`bookmark-current-${paneId}`" @click="saved.toggleBookmark(path)"><i :class="!archive && saved.isBookmarked(path)?'pi pi-star-fill':'pi pi-star'"/></button>
      <button type="button" :disabled="loading" title="刷新" aria-label="刷新" @click="emit('refresh')"><i :class="loading?'pi pi-spin pi-spinner':'pi pi-refresh'"/></button>
      <button type="button" :disabled="archive" :title="archive?'整理包内内容请使用选项中的编辑功能':'新建文件夹'" aria-label="新建文件夹" @click="emit('create')"><i class="pi pi-plus"/></button>
      <button type="button" class="selection-mode-button" :class="{active:selectionMode}" :title="selectionMode?'退出多选模式（Esc）':'进入多选模式'" :aria-pressed="!!selectionMode" :data-testid="`file-manager-selection-mode-${paneId}`" @click="emit('selection')"><i :class="selectionMode?'pi pi-times':'pi pi-check-square'"/></button>
      <button type="button" :title="filterActive?'筛选已启用，点击调整':archive?'搜索、密码与压缩包选项':'文件夹操作'" :aria-label="filterActive?'压缩包选项，筛选已启用':archive?'压缩包选项':'文件夹操作'" :data-testid="archive?'archive-pane-tools':`file-pane-tools-${paneId}`" :aria-expanded="!!toolsOpen" :class="{active:toolsOpen||filterActive}" @click="locationsOpen=false;emit('tools',$event)"><i class="pi pi-sliders-h"/><span v-if="filterActive" class="filter-dot" aria-hidden="true"/></button>
    </nav>
    <div class="path-strip" :title="path"><i :class="archive?'pi pi-box':'pi pi-folder-open'"/><nav class="path-breadcrumbs" :aria-label="`${paneId==='left'?'左':'右'}栏路径`" :data-testid="`file-manager-breadcrumbs-${paneId}`"><template v-for="(crumb,index) in crumbs" :key="`${index}-${crumb.path}`"><i v-if="index" class="pi pi-angle-right"/><button type="button" :class="{current:index===crumbs.length-1}" :title="crumb.path" @click="emit('breadcrumb',index)">{{ crumb.name }}</button></template></nav><span>{{ folders }} 文件夹 · {{ files }} 文件</span></div>
  </div>
</template>
<style scoped>
.pane-chrome{flex-shrink:0;min-width:0}.pane-nav{display:flex;align-items:center;gap:.3rem;padding:.55rem;border-bottom:1px solid var(--border-subtle);min-width:0}.pane-nav>button{display:grid;place-items:center;flex:0 0 2.1rem;width:2.1rem;height:2.1rem;border-radius:.6rem;color:var(--text-muted)}.pane-nav>button:hover:not(:disabled),.pane-nav>button.active{background:var(--bg-input);color:var(--dynamic-accent)}.pane-nav>button:disabled{opacity:.28}.pane-nav>button:focus-visible{outline:2px solid var(--dynamic-accent);outline-offset:-2px}.path-combobox{position:relative;display:flex;align-items:center;flex:1;gap:.45rem;min-width:0;height:2.1rem;border:1px solid var(--border-subtle);border-radius:.6rem;background:var(--bg-input);padding:0 .6rem}.path-combobox>i{color:var(--dynamic-accent);flex-shrink:0;font-size:.8rem}.path-combobox input{min-width:0;flex:1;width:100%;background:transparent;font-size:.68rem;font-weight:700;color:var(--text-base);outline:none}.location-toggle{flex-shrink:0;color:var(--text-muted);font-size:.6rem}.path-combobox:focus-within{border-color:var(--dynamic-accent)}.location-menu{position:absolute;z-index:50;left:0;top:calc(100% + 6px);width:max(100%,16rem);max-width:min(23rem,calc(100vw - 3rem));max-height:18rem;overflow:auto;padding:6px;border:1px solid var(--border-subtle);border-radius:10px;background:var(--bg-modal);box-shadow:0 12px 28px #0003}.location-menu>strong{display:block;padding:6px;font-size:10px;color:var(--text-muted)}.location-menu>button{display:flex;align-items:center;gap:8px;width:100%;padding:8px;border-radius:7px;font-size:11px;text-align:left;color:var(--text-base)}.location-menu>button:hover{background:var(--bg-input);color:var(--dynamic-accent)}.location-menu>button>i{color:var(--dynamic-accent);flex-shrink:0}.location-menu span{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.location-menu b,.location-menu small{display:block;overflow:hidden;text-overflow:ellipsis}.location-menu small{font-size:10px;color:var(--text-muted)}.path-strip{display:grid;grid-template-columns:auto minmax(0,1fr) auto;align-items:center;gap:.45rem;padding:.42rem .75rem;border-bottom:1px solid var(--border-subtle);color:var(--text-muted);font-size:.64rem}.path-strip>i{color:var(--dynamic-accent)}.path-strip>span{white-space:nowrap}.path-breadcrumbs{display:flex;min-width:0;align-items:center;gap:.1rem;overflow-x:auto;scrollbar-width:none}.path-breadcrumbs::-webkit-scrollbar{display:none}.path-breadcrumbs>i{font-size:.5rem;flex-shrink:0}.path-breadcrumbs button{flex-shrink:0;max-width:9rem;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;border-radius:.45rem;padding:.28rem .38rem;font-size:.63rem;font-weight:800}.path-breadcrumbs button.current,.path-breadcrumbs button:hover{color:var(--dynamic-accent);background:color-mix(in srgb,var(--dynamic-accent) 9%,transparent)}
@media(max-width:1100px){.pane-nav{flex-wrap:wrap}.pane-nav>.path-combobox{order:2;flex:1 0 100%}}@media(max-width:900px){.pane-nav>button{flex-basis:1.85rem;width:1.85rem;height:1.85rem}.path-combobox{height:1.85rem}}@container file-pane (max-width:360px){.path-strip{grid-template-columns:auto minmax(0,1fr)}.path-strip>span{display:none}}
</style>
<style scoped>.pane-nav>button{position:relative}.filter-dot{position:absolute;right:3px;top:3px;width:5px;height:5px;border-radius:50%;background:var(--dynamic-accent)}</style>
