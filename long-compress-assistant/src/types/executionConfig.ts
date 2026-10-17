import type { CompressionOptions } from '@/stores/compression'
import type { ImageCompressionSettings } from '@/utils/imageCompressionWorkspace'
import type { VideoCompressionSettings } from '@/types/video'
import type { PdfOptimizationMode } from '@/types/pdf'
import { isZipPasswordEncoding, type ZipPasswordEncoding } from './zipPassword'

export interface ExtractionConfigV1 {
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
  sourceZipPasswordEncoding?: ZipPasswordEncoding
  zipMethod?: 'deflate' | 'zstd'
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
export type ExecutionConfigV1 = ExtractionConfigV1 | ArchiveCompressionConfigV1 | ArchiveConversionConfigV1 | ImageConfigV1 | VideoConfigV1 | PdfConfigV1

/** Explicit fields only: never serialize passwords, deletion or transient consent. */
export function copyExecutionConfig(config?: ExecutionConfigV1 | null): ExecutionConfigV1 | null {
  if (!config || config.schemaVersion !== 1) return null
  if (config.kind === 'archive-extraction' && config.zipPasswordEncoding != null
      && !isZipPasswordEncoding(config.zipPasswordEncoding)) return null
  if (config.kind === 'archive-conversion' && config.zipPasswordEncoding !== undefined
      && !isZipPasswordEncoding(config.zipPasswordEncoding)) return null
  if (config.kind === 'archive-conversion' && config.sourceZipPasswordEncoding != null
      && !isZipPasswordEncoding(config.sourceZipPasswordEncoding)) return null
  if (config.kind === 'archive-compression' && config.settings.zipPasswordEncoding !== undefined
      && !isZipPasswordEncoding(config.settings.zipPasswordEncoding)) return null
  switch (config.kind) {
    case 'archive-conversion': return { schemaVersion: 1, kind: config.kind,
      ...(config.sourceZipPasswordEncoding ? { sourceZipPasswordEncoding: config.sourceZipPasswordEncoding } : {}),
      zipMethod: config.zipMethod ?? 'deflate',
      zipEncryption: config.zipEncryption ?? 'aes256', zipPasswordEncoding: config.zipPasswordEncoding ?? 'utf8',
      ...(config.zipSplitMode ? { zipSplitMode: config.zipSplitMode } : {}),
      outputPath: config.outputPath, format: config.format, level: config.level,
      rarRecoveryPercent: config.rarRecoveryPercent ?? 0,
      rarRecoveryVolumes: config.rarRecoveryVolumes ?? 0,
      splitSize: config.splitSize, encryptFilenames: config.encryptFilenames, passwordRequired: config.passwordRequired }
    case 'archive-extraction': return { schemaVersion: 1, kind: config.kind,
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
