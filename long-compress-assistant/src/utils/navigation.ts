// The displayed shortcuts and their keyboard handling share one definition.
export const APP_NAVIGATION = [
  { name: 'Decompress', path: '/decompress', icon: 'pi pi-folder-open', label: 'nav.decompress', key: 'o' },
  { name: 'Compress', path: '/compress', icon: 'pi pi-box', label: 'nav.compress', key: 'n' },
  { name: 'SpecialCompression', path: '/special-compression', icon: 'pi pi-sparkles', label: 'nav.special_compression', key: 's', shift: true },
  { name: 'ArchiveBrowser', path: '/browser', icon: 'pi pi-folder', label: 'nav.browser', key: 'b' },
  { name: 'ToolsCenter', path: '/tools', icon: 'pi pi-th-large', label: 'nav.tools', key: 't', shift: true },
  { name: 'Vault', path: '/vault', icon: 'pi pi-shield', label: 'nav.vault', key: 'v', shift: true },
  { name: 'History', path: '/history', icon: 'pi pi-history', label: 'nav.history', key: 'h' },
  { name: 'Settings', path: '/settings', icon: 'pi pi-cog', label: 'nav.settings', key: ',' },
].map(item => ({ ...item, shortcut: `Ctrl+${item.shift ? 'Shift+' : ''}${item.key.toUpperCase()}` }))

export const TOOL_NAVIGATION = [
  { name: 'SourceComparison', path: '/tools/compare', icon: 'pi pi-arrows-h', label: 'nav.compare', key: 'd', shift: true },
  { name: 'Workflows', path: '/tools/workflows', icon: 'pi pi-sitemap', label: 'nav.workflows', key: 'w', shift: true },
  { name: 'ArchiveLibrary', path: '/tools/library', icon: 'pi pi-database', label: 'nav.library', key: 'l', shift: true },
  { name: 'FileIntegrity', path: '/tools/integrity', icon: 'pi pi-verified', label: 'nav.integrity', key: 'i', shift: false },
].map(item => ({ ...item, shortcut: `Ctrl+${item.shift ? 'Shift+' : ''}${item.key.toUpperCase()}` }))

export const TOOLS_CENTER_EMBEDDED = Symbol('tools-center-embedded')

export const navigationPathForShortcut = (event: KeyboardEvent): string | undefined => {
  if (event.defaultPrevented || event.isComposing || event.altKey || !(event.ctrlKey || event.metaKey)) return
  const target = event.target
  if (target instanceof Element && target.closest('input,textarea,select,[contenteditable]:not([contenteditable="false"])')) return
  // Do not leave a review dialog behind when a shortcut is pressed on a button.
  if (Array.from(document.querySelectorAll('[role="dialog"][aria-modal="true"]')).some(dialog => dialog.getClientRects().length)) return
  return [...APP_NAVIGATION, ...TOOL_NAVIGATION].find(item => item.key === event.key.toLowerCase() && Boolean(item.shift) === event.shiftKey)?.path
}
