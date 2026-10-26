export interface LzopRecoveryEntry {id:number;name:string;bytes:number;sha256:string;isDirectory?:boolean}
export interface LzopRecoveryReport {format?:'LZOP'|'TAR.LZO';revision:string;entries:LzopRecoveryEntry[];complete:boolean;stopReason:string;stoppedMember:string|null;stoppedOffset:number;availableVolumes:number;ignoredVolumes:number}
export interface LzopRecoveryOutcome {outputDirectory:string;recoveredFiles:number;recoveredDirectories?:number;recoveredBytes:number;entries:LzopRecoveryEntry[];stopReason:string;partial:true}
