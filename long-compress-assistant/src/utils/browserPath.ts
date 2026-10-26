export function browserBreadcrumbs(path: string): Array<{ name: string; path: string }> {
  const drive = path.match(/^([A-Za-z]:)[\\/]*(.*)$/)
  const parts = (drive ? drive[2] : path.replace(/^[\\/]+/, '')).split(/[\\/]+/).filter(Boolean)
  let current = drive ? `${drive[1]}\\` : path.startsWith('\\\\') ? '\\\\' : '/'
  const crumbs = drive ? [{ name: drive[1], path: current }] : path.startsWith('\\\\') ? [] : [{ name: '/', path: '/' }]
  for (const name of parts) {
    current = `${current}${/[\\/]$/.test(current) ? '' : drive || path.startsWith('\\\\') ? '\\' : '/'}${name}`
    crumbs.push({ name, path: current })
  }
  return crumbs
}
