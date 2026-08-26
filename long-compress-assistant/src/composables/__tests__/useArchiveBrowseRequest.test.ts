import { describe, expect, it, vi } from 'vitest'
import { describeArchiveBrowseError, useArchiveBrowseRequest } from '../useArchiveBrowseRequest'

describe('useArchiveBrowseRequest', () => {
  it('forwards one request id and cancels the active backend read', async () => {
    let resolveBrowse!: (value: any) => void
    const browseArchive = vi.fn(() => new Promise(resolve => { resolveBrowse = resolve }))
    const cancelArchiveBrowse = vi.fn().mockResolvedValue(undefined)
    const request = useArchiveBrowseRequest(browseArchive, cancelArchiveBrowse)

    const pending = request.run('C:/archives/large.tar', '')
    expect(request.loading.value).toBe(true)
    const browseId = browseArchive.mock.calls[0][2]
    request.cancel()

    expect(cancelArchiveBrowse).toHaveBeenCalledWith(browseId)
    expect(request.loading.value).toBe(false)
    expect(request.notice.value).toBe('已取消读取压缩包内容')
    resolveBrowse({ entries: [] })
    await pending
    expect(request.loading.value).toBe(false)
  })

  it('maps structured backend failures to user-facing text', () => {
    expect(describeArchiveBrowseError('ARCHIVE_BROWSE_TIMEOUT|读取超时|detail')).toBe('读取超时')
    expect(describeArchiveBrowseError(new Error('raw parser detail'))).toContain('确认文件完整')
  })
})
