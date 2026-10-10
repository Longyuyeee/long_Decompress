import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useSavedLocations } from '../useSavedLocations'

describe('saved directory locations', () => {
  beforeEach(() => localStorage.clear())
  it('persists bookmarks, deduplicates Windows paths and remembers the last 20 successful locations', () => {
    const locations = useSavedLocations()
    expect(locations.toggleBookmark('C:\\资料')).toBe(true)
    expect(useSavedLocations().bookmarks.value).toEqual(['C:\\资料'])
    expect(locations.isBookmarked('c:/资料/')).toBe(true)
    for (let i = 0; i < 30; i++) locations.remember(`C:/folder${i}`)
    expect(useSavedLocations().recent.value).toHaveLength(20)
    locations.toggleBookmark('c:/资料/')
    expect(useSavedLocations().bookmarks.value).toEqual([])
  })
  it('does not overwrite unreadable saved data during automatic navigation', () => {
    localStorage.setItem('long-saved-locations-v1', '{invalid')
    const locations = useSavedLocations()
    expect(locations.remember('C:/')).toBe(false)
    expect(locations.error.value).toContain('保留原数据')
    expect(localStorage.getItem('long-saved-locations-v1')).toBe('{invalid')
  })
  it('reports persistence failure without pretending a bookmark was saved', () => {
    const storage = { getItem: () => null, setItem: vi.fn(() => { throw new Error('full') }) } as unknown as Storage
    const locations = useSavedLocations(storage)
    expect(locations.toggleBookmark('C:/')).toBe(false)
    expect(locations.bookmarks.value).toEqual([])
    expect(locations.error.value).toContain('保存失败')
  })
})
