/** Undefined means the listing cannot justify changing the user's setting. */
export function inferExtractionSubfolder(entries: readonly string[]): boolean | undefined {
  const roots = new Set<string>()
  for (const entry of entries) {
    const path = entry.replace(/\\/g, '/').replace(/\/+$/, '')
    if (!path) continue
    const parts = path.split('/')
    if (parts.some(part => !part || part === '.' || part === '..') || /^[a-z]:/i.test(path)) return undefined
    roots.add(parts[0])
  }
  return roots.size ? roots.size > 1 : undefined
}
