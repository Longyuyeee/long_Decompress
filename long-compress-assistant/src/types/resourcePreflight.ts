export type ResourcePreflightStatus = 'ready' | 'warning' | 'blocked'
export type StorageLocation = 'local' | 'network' | 'removable' | 'unknown'
export type StorageMedium = 'ssd' | 'hdd' | 'unknown'

export interface ResourcePreflightReport {
  operation: 'compression' | 'decompression'
  outputPath: string
  probePath: string
  mountPoint: string | null
  fileSystem: string | null
  location: StorageLocation
  medium: StorageMedium
  totalBytes: number | null
  availableBytes: number | null
  estimatedOutputBytes: number | null
  requiredBytes: number | null
  reserveBytes: number
  estimateSource: 'archive_metadata' | 'provided_estimate' | 'unknown'
  estimateReliable: boolean
  status: ResourcePreflightStatus
  canStart: boolean
  summary: string
  warnings: string[]
}

export interface ResourcePreflightRequest {
  operation: 'compression' | 'decompression'
  outputPath: string
  sourcePaths: string[]
  password?: string
  estimatedOutputBytes?: number
  estimateReliable?: boolean
}
