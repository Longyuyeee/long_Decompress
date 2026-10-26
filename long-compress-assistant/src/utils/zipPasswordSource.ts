import { archiveSourceFormat } from './archiveSource'
import type { ZipPasswordEncoding } from '@/types/zipPassword'

/** Detected content wins over the suffix, including renamed ZIPs and SFX. */
export function supportsZipPasswordEncoding(path: string, actualFormat?: string | null): boolean {
  const format = (actualFormat || archiveSourceFormat(path))?.toLowerCase()
  // Unknown sources can still be ZIPs. Do not hide a restored explicit choice.
  // ALZ reuses this persisted source-codepage field for its ANSI filenames and
  // password bytes. Undefined still means the format-specific default (CP949).
  // EGG filenames retain their own Unicode/legacy marker; only its key bytes
  // use this field. Undefined EGG key encoding means UTF-8.
  return !format || format === 'zip' || format === 'zipx' || format === 'alz' || format === 'egg' || format === 'lzo' || format === 'lzop'
}

export function zipPasswordEncodingForSource(path: string, encoding?: ZipPasswordEncoding, actualFormat?: string | null) {
  return supportsZipPasswordEncoding(path, actualFormat) ? encoding : undefined
}
