import type {ZpaqSnapshot} from './zpaqSnapshot'
export interface ZpaqBlock {id:number;offset:number;compressedSize:number;state:'verified'|'failed'|'unchecked';files:number}
export interface ZpaqCheckedFile {path:string;size:number;state:'verified'|'affected'|'unchecked';blocks:number[]}
export interface ZpaqBlockReport {snapshot:ZpaqSnapshot;blocks:ZpaqBlock[];entries:ZpaqCheckedFile[];revision:string}
export function zpaqCheckCounts(report:ZpaqBlockReport){return {verified:report.entries.filter(e=>e.state==='verified').length,affected:report.entries.filter(e=>e.state==='affected').length,unchecked:report.entries.filter(e=>e.state==='unchecked').length}}
export function zpaqReportLines(report:ZpaqBlockReport){
 const counts=zpaqCheckCounts(report)
 return [`ZPAQ 快照 ${report.snapshot.version} (${report.snapshot.date} UTC)`,`检查通过 ${counts.verified} · 受影响 ${counts.affected} · 未检查 ${counts.unchecked}`,`检查范围：所选快照引用的数据块；受影响不等于每个文件已证实损坏，未检查不计入通过。未执行部分提取。`,`来源目录标识：${report.revision}`,
  ...report.blocks.map(b=>`数据块 #${b.id} · ${b.state==='verified'?'检查通过':b.state==='failed'?'校验失败':'未检查'} · 偏移 ${b.offset} · ${b.compressedSize} bytes · ${b.files} 个关联文件`),
  ...report.entries.map(e=>`${e.state==='verified'?'检查通过':e.state==='affected'?'受影响':'未检查'}：${e.path} · ${e.size} bytes · 关联块 ${e.blocks.join(', ')||'无'}`)]
}
