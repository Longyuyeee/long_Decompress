import {invoke} from '@tauri-apps/api/tauri'
import type {ArchiveDiagnosticReport} from '@/composables/useTauriCommands'
import {copyZpaqRetrieval,type ZpaqRetrievalSelection} from '@/types/zpaqRetrieval'
export async function checkZpaqRetrieval(path:string,password:string,selection:ZpaqRetrievalSelection,diagnosticId:string){
 if(!copyZpaqRetrieval(selection))throw Error('部分取回范围无效，请重新诊断并选择')
 const report=await invoke<ArchiveDiagnosticReport>('diagnose_archive',{diagnosticId,filePath:path,password:password||null,zpaqSnapshot:selection.snapshot})
 if(['password_required','wrong_password','ambiguous_data'].includes(report.status))throw Error('请核对并填写此来源的密码后重新检查；未把其他受影响文件加入取回范围')
 const blocks=report.zpaqBlocks
 if(!blocks||blocks.revision!==selection.revision||selection.paths.some(p=>!blocks.entries.some(e=>e.path===p&&e.state==='verified')))throw Error('来源或诊断结果已变化，请返回归档诊断重新选择文件；未扩大为整包解压')
}
