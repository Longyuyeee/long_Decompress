import { copyComparisonReference } from './comparisonPackage'
import type { ZipEditSession } from '@/stores/zipEdit'
import type { ZipEditCatalog, ZipEditDraft, ZipStructureOperation, ZipEditOutputMode } from '@/types/zipEdit'
import { isZipPasswordEncoding, type ZipPasswordEncoding } from '@/types/zipPassword'
export interface SavedZipEdit {
 sourcePath:string;catalog:ZipEditCatalog;drafts:ZipEditDraft[];operations:ZipStructureOperation[];method:'deflate'|'store';level:number;outputPath:string;saves:number;taskId?:string;savedAt:string
 sourceEncoding?:ZipPasswordEncoding;outputMode:ZipEditOutputMode;outputEncoding:ZipPasswordEncoding
}
const text=(value:unknown):value is string=>typeof value==='string'&&value.length>0&&value.length<=32768
export function savedZipEdit(session:ZipEditSession):SavedZipEdit {
 return {sourcePath:session.sourcePath,catalog:session.catalog,drafts:session.drafts.map(row=>({id:row.id,batch:row.batch,order:row.order,choice:row.choice,sourcePath:row.sourcePath,targetPath:row.targetPath,isDir:row.isDir,size:row.size,sourceRevision:row.sourceRevision,comparison:copyComparisonReference(row.comparison)})),operations:session.operations.map(row=>({id:row.id,order:row.order,kind:row.kind,targets:row.targets.map(target=>({id:target.id,path:target.path}))})),method:session.method,level:session.level,outputPath:session.outputPath,saves:session.saves,taskId:session.taskId,savedAt:new Date().toISOString(),sourceEncoding:session.sourceEncoding,outputMode:session.outputMode,outputEncoding:session.outputEncoding}
}
export function parseZipEdits(raw:string|null):SavedZipEdit[]{
 if(!raw)return[]
 const value=JSON.parse(raw)
 if(value.version!==1||!Array.isArray(value.sessions))throw new Error('编辑草稿格式无效，上次记录保留')
 return value.sessions.map((row:any)=>{
  if(!text(row.sourcePath)||!text(row.catalog?.revision)||!Array.isArray(row.catalog.entries)||row.catalog.entries.length>1000000||!Array.isArray(row.catalog.methods)||!Array.isArray(row.drafts)||row.drafts.length>10000||!Array.isArray(row.operations)||row.operations.length>10000)throw new Error('编辑草稿不完整，上次记录保留')
  if(!row.catalog.entries.every((entry:any)=>text(entry.path)&&typeof entry.isDir==='boolean'&&Number.isFinite(entry.size))||!row.drafts.every((draft:any)=>text(draft.id)&&text(draft.batch)&&text(draft.sourcePath)&&text(draft.targetPath)&&text(draft.sourceRevision)&&typeof draft.isDir==='boolean'&&Number.isFinite(draft.size)&&['choose','replace','skip','keep-both'].includes(draft.choice))||!row.operations.every((op:any)=>text(op.id)&&Number.isFinite(op.order)&&['rename','delete'].includes(op.kind)&&Array.isArray(op.targets)&&op.targets.length<=200000&&op.targets.every((target:any)=>text(target.id)&&(target.path===null||text(target.path)))))throw new Error('编辑草稿条目无效，上次记录保留')
  for (const draft of row.drafts) if (draft.comparison && !copyComparisonReference(draft.comparison)) throw new Error('组包来源引用不完整，上次记录保留')
  const session={sourcePath:row.sourcePath,catalog:{revision:row.catalog.revision,methods:row.catalog.methods.filter(text),entries:row.catalog.entries.map((entry:any)=>({path:entry.path,isDir:entry.isDir,size:entry.size})),encrypted:row.catalog.encrypted===true,defaultEncryption:row.catalog.defaultEncryption,encryptions:Object.fromEntries(Object.entries(row.catalog.encryptions||{}).filter(([,value])=>['none','zipcrypto','aes128','aes192','aes256'].includes(String(value))))},drafts:row.drafts,operations:row.operations,method:row.method==='store'?'store':'deflate',level:Number.isInteger(row.level)&&row.level>=0&&row.level<=9?row.level:6,outputPath:typeof row.outputPath==='string'?row.outputPath:'',saves:Number.isInteger(row.saves)?row.saves:0,taskId:text(row.taskId)?row.taskId:undefined,sourceEncoding:isZipPasswordEncoding(row.sourceEncoding)?row.sourceEncoding:undefined,outputMode:['keep','none','aes256','zipcrypto'].includes(row.outputMode)?row.outputMode:'keep',outputEncoding:isZipPasswordEncoding(row.outputEncoding)?row.outputEncoding:'utf8'} as ZipEditSession
  return {...savedZipEdit(session),savedAt:typeof row.savedAt==='string'?row.savedAt:''}
 })
}
