import type { BatchConfigPatch } from '@/types/batchConfig'
import type { ConversionPatch } from '@/types/archiveConversion'
export interface BatchSubject { name: string; path?: string; format?: string }
/** Describe only the staged patch. Never include credential values. */
export function batchConfigSummary(patch: BatchConfigPatch | ConversionPatch): string[] {
  const rows: string[] = []
  if (patch.format !== undefined) rows.push(`格式：${patch.format.toUpperCase()}`)
  if (patch.level !== undefined) rows.push(`压缩强度：${patch.level}`)
  if(patch.sevenZipFilter !== undefined) rows.push(`7Z 预处理：${patch.sevenZipFilter === 'delta' ? 'Delta' : patch.sevenZipFilter === 'bcj_x86' ? 'x86 BCJ' : patch.sevenZipFilter === 'bcj_arm32' ? 'ARM32 BCJ' : patch.sevenZipFilter === 'bcj_arm_thumb' ? 'ARM Thumb BCJ' : patch.sevenZipFilter === 'bcj_arm64' ? 'ARM64 BCJ' : patch.sevenZipFilter === 'bcj_riscv' ? 'RISC-V BCJ' : '关闭'}`)
  if(patch.sevenZipDeltaDistance !== undefined) rows.push(`Delta 距离：${patch.sevenZipDeltaDistance}`)
  if (patch.sevenZipMethod !== undefined) rows.push(`7Z 方法：${patch.sevenZipMethod === 'bzip2' ? 'Bzip2' : patch.sevenZipMethod.toUpperCase()}`)
  if (patch.zipMethod !== undefined) rows.push(`ZIP 方法：${({store:'仅存储（Store）',deflate:'Deflate',zstd:'Zstandard',bzip2:'BZip2',ppmd:'PPMd',lzma:'LZMA'})[patch.zipMethod]}`)
  if (patch.wimMethod !== undefined) rows.push(`WIM/SWM 方法：${patch.wimMethod === 'store' ? '仅存储' : patch.wimMethod === 'solid' ? 'LZMS 固实' : patch.wimMethod.toUpperCase()}`)
  if (patch.outputDirectory !== undefined) rows.push(`输出目录：${patch.outputDirectory || '各自源文件目录'}`)
  if (patch.password !== undefined) rows.push(patch.password ? '新包密码：统一设置' : '新包密码：关闭加密')
  if ('sourcePassword' in patch && patch.sourcePassword !== undefined) rows.push(patch.sourcePassword ? '原包密码：统一填写' : '原包密码：清空')
  if (patch.encryptFilenames !== undefined) rows.push(`文件名加密：${patch.encryptFilenames ? '启用' : '关闭'}`)
  if ('splitArchive' in patch && patch.splitArchive !== undefined) rows.push(`分卷：${patch.splitArchive ? `${patch.splitSize} MiB` : '关闭'}`)
  if ('splitMiB' in patch && patch.splitMiB !== undefined) rows.push(`分卷：${patch.splitMiB ? `${patch.splitMiB} MiB` : '关闭'}`)
  if (patch.zipSplitMode !== undefined) rows.push(`ZIP 卷型：${patch.zipSplitMode === 'standard' ? '标准分卷' : '数字切卷'}`)
  if (patch.rarRecoveryPercent !== undefined) rows.push(`RAR 恢复保护：${patch.rarRecoveryPercent}%`)
  if (patch.rarRecoveryVolumes !== undefined) rows.push(`RAR 恢复卷：${patch.rarRecoveryVolumes} 个`)
  if ('extractToSubfolder' in patch && patch.extractToSubfolder !== undefined) rows.push(`文件放置：${patch.extractToSubfolder ? '创建同名子文件夹' : '直接放入输出目录'}`)
  if (patch.zipPasswordEncoding !== undefined) rows.push(`ZIP 密码编码：${patch.zipPasswordEncoding === 'auto' ? '自动兼容' : patch.zipPasswordEncoding.toUpperCase()}`)
  if ('sourceZipPasswordEncoding' in patch) rows.push(`原包读取编码：${patch.sourceZipPasswordEncoding?.toUpperCase() || '格式默认值'}`)
  return rows
}
