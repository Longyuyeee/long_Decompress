export interface RecoveryTimePreservation {preservedFiles:number;unpreservedFiles:number}
export const recoveryTimeDescription=(value:RecoveryTimePreservation)=>`原修改时间：${value.preservedFiles} 个文件已保留${value.unpreservedFiles?`，${value.unpreservedFiles} 个未完整保留`:''}。`
export function validTimePreservation(value:unknown,files:number):value is RecoveryTimePreservation {
 if(!value||typeof value!=='object')return false
 const v=value as RecoveryTimePreservation
 return Number.isSafeInteger(v.preservedFiles)&&v.preservedFiles>=0&&v.preservedFiles<=files&&Number.isSafeInteger(v.unpreservedFiles)&&v.unpreservedFiles===files-v.preservedFiles
}
export interface LzopRecoveryEntry {id:number;name:string;bytes:number;sha256:string;isDirectory?:boolean;modifiedTime?:{seconds:number;nanoseconds:number};timeWarning?:string}
export interface LzopRecoveryReport {format?:'LZOP'|'TAR.LZO';revision:string;entries:LzopRecoveryEntry[];complete:boolean;stopReason:string;stoppedMember:string|null;stoppedOffset:number;availableVolumes:number;ignoredVolumes:number}
export interface LzopRecoveryOutcome {outputDirectory:string;recoveredFiles:number;recoveredDirectories?:number;recoveredBytes:number;entries:LzopRecoveryEntry[];stopReason:string;partial:true;timePreservation?:RecoveryTimePreservation}
