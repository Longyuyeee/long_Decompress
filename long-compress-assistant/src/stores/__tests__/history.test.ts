import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { HISTORY_WINDOW_LIMIT, useHistoryStore } from '../history'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
const row = (id: string) => ({ id, completedAt: '2026-09-23T00:00:00Z', status: 'failed' })
const cursor = { completedAt: '2026-09-23T00:00:00Z', id: 'b' }
describe('history pagination', () => {
  it('bounds the window while I browse 1200 records and can return to the newest without losing any history', async () => {
    const database = Array.from({ length: 1200 }, (_, i) => row(`record-${String(i).padStart(4, '0')}`)).reverse()
    mocks.invoke.mockImplementation(async (_command, args) => {
      let matches = database.filter(record => !args.cursor || (args.newer ? record.id > args.cursor.id : record.id < args.cursor.id))
      if (args.newer) matches = matches.reverse()
      const records = matches.slice(0, args.limit)
      const last = records.at(-1)
      const nextCursor = matches.length > args.limit && last ? { completedAt: last.completedAt, id: last.id } : null
      return { records: args.newer ? records.reverse() : records, nextCursor, logsOmitted: true }
    })
    const store = useHistoryStore()
    const seen = new Set<string>()
    await store.fetchHistory()
    do {
      store.records.forEach(record => seen.add(record.id))
      expect(store.records.length).toBeLessThanOrEqual(HISTORY_WINDOW_LIMIT)
      if (!store.hasMore) break
      await store.loadMore()
    } while (true)
    expect(seen.size).toBe(1200)
    expect(store.hasNewer).toBe(true)
    while (store.hasNewer) {
      await store.loadNewer()
      expect(store.records.length).toBeLessThanOrEqual(HISTORY_WINDOW_LIMIT)
    }
    expect(store.records[0].id).toBe('record-1199')
    expect(store.records.at(-1)?.id).toBe('record-0700')
    expect(store.hasMore).toBe(true)
    expect(database).toHaveLength(1200)
    expect(mocks.invoke.mock.calls.every(([command]) => command === 'list_task_history_page')).toBe(true)
  })
  it('搜索条件变化后丢弃旧响应，下一页沿用当前条件', async () => {
    const store = useHistoryStore()
    let resolve!: (page: unknown) => void
    mocks.invoke.mockImplementationOnce(() => new Promise(done => { resolve = done }))
    const old = store.fetchHistory()
    store.prepareQuery({ text: '旧照片', status: 'completed' })
    mocks.invoke.mockResolvedValueOnce({ records: [row('b')], nextCursor: cursor })
    await store.fetchHistory()
    resolve({ records: [row('unrelated')], nextCursor: null })
    await old
    expect(store.records.map(r => r.id)).toEqual(['b'])
    mocks.invoke.mockResolvedValueOnce({ records: [row('a')], nextCursor: null })
    await store.loadMore()
    expect(mocks.invoke).toHaveBeenLastCalledWith('list_task_history_page', {
      limit: 100, cursor, query: { text: '旧照片', status: 'completed' }, omitLogs: true,
    })
  })
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
    expect(mocks.invoke).toHaveBeenLastCalledWith('list_task_history_page', { limit: 100, cursor, query: { text: '' }, omitLogs: true })
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
