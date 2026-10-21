import {isZipPasswordEncoding,type ZipPasswordEncoding} from './zipPassword'
export type ComparisonSide='left'|'right'
export interface ComparisonSourceRef {path:string;kind:'directory'|'zip';revision:string;encrypted?:boolean}
export interface ComparisonEntry {path:string;directory:boolean;size:number;modified:string|null;selectable:boolean;encrypted?:boolean}
export interface ComparisonSource {source:ComparisonSourceRef;name:string;entries:ComparisonEntry[]}
export interface ComparisonSelection {source:ComparisonSourceRef;paths:string[];keepStructure:boolean;conflictPolicy:'rename'|'skip';zipPasswordEncoding?:ZipPasswordEncoding}
export type ComparisonKind='left_only'|'right_only'|'type'|'size'|'time'|'candidate'|'directory'|'same'|'different'|'metadata'|'unreadable'
export interface ComparisonRow {path:string;kind:ComparisonKind;reason:string;left?:ComparisonEntry;right?:ComparisonEntry}
export const comparisonLabels:Record<ComparisonKind,string>={left_only:'仅左侧',right_only:'仅右侧',type:'类型变化',size:'大小变化',time:'时间变化',candidate:'内容未核对',directory:'两侧目录',same:'内容相同',different:'内容不同',metadata:'内容相同 · 时间变化',unreadable:'核对未完成'}
export function compareEntries(left:ComparisonEntry[],right:ComparisonEntry[]):ComparisonRow[]{
 const a=new Map(left.map(e=>[e.path,e])),b=new Map(right.map(e=>[e.path,e]))
 return [...new Set([...a.keys(),...b.keys()])].sort((a,b)=>a.localeCompare(b,'zh-CN',{numeric:true})).map(path=>{
  const l=a.get(path),r=b.get(path)
  const [kind,reason]:[ComparisonKind,string]=!l?['right_only','仅右侧存在，可从右侧取回']:!r?['left_only','仅左侧存在，可从左侧取回']:l.directory!==r.directory?['type','一侧为文件，另一侧为文件夹；请分别选择可取回条目']:l.directory?['directory','两侧均有目录；内容差异请查看其子项']:l.size!==r.size?['size','文件大小不同']:l.modified&&r.modified&&l.modified!==r.modified?['time','记录时间不同；内容仍未核对']:['candidate',l.modified&&r.modified?'大小与记录时间相同；尚未核对实际内容':'大小相同，缺少可比时间；尚未核对实际内容']
  return {path,kind,reason,left:l,right:r}
 })
}
export function copyComparisonSelection(value:ComparisonSelection):ComparisonSelection|null{
 if(!value||!value.source||!['directory','zip'].includes(value.source.kind)||typeof value.source.path!=='string'||!value.source.path||value.source.path.length>32768||typeof value.source.revision!=='string'||!/^[a-f0-9]{64}$/i.test(value.source.revision)||!Array.isArray(value.paths)||!value.paths.length||value.paths.length>2000||JSON.stringify(value.paths).length>60000||!value.paths.every(p=>typeof p==='string'&&p.length>0&&!p.includes('\\')&&!p.includes(':')&&!p.split('/').some(s=>!s||s==='.'||s==='..'))||typeof value.keepStructure!=='boolean'||!['rename','skip'].includes(value.conflictPolicy))return null
 if(value.zipPasswordEncoding!==undefined&&!isZipPasswordEncoding(value.zipPasswordEncoding))return null
 return {source:{...(value.source.encrypted?{encrypted:true}:{}),path:value.source.path,kind:value.source.kind,revision:value.source.revision},paths:[...value.paths],keepStructure:value.keepStructure,conflictPolicy:value.conflictPolicy,...(value.zipPasswordEncoding?{zipPasswordEncoding:value.zipPasswordEncoding}:{})}
}

export interface ContentVerdict {path:string;kind:'same'|'different'|'metadata'|'unreadable';reason:string}
export interface ContentProgress {processedBytes:number;totalBytes:number;completed:number;total:number;path:string;side:string}
export const hasPair=(row:ComparisonRow)=>!!row.left&&!!row.right&&!row.left.directory&&!row.right.directory
