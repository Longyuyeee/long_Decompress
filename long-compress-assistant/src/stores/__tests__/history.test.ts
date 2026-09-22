import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useHistoryStore } from '../history'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
const row = (id: string) => ({ id, completedAt: '2026-09-23T00:00:00Z', status: 'failed' })
const cursor = { completedAt: '2026-09-23T00:00:00Z', id: 'b' }
describe('history pagination', () => {
  beforeEach(() => { setActivePinia(createPinia()); vi.resetAllMocks() })
  it('加载更早记录并保留第一页；失败可以重试且不重复记录', async () => {
    const store = useHistoryStore()
    mocks.invoke.mockResolvedValueOnce({ records: [row('b')], nextCursor: cursor })
    await store.fetchHistory()
    mocks.invoke.mockRejectedValueOnce(new Error('数据库暂时不可用'))
    await expect(store.loadMore()).rejects.toThrow()
    expect(store.records.map(r => r.id)).toEqual(['b'])
    expect(store.hasMore).toBe(true)
    mocks.invoke.mockResolvedValueOnce({ records: [row('a')], nextCursor: null })
    await store.loadMore()
    expect(store.records.map(r => r.id)).toEqual(['b', 'a'])
    expect(store.hasMore).toBe(false)
    expect(mocks.invoke).toHaveBeenLastCalledWith('list_task_history_page', { limit: 100, cursor })
  })
  it('刷新期间旧的加载更多响应不能覆盖新列表', async () => {
    const store = useHistoryStore()
    mocks.invoke.mockResolvedValueOnce({ records: [row('b')], nextCursor: cursor })
    await store.fetchHistory()
    let resolve!: (page: unknown) => void
    mocks.invoke.mockImplementationOnce(() => new Promise(done => { resolve = done }))
    const loading = store.loadMore()
    mocks.invoke.mockResolvedValueOnce({ records: [row('c')], nextCursor: null })
    await store.fetchHistory()
    resolve({ records: [row('a')], nextCursor: null })
    await loading
    expect(store.records.map(r => r.id)).toEqual(['c'])
  })
  it('清空期间旧查询不能把已删除的历史加回来', async () => {
    const store = useHistoryStore()
    let resolve!: (page: unknown) => void
    mocks.invoke.mockImplementationOnce(() => new Promise(done => { resolve = done }))
    const loading = store.fetchHistory()
    mocks.invoke.mockResolvedValueOnce(undefined)
    await store.clearHistory()
    resolve({ records: [row('a')], nextCursor: cursor })
    await loading
    expect(store.records).toEqual([])
    expect(store.hasMore).toBe(false)
    expect(store.isLoading).toBe(false)
  })
  it('单条删除后，正在返回的下一页不能复活该记录', async () => {
    const store = useHistoryStore()
    mocks.invoke.mockResolvedValueOnce({ records: [row('b')], nextCursor: cursor })
    await store.fetchHistory()
    let resolve!: (page: unknown) => void
    mocks.invoke.mockImplementationOnce(() => new Promise(done => { resolve = done }))
    const loading = store.loadMore()
    await store.loadMore()
    expect(mocks.invoke).toHaveBeenCalledTimes(2)
    mocks.invoke.mockResolvedValueOnce(undefined)
    await store.deleteRecord('b')
    resolve({ records: [row('b'), row('a')], nextCursor: null })
    await loading
    expect(store.records.map(r => r.id)).toEqual(['a'])
  })
  it('清空失败不能丢掉列表或更早页游标', async () => {
    const store = useHistoryStore()
    mocks.invoke.mockResolvedValueOnce({ records: [row('b')], nextCursor: cursor })
    await store.fetchHistory()
    mocks.invoke.mockRejectedValueOnce(new Error('read-only database'))
    await expect(store.clearHistory()).rejects.toThrow('read-only')
    expect(store.records.map(r => r.id)).toEqual(['b'])
    expect(store.hasMore).toBe(true)
  })
})
