import type { ZpaqSnapshot } from './zpaqSnapshot'
export type DiffKind='added'|'changed'|'candidate'|'metadata'|'removed'|'unchanged'
export interface DiffSide {size:number;date:number;directory:boolean;selectable:boolean}
export interface ZpaqChange {path:string;kind:DiffKind;reason:string;before:DiffSide|null;after:DiffSide|null;verified:boolean}
export interface ZpaqComparison {before:ZpaqSnapshot;after:ZpaqSnapshot;beforeCatalogId:string;afterCatalogId:string;revision:string;entries:ZpaqChange[];counts:Partial<Record<DiffKind,number>>}
export const diffLabels:Record<DiffKind,string>={added:'新增',changed:'变化',candidate:'内容候选',metadata:'元数据',removed:'移除',unchanged:'未变'}
export const selectionKey=(side:'before'|'after',path:string)=>JSON.stringify([side,path])
export function defaultDiffSelection(result:ZpaqComparison){
  return new Set(result.entries.filter(e=>e.kind!=='unchanged').flatMap(e=>{
    const side=e.kind==='removed'?'before':e.after?.selectable?'after':'before'
    return e[side]?.selectable?[selectionKey(side,e.path)]:[]
  }))
}
export function selectedDiffGroups(result:ZpaqComparison,selected:Set<string>){
  return (['before','after'] as const).flatMap(side=>{
    const paths=result.entries.filter(e=>e[side]?.selectable&&selected.has(selectionKey(side,e.path))).map(e=>e.path)
    return paths.length?[{side,snapshot:result[side],catalogId:side==='before'?result.beforeCatalogId:result.afterCatalogId,paths}]:[]
  })
}
export function mergeVerifiedComparison(old:ZpaqComparison|null,next:ZpaqComparison):ZpaqComparison {
  if (!old || old.revision!==next.revision) return next
  const verified=new Map(old.entries.filter(e=>e.verified).map(e=>[e.path,e]))
  const entries=next.entries.map(e=>!e.verified&&verified.has(e.path)?verified.get(e.path)!:e)
  const counts:ZpaqComparison['counts']={};for(const e of entries)counts[e.kind]=(counts[e.kind]||0)+1
  return {...next,entries,counts}
}
