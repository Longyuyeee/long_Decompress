/** ZIP password bytes, not archive filenames or text-preview encoding. */
export const ZIP_PASSWORD_ENCODINGS = ['utf8', 'windows_local', 'cp932', 'cp936', 'cp950'] as const
export type ZipPasswordEncoding = typeof ZIP_PASSWORD_ENCODINGS[number]

export function isZipPasswordEncoding(value: unknown): value is ZipPasswordEncoding {
  return typeof value === 'string' && (ZIP_PASSWORD_ENCODINGS as readonly string[]).includes(value)
}
