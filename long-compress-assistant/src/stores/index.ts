/**
 * Pinia Store 导出文件
 */

export { useAppStore } from './app'
export { useUIStore } from './ui'
export { useCompressionStore } from './compression'
export {
  usePasswordStore,
  PasswordCategory,
  PasswordStrength,
  CustomFieldType
} from './password'

// 注意：useFileStore 和 useDecompressionStore 已废弃，当前未被任何活跃代码使用。
// 它们保留在源码中供未来参考，但不从 barrel 导出。

// 类型导出
export type { FileItem, DecompressTask, AppSettings } from '../types'
export type { Notification, ModalState, Toast } from './ui'
export type { CompressionOptions, CompressionTask, CompressionHistory } from './compression'
export type {
  PasswordEntry,
  PasswordGroup,
  CustomField,
  AddPasswordRequest,
  UpdatePasswordRequest,
  PasswordStrengthAssessment
} from './password'
