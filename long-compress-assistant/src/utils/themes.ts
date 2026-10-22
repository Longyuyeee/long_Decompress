export const APP_THEMES = [
  { value: 'light', icon: 'pi pi-sun', preview: ['#ffffff', '#e2e8f0', '#0ea5e9'] },
  { value: 'dark', icon: 'pi pi-moon', preview: ['#0b0e14', '#263245', '#38bdf8'] },
  { value: 'cyberpunk', icon: 'pi pi-bolt', preview: ['#0d0221', '#44205c', '#00f2ff'] },
  { value: 'twilight', icon: 'pi pi-star', preview: ['#1e1e2e', '#414664', '#b4befe'] },
  { value: 'sepia', icon: 'pi pi-book', preview: ['#f4ecd8', '#d9c7a3', '#94713d'] },
  { value: 'glacier', icon: 'pi pi-sun', preview: ['#eef7fc', '#bedeea', '#087fa1'], accent: '#087fa1' },
  { value: 'forest', icon: 'pi pi-sparkles', preview: ['#0d1a17', '#25483d', '#6bd4a1'], accent: '#6bd4a1' },
  { value: 'nebula', icon: 'pi pi-star', preview: ['#171226', '#44305f', '#e2a4ee'], accent: '#e2a4ee' },
  { value: 'obsidian', icon: 'pi pi-star', preview: ['#141414', '#38332a', '#d9b76f'], accent: '#d9b76f' },
  { value: 'auto', icon: 'pi pi-desktop', preview: ['#f8fafc', '#334155', '#0ea5e9'] },
] as const

export type AppTheme = typeof APP_THEMES[number]['value']
export const parseAppTheme = (value: unknown): AppTheme => APP_THEMES.some(theme => theme.value === value) ? value as AppTheme : 'auto'

export const accentForeground = (hex: string) => {
  if (!/^#[\da-f]{6}$/i.test(hex)) return '#ffffff'
  const channels = [1, 3, 5].map(index => parseInt(hex.slice(index, index + 2), 16) / 255)
    .map(channel => channel <= .04045 ? channel / 12.92 : ((channel + .055) / 1.055) ** 2.4)
  const luminance = channels[0] * .2126 + channels[1] * .7152 + channels[2] * .0722
  return luminance > .179 ? '#101820' : '#ffffff'
}
