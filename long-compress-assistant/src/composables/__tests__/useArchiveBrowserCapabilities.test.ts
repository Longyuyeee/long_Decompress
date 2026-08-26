import { beforeEach, describe, expect, it, vi } from 'vitest'

const state = vi.hoisted(() => ({
  capabilities: {
    value: {
      browseExtensions: ['zip', '7z', 'zst', 'qcow2'],
      nestedExtensions: ['zip', '7z', 'zst'],
      boundedPreviewFormats: ['ZIP', 'TAR'],
      imagePreviewExtensions: ['png', 'jpg'],
      textPreviewExtensions: ['txt', 'md'],
    },
  },
  refresh: vi.fn(),
}))

vi.mock('@/composables/useArchiveEngine', () => ({
  useArchiveEngine: () => ({
    capabilities: state.capabilities,
    loading: { value: false },
    refresh: state.refresh,
  }),
}))

import { useArchiveBrowserCapabilities } from '../useArchiveBrowserCapabilities'

describe('useArchiveBrowserCapabilities', () => {
  beforeEach(() => vi.clearAllMocks())

  it('uses backend workspace policy for nested and preview decisions', () => {
    const capabilities = useArchiveBrowserCapabilities()
    expect(capabilities.isNestedArchiveName('payload.zst')).toBe(true)
    expect(capabilities.isNestedArchiveName('report.docx')).toBe(false)
    expect(capabilities.entryCategory('disk.qcow2')).toBe('archive')
    expect(capabilities.isNestedArchiveName('disk.qcow2')).toBe(false)
    expect(capabilities.previewKind('cover.png')).toBe('image')
    expect(capabilities.previewKind('readme.md')).toBe('text')
    expect(capabilities.supportsBoundedPreview('TAR.ZST')).toBe(true)
    expect(capabilities.supportsBoundedPreview('7Z')).toBe(false)
  })
})
