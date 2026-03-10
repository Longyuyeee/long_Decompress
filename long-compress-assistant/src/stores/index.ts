/**
 * Pinia Store 导出文件
 */

export { useAppStore } from './app'
export { useFileStore } from './file'
export { useUIStore } from './ui'
export { useCompressionStore } from './compression'
export { useDecompressionStore } from './decompression'
export { 
  usePasswordStore, 
  PasswordCategory, 
  PasswordStrength, 
  CustomFieldType 
} from './password'

// 类型导出
export type { FileItem, DecompressTask, AppSettings } from './app'
export type { FileHistory, FavoriteFile } from './file'
export type { Notification, ModalState, Toast } from './ui'
export type { CompressionOptions, CompressionTask, CompressionHistory } from './compression'
export type { DecompressSettings, DecompressTask as DecompressStoreTask, DecompressHistory } from './decompression'
export type { 
  PasswordEntry, 
  PasswordGroup, 
  CustomField,
  AddPasswordRequest,
  UpdatePasswordRequest,
  PasswordStrengthAssessment
} from './password'
