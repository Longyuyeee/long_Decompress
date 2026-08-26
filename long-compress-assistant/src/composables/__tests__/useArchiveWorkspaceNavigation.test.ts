import { describe, expect, it } from 'vitest'
import { ref } from 'vue'
import { useArchiveWorkspaceNavigation } from '../useArchiveWorkspaceNavigation'

describe('useArchiveWorkspaceNavigation', () => {
  it('derives a folder tree and preserves deterministic back/forward navigation', () => {
    const result = ref<any>({
      entries: [
        { path: 'docs/guides/readme.txt', name: 'readme.txt', isDir: false },
        { path: 'images/cover.png', name: 'cover.png', isDir: false },
      ],
    })
    const navigation = useArchiveWorkspaceNavigation(result)
    expect(navigation.directories.value).toEqual(['docs', 'docs/guides', 'images'])

    navigation.navigateToDirectory('docs')
    navigation.navigateToDirectory('docs/guides')
    navigation.goBack()
    expect(navigation.activeDirectory.value).toBe('docs')
    navigation.goForward()
    expect(navigation.activeDirectory.value).toBe('docs/guides')
    expect(navigation.breadcrumbs.value.map(item => item.name)).toEqual(['根目录', 'docs', 'guides'])
  })

  it('reconciles navigation when a refreshed archive removes folders', () => {
    const result = ref<any>({ entries: [{ path: 'old/file.txt', name: 'file.txt', isDir: false }] })
    const navigation = useArchiveWorkspaceNavigation(result)
    navigation.navigateToDirectory('old')
    result.value = { entries: [{ path: 'new/file.txt', name: 'file.txt', isDir: false }] }
    navigation.reconcile()
    expect(navigation.activeDirectory.value).toBe('')
    expect(navigation.navigationBack.value).toEqual([''])
  })
})
