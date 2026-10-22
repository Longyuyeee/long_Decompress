import {copyReaderSelection} from './archiveReading'
import {copyComparisonSelection,type ComparisonSelection} from './sourceComparison'
import {copyZpaqRetrieval} from './zpaqRetrieval'
import { isZpaqSnapshot, type ZpaqSnapshot } from './zpaqSnapshot'
import type { CompressionOptions } from '@/stores/compression'
import type { ImageCompressionSettings } from '@/utils/imageCompressionWorkspace'
import type { VideoCompressionSettings } from '@/types/video'
import type { PdfOptimizationMode } from '@/types/pdf'
import { isZipPasswordEncoding, type ZipPasswordEncoding } from './zipPassword'

export interface ExtractionConfigV1 {
  readerSelection?: import('@/types/archiveReading').ReaderSelection
  comparisonSelection?: ComparisonSelection
  librarySourceId?: string
  libraryRevision?: string
  libraryKeepStructure?: boolean
  libraryConflictPolicy?: 'rename' | 'skip'

  zpaqCatalogId?: string
  zpaqRetrieval?: import('@/types/zpaqRetrieval').ZpaqRetrievalSelection
  selectedEntries?: string[]
  zpaqSnapshot?: ZpaqSnapshot
  zipPasswordEncoding?: ZipPasswordEncoding
  schemaVersion: 1
  kind: 'archive-extraction'
  outputDirectory: string
  extractToSubfolder: boolean
}
export interface ArchiveCompressionConfigV1 {
  schemaVersion: 1
  kind: 'archive-compression'
  outputDirectory: string
  passwordRequired: boolean
  settings: Omit<CompressionOptions, 'password' | 'deleteAfter'>
}
export interface ImageConfigV1 {
  schemaVersion: 1
  kind: 'image-compression'
  settings: ImageCompressionSettings
}
export interface VideoConfigV1 {
  schemaVersion: 1
  kind: 'video-compression'
  outputDirectory: string
  settings: VideoCompressionSettings
}
export interface PdfConfigV1 {
  schemaVersion: 1
  kind: 'pdf-optimization'
  outputDirectory: string
  mode: PdfOptimizationMode
  allowLargerOutput: boolean
}
export interface ArchiveConversionConfigV1 {
  /** Whether an input credential must be confirmed; never the credential itself. */
  sourcePasswordRequired?: boolean
  operation?: 'zpaq-update'
  updateSources?: string[]
  archivePrefix?: string
  sourceZpaqSnapshot?: ZpaqSnapshot
  sourceZipPasswordEncoding?: ZipPasswordEncoding
  wimMethod?: 'store' | 'xpress' | 'lzx' | 'lzms' | 'solid'
  zipMethod?: 'deflate' | 'zstd' | 'bzip2' | 'ppmd' | 'lzma'
  zipEncryption?: 'aes256' | 'zipcrypto'
  zipSplitMode?: 'numeric' | 'standard'
  zipPasswordEncoding?: ZipPasswordEncoding
  schemaVersion: 1
  kind: 'archive-conversion'
  outputPath: string
  format: string
  level: number
  splitSize: number
  encryptFilenames: boolean
  rarRecoveryPercent?: number
  rarRecoveryVolumes?: number
  passwordRequired: boolean
}
export interface ArchiveEditConfigV1 { schemaVersion: 1; kind: 'archive-edit'; outputPath: string }
export type ExecutionConfigV1 = ArchiveEditConfigV1 | ExtractionConfigV1 | ArchiveCompressionConfigV1 | ArchiveConversionConfigV1 | ImageConfigV1 | VideoConfigV1 | PdfConfigV1

/** Explicit fields only: never serialize passwords, deletion or transient consent. */
export function copyExecutionConfig(config?: ExecutionConfigV1 | null): ExecutionConfigV1 | null {
  if (!config || config.schemaVersion !== 1) return null
  if (config.kind === 'archive-conversion' && config.operation !== undefined) {
    if (config.operation !== 'zpaq-update' || config.format !== 'zpaq' || config.sourceZpaqSnapshot
        || !Array.isArray(config.updateSources) || !config.updateSources.length || config.updateSources.length > 1000
        || !config.updateSources.every(path => typeof path === 'string' && path.trim().length > 0 && path.length <= 32768)
        || (config.archivePrefix !== undefined && (typeof config.archivePrefix !== 'string' || config.archivePrefix.length > 32768))) return null
  }
  if(config.kind==='archive-extraction'&&config.readerSelection!==undefined&&!copyReaderSelection(config.readerSelection))return null
  if(config.kind==='archive-extraction'&&config.comparisonSelection!==undefined&&!copyComparisonSelection(config.comparisonSelection))return null
  if (config.kind === 'archive-extraction' && config.zpaqCatalogId !== undefined
      && (!config.zpaqSnapshot || !/^[a-f0-9]{64}$/.test(config.zpaqCatalogId) || !Array.isArray(config.selectedEntries)
      || !config.selectedEntries.length || config.selectedEntries.length > 2000 || JSON.stringify(config.selectedEntries).length > 60000
      || !config.selectedEntries.every(path=>typeof path==='string'&&path.length>0&&path.length<=32768))) return null
  if (config.kind === 'archive-extraction' && config.librarySourceId !== undefined && (
      typeof config.librarySourceId !== 'string' || !config.librarySourceId || config.librarySourceId.length > 100
      || typeof config.libraryRevision !== 'string' || !config.libraryRevision || config.libraryRevision.length > 60000
      || !Array.isArray(config.selectedEntries) || !config.selectedEntries.length || config.selectedEntries.length > 2000 || JSON.stringify(config.selectedEntries).length > 60000
      || !config.selectedEntries.every(path=>typeof path==='string'&&path.length>0&&path.length<=32768)
      || typeof config.libraryKeepStructure !== 'boolean' || !['rename','skip'].includes(config.libraryConflictPolicy || ''))) return null
  if (config.kind === 'archive-extraction' && config.zpaqSnapshot != null && !isZpaqSnapshot(config.zpaqSnapshot)) return null
  if(config.kind==='archive-extraction'&&config.zpaqRetrieval!==undefined&&(!copyZpaqRetrieval(config.zpaqRetrieval)||(config.zpaqSnapshot?.version!==config.zpaqRetrieval.snapshot.version||config.zpaqSnapshot?.date!==config.zpaqRetrieval.snapshot.date)||config.readerSelection||config.comparisonSelection||config.librarySourceId||config.zpaqCatalogId))return null
  if (config.kind === 'archive-conversion' && config.sourceZpaqSnapshot != null && !isZpaqSnapshot(config.sourceZpaqSnapshot)) return null
  if (config.kind === 'archive-extraction' && config.zipPasswordEncoding != null
      && !isZipPasswordEncoding(config.zipPasswordEncoding)) return null
  if (config.kind === 'archive-conversion' && config.zipPasswordEncoding !== undefined
      && !isZipPasswordEncoding(config.zipPasswordEncoding)) return null
  if (config.kind === 'archive-conversion' && config.sourceZipPasswordEncoding != null
      && !isZipPasswordEncoding(config.sourceZipPasswordEncoding)) return null
  if (config.kind === 'archive-compression' && config.settings.zipPasswordEncoding !== undefined
      && !isZipPasswordEncoding(config.settings.zipPasswordEncoding)) return null
  if (config.kind === 'archive-conversion' && config.sourcePasswordRequired !== undefined && typeof config.sourcePasswordRequired !== 'boolean') return null
  const method = config.kind === 'archive-compression' ? config.settings.wimMethod : config.kind === 'archive-conversion' ? config.wimMethod : undefined
  if (method !== undefined && method !== 'store' && method !== 'xpress' && method !== 'lzx' && method !== 'lzms' && method !== 'solid') return null
  if (config.kind === 'archive-edit' && (typeof config.outputPath !== 'string' || !config.outputPath.trim())) return null
  switch (config.kind) {
    case 'archive-edit': return { schemaVersion: 1, kind: config.kind, outputPath: config.outputPath }
    case 'archive-conversion': return { schemaVersion: 1, kind: config.kind,
      ...(config.sourcePasswordRequired !== undefined ? { sourcePasswordRequired: config.sourcePasswordRequired } : {}),
      ...(config.operation === 'zpaq-update' ? { operation: config.operation, updateSources: [...(config.updateSources || [])], archivePrefix: config.archivePrefix || '' } : {}),
      ...(config.sourceZpaqSnapshot ? { sourceZpaqSnapshot: { ...config.sourceZpaqSnapshot } } : {}),
      ...(config.sourceZipPasswordEncoding ? { sourceZipPasswordEncoding: config.sourceZipPasswordEncoding } : {}),
      ...(config.wimMethod ? { wimMethod: config.wimMethod } : {}),
      zipMethod: config.zipMethod ?? 'deflate',
      zipEncryption: config.zipEncryption ?? 'aes256', zipPasswordEncoding: config.zipPasswordEncoding ?? 'utf8',
      ...(config.zipSplitMode ? { zipSplitMode: config.zipSplitMode } : {}),
      outputPath: config.outputPath, format: config.format, level: config.level,
      rarRecoveryPercent: config.rarRecoveryPercent ?? 0,
      rarRecoveryVolumes: config.rarRecoveryVolumes ?? 0,
      splitSize: config.splitSize, encryptFilenames: config.encryptFilenames, passwordRequired: config.passwordRequired }
    case 'archive-extraction': return { schemaVersion: 1, kind: config.kind,
      ...(config.zpaqRetrieval?{zpaqRetrieval:copyZpaqRetrieval(config.zpaqRetrieval)!}:{}),
      ...(config.readerSelection?{readerSelection:copyReaderSelection(config.readerSelection)!}:{}),
      ...(config.comparisonSelection?{comparisonSelection:copyComparisonSelection(config.comparisonSelection)!}:{}),
      ...(config.librarySourceId ? {librarySourceId:config.librarySourceId,libraryRevision:config.libraryRevision,libraryKeepStructure:config.libraryKeepStructure,libraryConflictPolicy:config.libraryConflictPolicy,selectedEntries:[...(config.selectedEntries||[])]} : {}),
      ...(config.zpaqCatalogId ? { zpaqCatalogId: config.zpaqCatalogId, selectedEntries: [...(config.selectedEntries || [])] } : {}),
      ...(config.zpaqSnapshot ? { zpaqSnapshot: { ...config.zpaqSnapshot } } : {}),
      ...(config.zipPasswordEncoding ? { zipPasswordEncoding: config.zipPasswordEncoding } : {}),
      outputDirectory: config.outputDirectory, extractToSubfolder: config.extractToSubfolder }
    case 'archive-compression': {
      const s = config.settings
      return { schemaVersion: 1, kind: config.kind, outputDirectory: config.outputDirectory,
        passwordRequired: config.passwordRequired, settings: {
          format: s.format, level: s.level, filename: s.filename, splitArchive: s.splitArchive,
          splitSize: s.splitSize, keepStructure: s.keepStructure, verifyAfter: s.verifyAfter,
          createSolidArchive: s.createSolidArchive, fastPack: false,
          ...(s.rarRecoveryPercent !== undefined ? { rarRecoveryPercent: s.rarRecoveryPercent } : {}),
          ...(s.rarRecoveryVolumes !== undefined ? { rarRecoveryVolumes: s.rarRecoveryVolumes } : {}),
          ...(s.encryptFilenames !== undefined ? { encryptFilenames: s.encryptFilenames } : {}),
          ...(s.zipPasswordEncoding ? { zipPasswordEncoding: s.zipPasswordEncoding } : {}),
          ...(s.wimMethod ? { wimMethod: s.wimMethod } : {}),
          ...(s.zipMethod ? { zipMethod: s.zipMethod } : {}),
          ...(s.zipEncryption ? { zipEncryption: s.zipEncryption } : {}),
          ...(s.zipSplitMode ? { zipSplitMode: s.zipSplitMode } : {}),
        } }
    }
    case 'image-compression': {
      const s = config.settings
      return { schemaVersion: 1, kind: config.kind, settings: {
        mode: s.mode, quality: s.quality, resizeMode: s.resizeMode, maxWidth: s.maxWidth,
        maxHeight: s.maxHeight, outputFormat: s.outputFormat, preserveMetadata: s.preserveMetadata,
        outputDirectory: s.outputDirectory, conflictPolicy: s.conflictPolicy,
      } }
    }
    case 'video-compression': {
      const s = config.settings
      return { schemaVersion: 1, kind: config.kind, outputDirectory: config.outputDirectory,
        settings: { preset: s.preset, quality: s.quality, maxWidth: s.maxWidth, maxHeight: s.maxHeight } }
    }
    case 'pdf-optimization': return { schemaVersion: 1, kind: config.kind,
      outputDirectory: config.outputDirectory, mode: config.mode, allowLargerOutput: config.allowLargerOutput }
    default: return null
  }
}
