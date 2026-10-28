export interface RecoveryTimePreservation {preservedFiles:number;unpreservedFiles:number;preservedDirectories?:number;unpreservedDirectories?:number}
export const recoveryTimeDescription=(value:RecoveryTimePreservation)=>{
 const descriptions=[]
 if(value.preservedFiles+value.unpreservedFiles)descriptions.push(`${value.preservedFiles} 个文件已保留${value.unpreservedFiles?`，${value.unpreservedFiles} 个未完整保留`:''}`)
 if(value.preservedDirectories!==undefined&&value.unpreservedDirectories!==undefined&&value.preservedDirectories+value.unpreservedDirectories)descriptions.push(`${value.preservedDirectories} 个目录已保留${value.unpreservedDirectories?`，${value.unpreservedDirectories} 个未完整保留`:''}`)
 return `原修改时间：${descriptions.join('；')}。`
}
export function validTimePreservation(value:unknown,files:number,directories=0):value is RecoveryTimePreservation {
 if(!value||typeof value!=='object')return false
 const v=value as RecoveryTimePreservation
 const pair=(preserved:unknown,unpreserved:unknown,total:number)=>Number.isSafeInteger(preserved)&&Number(preserved)>=0&&Number(preserved)<=total&&Number.isSafeInteger(unpreserved)&&unpreserved===total-Number(preserved)
 return pair(v.preservedFiles,v.unpreservedFiles,files)&&((v.preservedDirectories===undefined&&v.unpreservedDirectories===undefined)||pair(v.preservedDirectories,v.unpreservedDirectories,directories))
}
export interface LzopRecoveryEntry {id:number;name:string;bytes:number;sha256:string;isDirectory?:boolean;storedBytes?:number;linkTarget?:string;modifiedTime?:{seconds:number;nanoseconds:number};timeWarning?:string}
export interface LzopRecoveryReport {format?:'LZOP'|'TAR.LZO';revision:string;entries:LzopRecoveryEntry[];complete:boolean;stopReason:string;stoppedMember:string|null;stoppedOffset:number;availableVolumes:number;ignoredVolumes:number}
export interface LzopRecoveryOutcome {outputDirectory:string;recoveredFiles:number;recoveredDirectories?:number;recoveredBytes:number;entries:LzopRecoveryEntry[];stopReason:string;partial:true;timePreservation?:RecoveryTimePreservation}
