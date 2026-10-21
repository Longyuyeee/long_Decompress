import {ref,shallowReactive,shallowRef} from 'vue'
import type {ArchiveEntryInfo,ArchiveImagePreview} from './useTauriCommands'
export interface ReaderImage extends ArchiveEntryInfo {ambiguous:boolean}
export function readerImages(entries:ArchiveEntryInfo[],isImage:(name:string)=>boolean):ReaderImage[]{
 const map=new Map<string,ReaderImage>()
 for(const entry of entries)if(!entry.isDir&&isImage(entry.name)){const old=map.get(entry.path);if(old)old.ambiguous=true;else map.set(entry.path,{...entry,ambiguous:false})}
 return [...map.values()].sort((a,b)=>a.path.localeCompare(b.path,'zh-CN',{numeric:true,sensitivity:'base'})||a.path.localeCompare(b.path))
}
export function readerError(cause:unknown){
 const raw=String(cause).replace(/^Error:\s*/i,'')
 if(raw.includes('IMAGE_READER_CHANGED|'))return raw.split('IMAGE_READER_CHANGED|').at(-1)!
 if(raw.includes('IMAGE_READER_CANCELLED|'))return '本次读取已停止，可以重试或翻到下一张。'
 if(/password is incorrect|Invalid password|密码不正确/i.test(raw))return '密码不正确，请返回列表修改来源密码后重新阅读。'
 if(/password required|Password required/i.test(raw))return '此图片需要密码，请返回列表填写来源密码后重新阅读。'
 if(/Image preview is limited/.test(raw))return '此图片解压后超过 8 MiB，请沿原提取功能取出后查看。'
 if(/Image dimensions exceed/.test(raw))return '此图片尺寸超过内置阅读范围（单边 8192、1600 万像素），请提取后查看。'
 if(/bounded scan budget/.test(raw))return '定位此图片需扫描超过 64 MiB，请提取后查看。'
 if(/not a supported raster image/.test(raw))return '内容不是支持的 PNG、JPEG、GIF、WebP 或 BMP 图片，可跳过或提取后查看。'
 return raw
}
export interface ReaderAsset {path:string;url:string;width:number;height:number;byteSize:number;mimeType:string}
export interface ReaderProgress {phase:string;processedBytes:number;totalBytes:number}
interface Client {
 read:(path:string,id:string)=>Promise<ArchiveImagePreview>;checkSource:()=>Promise<void>;cancel:(id:string)=>Promise<unknown>
 createUrl:(preview:ArchiveImagePreview)=>Promise<string>;revokeUrl:(url:string)=>void
}
export function useArchiveImageReader(client:Client){
 const cache=shallowReactive(new Map<string,ReaderAsset>()),current=shallowRef<ReaderAsset|null>(null),currentPath=ref(''),loading=ref(false),error=ref(''),stale=ref(false),activeRequest=ref(''),progress=ref<ReaderProgress>({phase:'opening',processedBytes:0,totalBytes:0})
 const pending=new Map<string,{id:string;cancelled:boolean;promise:Promise<ReaderAsset|null>}>()
 let sequence=0,disposed=false
 const forget=(path:string)=>{const old=cache.get(path);if(old){client.revokeUrl(old.url);cache.delete(path)}}
 const stopRequests=(keep:string[] = [])=>{for(const[path,job]of pending)if(!keep.includes(path)){job.cancelled=true;pending.delete(path);void client.cancel(job.id).catch(()=>undefined)}}
 function invalidate(cause:unknown){stale.value=true;error.value=readerError(cause);current.value=null;stopRequests();for(const path of [...cache.keys()])forget(path)}
 function trim(){
  const over=()=>cache.size>3||[...cache.values()].reduce((sum,a)=>sum+a.byteSize,0)>16*1024*1024||[...cache.values()].reduce((sum,a)=>sum+a.width*a.height,0)>24_000_000
  for(const path of [...cache.keys()])if(over()&&path!==currentPath.value)forget(path)
 }
 async function asset(path:string):Promise<ReaderAsset|null>{
  const old=cache.get(path);if(old){await client.checkSource();if(disposed||cache.get(path)!==old)return null;cache.delete(path);cache.set(path,old);return old}
  const prior=pending.get(path);if(prior)return prior.promise
  const job={id:crypto.randomUUID(),cancelled:false,promise:Promise.resolve(null) as Promise<ReaderAsset|null>}
  job.promise=(async()=>{
   const preview=await client.read(path,job.id)
   if(disposed||job.cancelled)return null
   const url=await client.createUrl(preview)
   if(disposed||job.cancelled){client.revokeUrl(url);return null}
   const value={path,url,width:preview.width,height:preview.height,byteSize:preview.byteSize,mimeType:preview.mimeType}
   forget(path);cache.set(path,value);trim();return cache.has(path)?value:null
  })().finally(()=>{if(pending.get(path)===job)pending.delete(path)})
  pending.set(path,job);return job.promise
 }
 async function show(path:string,next?:string,reason?:string){
  if(disposed)return
  const token=++sequence;currentPath.value=path;current.value=null;error.value='';loading.value=true;activeRequest.value='';progress.value={phase:'opening',processedBytes:0,totalBytes:0}
  stopRequests([path])
  if(stale.value){loading.value=false;error.value='来源已变化，请返回刷新来源后重新阅读。';return}
  if(reason){loading.value=false;error.value=reason;return}
  try{
   const request=asset(path);activeRequest.value=pending.get(path)?.id||''
   const value=await request
   if(token!==sequence||disposed)return
   current.value=value
   if(value&&next&&!cache.has(next))void asset(next).catch(cause=>{if(!disposed&&String(cause).includes('IMAGE_READER_CHANGED|'))invalidate(cause)})
  }catch(cause){if(token===sequence&&!disposed){if(String(cause).includes('IMAGE_READER_CHANGED|'))invalidate(cause);else error.value=readerError(cause)}}
  finally{if(token===sequence&&!disposed){loading.value=false;activeRequest.value=''}}
 }
 function cancel(){sequence++;stopRequests();loading.value=false;activeRequest.value='';error.value='本次读取已停止，可以重试或翻到下一张。'}
 function displayError(){forget(currentPath.value);current.value=null;error.value='图片数据无法完整显示，可重试、跳过或提取后查看。'}
 function update(id:string,value:ReaderProgress){if(!disposed&&loading.value&&id===activeRequest.value)progress.value=value}
 function dispose(){disposed=true;sequence++;stopRequests();for(const path of [...cache.keys()])forget(path);current.value=null}
 return {cache,current,currentPath,loading,error,stale,progress,show,forget,cancel,displayError,update,dispose}
}
