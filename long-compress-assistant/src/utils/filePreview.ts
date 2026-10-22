const textExtensions = new Set(['txt', 'md', 'log', 'csv', 'tsv', 'json', 'jsonl', 'xml', 'yml', 'yaml', 'ini', 'cfg', 'conf', 'toml', 'css', 'html', 'htm', 'js', 'ts', 'vue', 'rs', 'py', 'sql'])
const imageExtensions = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'])

export function localPreviewKind(name: string): 'image' | 'text' | null {
  const extension = name.split('.').pop()?.toLowerCase() || ''
  if (imageExtensions.has(extension)) return 'image'
  if (textExtensions.has(extension) || /^(readme|license|changelog|authors|copying|makefile|dockerfile)$/i.test(name)) return 'text'
  return null
}
