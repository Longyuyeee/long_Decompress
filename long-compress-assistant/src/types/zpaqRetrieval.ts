import {isZpaqSnapshot,type ZpaqSnapshot} from './zpaqSnapshot'
export interface ZpaqRetrievalSelection {snapshot:ZpaqSnapshot;revision:string;paths:string[]}
export interface ZpaqRetrievalReceipt {schemaVersion:1;kind:'zpaq-partial-retrieval';source:string;snapshot:ZpaqSnapshot;revision:string;entries:{path:string;size:number;state:'retrieved'|'affected'|'unchecked'|'not_selected'}[]}
export function copyZpaqRetrieval(value:unknown):ZpaqRetrievalSelection|null {
 const s=value as ZpaqRetrievalSelection
 if(!s||!isZpaqSnapshot(s.snapshot)||typeof s.revision!=='string'||!/^[a-f0-9]{64}$/i.test(s.revision)||!Array.isArray(s.paths)||!s.paths.length||s.paths.length>2000||new TextEncoder().encode(JSON.stringify(s.paths)).length>60000)return null
 if(!s.paths.every(p=>typeof p==='string'&&p.length>0&&!p.includes('\\')&&!p.startsWith('/')&&!p.split('/').some(c=>!c||c==='.'||c==='..')&&!/[:\u0000]/.test(p))||new Set(s.paths.map(p=>p.toLowerCase())).size!==s.paths.length)return null
 return {snapshot:{...s.snapshot},revision:s.revision,paths:[...s.paths]}
}
