import {parentDirectory} from './resultLocation'
import { shallowRef } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
export interface ZpaqMember { path:string;size:number;stamp:string }
export interface ZpaqGroup { name:string;members:ZpaqMember[];missing:string[] }
export const pendingZpaqGroup=shallowRef<{group:ZpaqGroup;path:string}|null>(null)
const confirmed=new Map<string,string>()
let finish:((path:string|null)=>void)|undefined
let queue:Promise<unknown>=Promise.resolve()
export function finishZpaqGroup(path:string|null){const resolve=finish;finish=undefined;pendingZpaqGroup.value=null;resolve?.(path)}
export function prepareZpaqSource(path:string):Promise<string>{
 if(!/\d+\.zpaq$/i.test(path))return Promise.resolve(path)
 const work=queue.catch(()=>{}).then(async()=>{
  const group=await invoke<ZpaqGroup|null>('suggest_zpaq_multipart',{path})
  if(!group)return path
  const key=JSON.stringify(group),previous=confirmed.get(key)
  if(previous)return previous
  const result=await new Promise<string|null>(resolve=>{finish=resolve;pendingZpaqGroup.value={group,path}})
  if(!result)throw new Error('已取消添加这份 ZPAQ 来源')
  // A single-file choice must not apply to other similarly named files.
  const details=await invoke<ZpaqGroup>('zpaq_multipart_details',{path:result})
  if(details?.members.length===group.members.length)confirmed.set(key,result)
  return result
 });queue=work;return work
}

/** A composite source reference is not the user's archive directory. */
export async function archiveSourceDirectory(path:string):Promise<string>{
 if(!/\.zpaq$/i.test(path))return parentDirectory(path)
 const group=await invoke<ZpaqGroup|null>('zpaq_multipart_details',{path})
 if(!group&&/[\\/]zpaq-sources[\\/]/i.test(path))throw new Error('来源组记录已不存在，请重新选择原成员确认整组')
 return parentDirectory(group?.members[0]?.path||path)
}
