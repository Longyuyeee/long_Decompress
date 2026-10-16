/** ZIP password bytes, not archive filenames or text-preview encoding. */
export const ZIP_PASSWORD_ENCODINGS = ['utf8', 'windows_local', 'cp932', 'cp936', 'cp950'] as const
export type ZipPasswordEncoding = typeof ZIP_PASSWORD_ENCODINGS[number]

export const ZIP_PASSWORD_ENCODING_OPTIONS = [
  { value: 'utf8', label: 'UTF-8' },
  { value: 'windows_local', label: '当前 Windows 代码页' },
  { value: 'cp932', label: '日文（CP932）' },
  { value: 'cp936', label: '简体中文（CP936）' },
  { value: 'cp950', label: '繁体中文（CP950）' },
]

export function isZipPasswordEncoding(value: unknown): value is ZipPasswordEncoding {
  return typeof value === 'string' && (ZIP_PASSWORD_ENCODINGS as readonly string[]).includes(value)
}
