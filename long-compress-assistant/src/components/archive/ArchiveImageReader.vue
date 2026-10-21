<script setup lang="ts">
import {computed,nextTick,onMounted,onUnmounted,ref,watch} from 'vue'
import {invoke} from '@tauri-apps/api/tauri'
import {listen,type UnlistenFn} from '@tauri-apps/api/event'
import {readerImages,useArchiveImageReader,type ReaderImage,type ReaderProgress} from '@/composables/useArchiveImageReader'
import type {ArchiveEntryInfo,ArchiveImagePreview} from '@/composables/useTauriCommands'
import type {ZipPasswordEncoding} from '@/types/zipPassword'
import type {ZpaqSnapshot} from '@/types/zpaqSnapshot'
const props=defineProps<{filePath:string;entries:ArchiveEntryInfo[];initialPath:string;directory:string;revision?:string;password:string;encoding?:ZipPasswordEncoding;snapshot?:ZpaqSnapshot;supported:boolean;imageExtensions:string[]}>()
const emit=defineEmits<{close:[];locate:[path:string];extract:[path:string];refresh:[]}>()
const dialog=ref<HTMLElement>(),stage=ref<HTMLElement>(),scope=ref<'directory'|'archive'>('directory'),path=ref(props.initialPath),pageInput=ref(1),zoom=ref(1),fit=ref(true),ready=ref(false)
const extensions=new Set(['png','jpg','jpeg','gif','webp','bmp','svg','ico','avif'])
const extension=(name:string)=>name.split('.').pop()?.toLowerCase()||''
const parent=(path:string)=>{const p=path.replace(/\\/g,'/');return p.slice(0,Math.max(0,p.lastIndexOf('/')))}
const all=computed(()=>readerImages(props.entries,name=>extensions.has(extension(name))))
const images=computed(()=>scope.value==='archive'?all.value:all.value.filter(entry=>parent(entry.path)===props.directory))
const index=computed(()=>images.value.findIndex(entry=>entry.path===path.value)),entry=computed(()=>images.value[index.value])
const thumbnails=computed(()=>{const start=Math.max(0,Math.min(index.value-3,images.value.length-7));return images.value.slice(start,start+7).map((entry,offset)=>({entry,index:start+offset}))})
const reader=useArchiveImageReader({
 checkSource:()=>invoke('check_image_reader_source',{filePath:props.filePath,revision:props.revision||''}),
 cancel:id=>invoke('cancel_archive_browse',{browseId:id}),
 read:(entryPath,requestId)=>invoke<ArchiveImagePreview>('read_archive_image',{filePath:props.filePath,entryPath,requestId,revision:props.revision||'',password:props.password||null,zipPasswordEncoding:props.encoding||null,zpaqSnapshot:props.snapshot||null}),
 createUrl:async preview=>{const raw=atob(preview.dataUrl.slice(preview.dataUrl.indexOf(',')+1)),bytes=new Uint8Array(raw.length);for(let n=0;n<raw.length;n++)bytes[n]=raw.charCodeAt(n);return URL.createObjectURL(new Blob([bytes],{type:preview.mimeType}))},
 revokeUrl:url=>URL.revokeObjectURL(url),
})
const reason=(entry?:ReaderImage)=>!props.revision?'来源清单需要刷新，请返回并刷新来源后重新阅读。':!entry?'当前范围没有图片，可切换到整包。':entry.ambiguous?'这个包内路径存在重复名称，无法唯一读取，请提取整理后查看。':!props.supported?'此归档暂不支持内置图片阅读，可沿原提取功能取出后查看。':!props.imageExtensions.includes(extension(entry.name))?'此图片格式暂不支持内置阅读，可跳过或提取后查看。':undefined
async function load(){
 if(!ready.value)return;pageInput.value=index.value+1;fit.value=true;zoom.value=1
 const next=images.value[index.value+1]
 await reader.show(path.value,next&&!reason(next)?next.path:undefined,reason(entry.value))
 await nextTick();if(stage.value){stage.value.scrollTop=0;stage.value.scrollLeft=0}
}
watch(path,()=>void load())
watch(scope,()=>{if(!images.value.some(e=>e.path===path.value))path.value=images.value[0]?.path||'';else void load()})
function go(index:number){const image=images.value[Math.max(0,Math.min(index,images.value.length-1))];if(image&&image.path!==path.value)path.value=image.path}
function jump(){if(Number.isFinite(pageInput.value))go(Math.trunc(pageInput.value)-1);pageInput.value=index.value+1}
function zoomBy(amount:number){fit.value=false;zoom.value=Math.max(.25,Math.min(4,Math.round((zoom.value+amount)*100)/100))}
const progressText=computed(()=>{const p=reader.progress.value;if(p.phase==='locating')return `正在定位图片 · 已扫描 ${bytes(p.processedBytes)}`;if(p.phase==='reading')return `正在读取 · ${bytes(p.processedBytes)}${p.totalBytes?' / '+bytes(p.totalBytes):''}`;return p.phase==='decoding'?'正在显示图片…':'正在读取图片，请稍候…'})
function bytes(value:number){return value>=1048576?(value/1048576).toFixed(1)+' MiB':value>=1024?(value/1024).toFixed(1)+' KiB':value+' B'}
function keydown(event:KeyboardEvent){
 event.stopPropagation()
 if(event.key==='Escape'){event.preventDefault();emit('close');return}
 if(event.key==='Tab'){
  const nodes=Array.from(dialog.value!.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),summary')).filter(e=>e.offsetParent!==null)
  const first=nodes[0],last=nodes.at(-1);if(event.shiftKey&&(document.activeElement===first||document.activeElement===dialog.value)){event.preventDefault();last?.focus()}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first?.focus()}return
 }
 if((event.target as HTMLElement).closest('input,select,textarea,[contenteditable=true]'))return
 if(['ArrowRight','PageDown'].includes(event.key)){event.preventDefault();go(index.value+1)}
 else if(['ArrowLeft','PageUp'].includes(event.key)){event.preventDefault();go(index.value-1)}
 else if(event.key==='Home'){event.preventDefault();go(0)}else if(event.key==='End'){event.preventDefault();go(images.value.length-1)}
 else if(['+','='].includes(event.key)){event.preventDefault();zoomBy(.25)}else if(event.key==='-'){event.preventDefault();zoomBy(-.25)}else if(event.key==='0'){event.preventDefault();fit.value=true}
}
let stop:UnlistenFn|undefined,disposed=false
onMounted(async()=>{
 if(!all.value.some(e=>e.path===props.initialPath&&parent(e.path)===props.directory))scope.value='archive'
 try{const unlisten=await listen<{requestId:string;progress:ReaderProgress}>('image-reader-progress',event=>reader.update(event.payload.requestId,event.payload.progress));if(disposed){unlisten();return}stop=unlisten}catch{ /* Reading itself remains usable without byte events. */ }
 ready.value=true;dialog.value?.focus();await load()
})
onUnmounted(()=>{disposed=true;stop?.();reader.dispose()})
</script>
<template>
 <Teleport to="body"><div class="reader-backdrop" data-testid="archive-image-reader" @keydown="keydown">
  <section ref="dialog" class="image-reader" role="dialog" aria-modal="true" aria-label="归档图片阅读" tabindex="-1">
   <header class="reader-header"><div class="reader-title"><span>归档图片阅读 · {{ filePath.replace(/\\/g,'/').split('/').pop() }}</span><h2 data-testid="reader-current-path">{{ entry?.path||'暂无图片' }}</h2></div><button class="reader-button icon" aria-label="关闭图片阅读" data-testid="reader-close" @click="emit('close')"><i class="pi pi-times"/></button></header>
   <div class="reader-toolbar" data-testid="reader-toolbar">
    <label class="reader-scope">范围<select v-model="scope" data-testid="reader-scope"><option value="directory">当前目录</option><option value="archive">整包图片</option></select></label>
    <div class="reader-controls"><button class="reader-button icon" data-testid="reader-previous" :disabled="index<=0" aria-label="上一张" @click="go(index-1)"><i class="pi pi-angle-left"/></button><label class="page-counter"><input v-model.number="pageInput" type="number" min="1" :max="images.length" aria-label="跳到第几张" data-testid="reader-page" @change="jump" @keyup.enter="jump"><span data-testid="reader-total">/ {{ images.length }}</span></label><button class="reader-button icon" data-testid="reader-next" :disabled="index>=images.length-1" aria-label="下一张" @click="go(index+1)"><i class="pi pi-angle-right"/></button></div>
    <div class="reader-controls zoom-controls"><button class="reader-button" :aria-pressed="fit" data-testid="reader-fit" @click="fit=true">适应窗口</button><button class="reader-button" :aria-pressed="!fit&&zoom===1" data-testid="reader-original" @click="fit=false;zoom=1">原始大小</button><button class="reader-button icon" :disabled="!fit&&zoom<=.25" aria-label="缩小" data-testid="reader-zoom-out" @click="zoomBy(-.25)">−</button><span class="zoom-label" data-testid="reader-zoom">{{ fit?'适应':Math.round(zoom*100)+'%' }}</span><button class="reader-button icon" :disabled="!fit&&zoom>=4" aria-label="放大" data-testid="reader-zoom-in" @click="zoomBy(.25)">+</button></div>
   </div>
   <main ref="stage" class="reader-stage custom-scrollbar" :class="{fit}" data-testid="reader-stage" :aria-busy="reader.loading.value">
    <div v-if="reader.loading.value" class="reader-message" role="status" data-testid="reader-progress"><i class="pi pi-spin pi-spinner"/><h3>{{ progressText }}</h3><progress v-if="reader.progress.value.phase==='reading'&&reader.progress.value.totalBytes" :value="reader.progress.value.processedBytes" :max="reader.progress.value.totalBytes"/><button class="reader-button" data-testid="reader-stop" @click="reader.cancel">停止读取</button></div>
    <div v-else-if="reader.error.value||!entry" class="reader-message" role="status" data-testid="reader-error"><i class="pi pi-image"/><h3>这张图片暂时无法显示</h3><p>{{ reader.error.value||'当前范围没有图片，可切换到整包。' }}</p><div class="reader-controls"><button v-if="!reader.stale.value&&props.revision" class="reader-button" data-testid="reader-retry" @click="reader.forget(path);load()">重试</button><button v-if="index<images.length-1" class="reader-button" data-testid="reader-skip" @click="go(index+1)">跳过，查看下一张</button><button v-if="reader.stale.value||!props.revision" class="reader-button" data-testid="reader-refresh" @click="emit('refresh')">返回并刷新来源</button></div></div>
    <div v-else-if="reader.current.value" class="reader-canvas"><img :src="reader.current.value.url" :alt="entry?.name" data-testid="reader-image" :style="fit?{}:{width:reader.current.value.width*zoom+'px',height:reader.current.value.height*zoom+'px'}" draggable="false" @error="reader.displayError"/></div>
   </main>
   <div class="reader-thumbnails custom-scrollbar" data-testid="reader-thumbnails" aria-label="相邻图片"><button v-for="item in thumbnails" :key="item.entry.path" class="reader-thumbnail" :class="{active:item.index===index}" :aria-current="item.index===index?'page':undefined" :data-reader-thumbnail="item.entry.path" :title="item.entry.path" @click="go(item.index)"><img v-if="reader.cache.get(item.entry.path)" :src="reader.cache.get(item.entry.path)!.url" alt="" loading="lazy"/><i v-else class="pi pi-image"/><span>{{ item.index+1 }} · {{ item.entry.name }}</span></button></div>
   <footer class="reader-footer"><div class="reader-info"><span v-if="reader.current.value">{{ reader.current.value.width }} × {{ reader.current.value.height }} · {{ bytes(reader.current.value.byteSize) }}</span><span>← → 翻页 · + − 缩放 · Esc 返回</span><details><summary>阅读范围</summary><p>PNG、JPEG、GIF、WebP、BMP，沿现有归档读取能力。每张最大 8 MiB、单边 8192、1600 万像素，TAR 最多扫描 64 MiB；受限内容可提取后查看。仅少量相邻缓存，关闭释放，原包保留。</p></details></div><div class="reader-controls"><button class="reader-button" :disabled="!entry" data-testid="reader-locate" @click="entry&&emit('locate',entry.path)">在列表定位</button><button class="reader-button accent" :disabled="!entry||reader.stale.value||!props.revision" data-testid="reader-extract" @click="entry&&emit('extract',entry.path)"><i class="pi pi-download"/>提取这张</button></div></footer>
  </section>
 </div></Teleport>
</template>
<style scoped>
.reader-backdrop{position:fixed;inset:0;z-index:1200;display:grid;place-items:center;padding:22px;background:#090f1ce0;backdrop-filter:blur(12px)}.image-reader{display:flex;flex-direction:column;min-width:0;min-height:0;width:min(1440px,100%);height:100%;max-height:1100px;border:1px solid var(--border-subtle);border-radius:20px;background:var(--bg-card);color:var(--text-base);overflow:hidden;box-shadow:0 24px 100px #0008;outline:none}.reader-header{display:flex;align-items:center;gap:16px;padding:16px 20px;border-bottom:1px solid var(--border-subtle);flex-shrink:0}.reader-title{flex:1;min-width:0}.reader-title>span{font-size:10px;color:var(--dynamic-accent);font-weight:800;overflow-wrap:anywhere}.reader-title h2{font-size:15px;font-weight:850;line-height:1.5;margin-top:5px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.reader-toolbar{display:flex;gap:12px;align-items:center;flex-wrap:wrap;padding:10px 20px;border-bottom:1px solid var(--border-subtle);flex-shrink:0}.reader-button{display:inline-flex;align-items:center;justify-content:center;gap:6px;border:1px solid var(--border-subtle);border-radius:9px;padding:8px 10px;font-size:11px;background:var(--bg-input);white-space:nowrap;min-height:32px}.reader-button.icon{min-width:32px;padding:7px}.reader-button.accent{background:var(--dynamic-accent);color:#fff;border-color:transparent}.reader-button[aria-pressed=true]{color:var(--dynamic-accent);border-color:var(--dynamic-accent)}button:disabled{opacity:.4;cursor:default}button:focus-visible,input:focus-visible,select:focus-visible,summary:focus-visible{outline:2px solid var(--dynamic-accent);outline-offset:2px}.reader-controls{display:flex;gap:7px;align-items:center;flex-wrap:wrap}.zoom-controls{margin-left:auto}.reader-scope{display:flex;gap:8px;align-items:center;font-size:10px;color:var(--text-muted)}select,.page-counter input{min-width:0;background:var(--bg-input);border:1px solid var(--border-subtle);border-radius:8px;font-size:11px;color:var(--text-base);padding:7px}.page-counter{display:flex;align-items:center;gap:6px;font-size:11px;color:var(--text-muted)}.page-counter input{width:58px;text-align:center}.zoom-label{min-width:33px;text-align:center;font-size:10px;color:var(--text-muted)}.reader-stage{flex:1;min-height:100px;min-width:0;overflow:auto;position:relative;background:repeating-conic-gradient(#141b28 0% 25%,#171f2e 0% 50%) 50%/20px 20px}.reader-canvas{min-width:100%;min-height:100%;width:max-content;display:flex;align-items:center;justify-content:center;padding:14px}.reader-canvas img{display:block;max-width:none;max-height:none;flex-shrink:0;object-fit:contain;box-shadow:0 6px 35px #0006}.fit .reader-canvas{position:absolute;inset:0;width:100%;height:100%;min-height:0;min-width:0}.fit .reader-canvas img{max-width:100%;max-height:100%;width:auto;height:auto}.reader-message{display:flex;flex-direction:column;align-items:center;justify-content:center;min-height:100%;width:min(640px,100%);margin:0 auto;padding:22px;gap:14px;text-align:center;color:#cbd5e1;font-size:12px;overflow-wrap:anywhere}.reader-message>i{font-size:28px;color:var(--dynamic-accent)}.reader-message h3{font-size:14px;color:#f1f5f9;font-weight:800}.reader-message p{font-size:11px;line-height:1.8}.reader-message .reader-button{color:var(--text-base)}.reader-message progress{width:min(280px,100%);height:5px;accent-color:var(--dynamic-accent)}.reader-thumbnails{display:flex;gap:8px;overflow-x:auto;flex-shrink:0;padding:10px 18px;border-top:1px solid var(--border-subtle)}.reader-thumbnail{width:96px;min-width:68px;flex:0 0 96px;border:1px solid var(--border-subtle);border-radius:9px;padding:5px;background:var(--bg-input)}.reader-thumbnail.active{border-color:var(--dynamic-accent);box-shadow:inset 0 0 0 1px var(--dynamic-accent)}.reader-thumbnail>img,.reader-thumbnail>i{display:block;height:42px;width:100%;object-fit:contain;border-radius:4px}.reader-thumbnail>i{padding-top:10px;font-size:22px;color:var(--text-muted)}.reader-thumbnail span{display:block;margin-top:5px;font-size:9px;text-overflow:ellipsis;overflow:hidden;white-space:nowrap;color:var(--text-muted)}.reader-footer{display:flex;gap:12px;align-items:center;justify-content:space-between;flex-wrap:wrap;padding:9px 18px;border-top:1px solid var(--border-subtle);flex-shrink:0}.reader-info{display:flex;gap:10px;align-items:center;flex-wrap:wrap;font-size:9px;color:var(--text-muted)}.reader-info details p{font-size:10px;line-height:1.8;max-width:520px;padding-top:8px}.reader-info summary{cursor:pointer;color:var(--dynamic-accent)}@media(max-width:800px){.reader-backdrop{padding:8px}.image-reader{border-radius:14px}.reader-header{padding:10px 12px;gap:8px}.reader-title h2{font-size:13px}.reader-toolbar{gap:7px;padding:8px 12px}.zoom-controls{margin-left:0}.reader-thumbnails{padding:7px 12px;gap:6px}.reader-thumbnail{flex-basis:72px;width:72px}.reader-thumbnail>img,.reader-thumbnail>i{height:32px}.reader-footer{padding:7px 12px;gap:6px}.reader-info>span:nth-child(2){display:none}.reader-button{font-size:10px;padding:7px 8px}.reader-canvas{padding:10px}}
</style>
