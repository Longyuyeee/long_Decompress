import { describe, expect, it, vi } from 'vitest'
import { describeArchiveBrowseError, useArchiveBrowseRequest } from '../useArchiveBrowseRequest'

describe('useArchiveBrowseRequest', () => {
  it('keeps a failed read reason on the page and clears it on the next attempt', async () => {
    const browse = vi.fn().mockRejectedValueOnce('ARCHIVE_BROWSE_CORRUPTED|LZIP 校验未通过，请重新获取完整文件|CRC32 mismatch').mockResolvedValueOnce({ entries: [] })
    const request = useArchiveBrowseRequest(browse, vi.fn())
    await expect(request.run('C:/资料.tar.lz')).rejects.toContain('CRC32')
    expect(request.notice.value).toBe('LZIP 校验未通过，请重新获取完整文件')
    await request.run('C:/完整资料.tlz')
    expect(request.notice.value).toBe('')
  })
  it('does not replace a cancellation notice with a late backend failure', async () => {
    let rejectBrowse!: (reason: string) => void
    const request = useArchiveBrowseRequest(() => new Promise((_, reject) => { rejectBrowse = reject }), vi.fn().mockResolvedValue(undefined))
    const pending = request.run('C:/资料.lz')
    request.cancel()
    rejectBrowse('ARCHIVE_BROWSE_CORRUPTED|旧读取失败|detail')
    await expect(pending).rejects.toContain('旧读取失败')
    expect(request.notice.value).toBe('已取消读取压缩包内容')
  })
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
    expect(describeArchiveBrowseError('ARCHIVE_BROWSE_INCOMPLETE|缺少 ZIP 分卷：资料.haozip01.zip|detail')).toBe('缺少 ZIP 分卷：资料.haozip01.zip')
    expect(describeArchiveBrowseError(new Error('raw parser detail'))).toContain('确认文件完整')
  })
})
