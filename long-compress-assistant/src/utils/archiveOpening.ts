export const archiveOpenOptions = [
  { value: 'here', label: '解压到当前文件夹', icon: 'pi pi-folder-open' },
  { value: 'subfolder', label: '解压到同名文件夹', icon: 'pi pi-folder' },
  { value: 'choose-folder', label: '选择解压文件夹…', icon: 'pi pi-directions' },
  { value: 'browse', label: '在双栏浏览器中查看', icon: 'pi pi-columns' },
] as const
export type ArchiveOpenAction = typeof archiveOpenOptions[number]['value']
export type ArchiveOpenBehavior = 'ask' | ArchiveOpenAction
export const isArchiveOpenBehavior = (value: unknown): value is ArchiveOpenBehavior =>
  value === 'ask' || archiveOpenOptions.some(option => option.value === value)
