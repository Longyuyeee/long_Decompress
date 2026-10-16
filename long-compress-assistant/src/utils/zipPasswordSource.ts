import { archiveSourceFormat } from './archiveSource'
import type { ZipPasswordEncoding } from '@/types/zipPassword'

/** Detected content wins over the suffix, including renamed ZIPs and SFX. */
export function supportsZipPasswordEncoding(path: string, actualFormat?: string | null): boolean {
  const format = (actualFormat || archiveSourceFormat(path))?.toLowerCase()
  // Unknown sources can still be ZIPs. Do not hide a restored explicit choice.
  return !format || format === 'zip' || format === 'zipx'
}

export function zipPasswordEncodingForSource(path: string, encoding?: ZipPasswordEncoding, actualFormat?: string | null) {
  return supportsZipPasswordEncoding(path, actualFormat) ? encoding : undefined
}
