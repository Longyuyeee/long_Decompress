import { snapshotArguments } from '@/types/zpaqSnapshot'
import { invoke } from '@tauri-apps/api/tauri'
import type { Task } from '@/stores/task'
import type { ArchiveDiagnosticReport } from '@/composables/useTauriCommands'
import { describeTaskFailure, getTaskRecoveryGuidance } from './taskFailure'
import { isDecompressArchivePath, isPotentialSplitArchivePath, isPotentialSfxPath } from './compressionFormat'
import type { ZipPasswordEncoding } from '@/types/zipPassword'
import { zipPasswordEncodingForSource } from './zipPasswordSource'

export function extractionRetryPolicy(task: Task) {
  const record = { status: task.status === 'failed' ? 'failed' as const : 'cancelled' as const, errorMessage: task.error }
  const category = describeTaskFailure(record)?.category
  const blocked = task.type !== 'decompression' || (task.workloadKind && task.workloadKind !== 'archive')
    || !['failed','cancelled'].includes(task.status) ? '仅支持已失败或已取消的归档解压任务。'
    : task.comparisonSelection ? '此任务保留资料对比的精确范围，请返回资料对比重新读取来源，或从历史记录按原范围创建任务。'
    : category === 'rollback-incomplete' ? '请先保留并人工核对原输出和恢复目录；此处不能直接重试回滚不完整的任务。'
    : task.sourceFiles.length !== 1 || task.fileFilter || (task.selectedEntries?.length && !(task.zpaqSnapshot && task.zpaqCatalogId) && !task.librarySourceId && !task.readerSelection)
      ? '此任务包含特殊来源或局部解压范围，请返回归档浏览重新选择范围。' : ''
  return { blocked, category, guidance: getTaskRecoveryGuidance(record), requireIntegrity: category === 'damaged' }
}

export interface ExtractionRetryInput {
  readerSelection?: import('@/types/archiveReading').ReaderSelection
  librarySourceId?: string
  libraryRevision?: string
  libraryKeepStructure?: boolean
  libraryConflictPolicy?: 'rename' | 'skip'

  zpaqSnapshot?: import('@/types/zpaqSnapshot').ZpaqSnapshot
  zpaqCatalogId?: string
  selectedEntries?: string[]
  source: string
  output: string
  password: string
  extractToSubfolder: boolean
  zipPasswordEncoding?: ZipPasswordEncoding
}

export async function validateExtractionRetry(
  input: ExtractionRetryInput,
  requireIntegrity: boolean,
  diagnosticId: string,
  cancelled: () => boolean,
) {
  const check = () => { if (cancelled()) throw new Error('已停止检查，未创建此任务。') }
  let path = input.source.trim()
  const outputPath = input.output.trim()
  if (!path || !outputPath) throw new Error('请确认来源文件与本次输出目录。')
  if (!isDecompressArchivePath(path) && !isPotentialSplitArchivePath(path) && !isPotentialSfxPath(path)) throw new Error('请选择支持的归档文件或首个分卷。')
  check()
  const split = await invoke<{ is_split: boolean; archive_format?: string; first_part?: string; is_complete?: boolean; missing_parts?: string[] } | null>('detect_split_archive', { path })
  check()
  if (split?.is_split) {
    if (!split.first_part || split.is_complete === false || split.missing_parts?.length) {
      throw new Error(`分卷仍不完整，请补齐后再检查：${split.missing_parts?.join('、') || '缺少必要分卷'}`)
    }
    path = split.first_part
  } else if (isPotentialSplitArchivePath(path) || (!isDecompressArchivePath(path) && !(isPotentialSfxPath(path) && split?.archive_format))) throw new Error('无法确认分卷组，请选择首卷。')
  const file = await invoke<{ name: string; is_dir: boolean; size: number }>('get_file_info', { path })
  check()
  if (file.is_dir || file.size <= 0) throw new Error('来源必须是非空归档文件，请确认已下载或复制完成。')
  let output: { is_dir: boolean }
  try {
    output = await invoke<{ is_dir: boolean }>('get_file_info', { path: outputPath })
  } catch (error) {
    check()
    // A comparison draft reserves a new version folder without creating it.
    // Keep that distinct output after a failure before extraction started.
    const parent = outputPath.slice(0, Math.max(outputPath.lastIndexOf('/'), outputPath.lastIndexOf('\\')) + 1)
    let parentExists = false
    if ((input.zpaqCatalogId || input.librarySourceId) && parent && parent !== outputPath) {
      try { parentExists = (await invoke<{ is_dir: boolean }>('get_file_info', { path: parent })).is_dir } catch { /* Show the original directory error. */ }
    }
    if (!parentExists) throw new Error(`无法读取本次输出目录「${outputPath}」，请创建该目录或选择已存在且可访问的目录。原归档未改动。详情：${String(error)}`)
    output = { is_dir: true }
  }
  check()
  if (!output.is_dir) throw new Error('输出位置必须是已存在的目录。')
  const zipPasswordEncoding = zipPasswordEncodingForSource(path, input.zipPasswordEncoding, split?.archive_format)
  if(input.readerSelection){await invoke('check_image_reader_source',{filePath:path,revision:input.readerSelection.revision});check()}
  if(input.librarySourceId){const source=await invoke<{path:string}>('library_selection',{sourceId:input.librarySourceId,revision:input.libraryRevision,paths:input.selectedEntries});check();if(source.path.toLowerCase()!==path.toLowerCase())throw Error('来源已重新定位，请返回资料库重新选择文件')}
  if (input.zpaqCatalogId && input.zpaqSnapshot) {
    await invoke('check_zpaq_snapshot_catalog', { filePath: path, password: input.password || null, snapshot: input.zpaqSnapshot, catalogId: input.zpaqCatalogId }); check()
  }
  if (input.zpaqSnapshot) {
    await invoke('browse_archive', { filePath: path, password: input.password || null, ...snapshotArguments(input.zpaqSnapshot) })
    check()
  }
  if (requireIntegrity) {
    const report = await invoke<ArchiveDiagnosticReport>('diagnose_archive', {
      diagnosticId, filePath: path, password: input.password || null, ...snapshotArguments(input.zpaqSnapshot),
      ...(zipPasswordEncoding !== undefined ? { zipPasswordEncoding } : {}),
    })
    check()
    if (report.status !== 'healthy' || !report.integrityTested) {
      throw new Error(report.status === 'password_encoding'
        ? '原包文件名或密码无法按所选编码表示，请确认原工具代码页后重新检查；这不代表文件损坏。'
        : report.status === 'password_required' || report.status === 'wrong_password'
        ? '检查完整性需要正确密码，请填写后重新检查。'
        : '来源尚未通过完整性检查，请重新获取完整文件，或先到文件完整性页面诊断。')
    }
  }
  return { path, outputPath, name: file.name, zipPasswordEncoding, format: split?.archive_format || undefined }
}
