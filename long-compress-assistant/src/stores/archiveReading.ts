import {defineStore} from 'pinia'
import {ref} from 'vue'
import {READING_KEY, copyReadingRecord, readingKey, type ArchiveReadingRecord} from '@/types/archiveReading'
import type {ZpaqSnapshot} from '@/types/zpaqSnapshot'

export const useArchiveReadingStore = defineStore('archiveReading', () => {
  const records = ref<ArchiveReadingRecord[]>([]), notice = ref('')
  try {
    const saved = JSON.parse(localStorage.getItem(READING_KEY) || 'null')
    if (saved?.version === 1 && Array.isArray(saved.records)) {
      const seen = new Set<string>()
      records.value = saved.records.slice(0, 30).flatMap((value: unknown) => {
        const row = copyReadingRecord(value)
        if (!row || seen.has(readingKey(row.filePath, row.snapshot))) return []
        seen.add(readingKey(row.filePath, row.snapshot)); return [row]
      })
    }
  } catch { notice.value = '上次阅读记录暂时无法读取，可以直接打开压缩包继续看。' }
  function persist() {
    try { localStorage.setItem(READING_KEY, JSON.stringify({version: 1, records: records.value})); notice.value = '' }
    catch { notice.value = '阅读位置暂未保存，请检查本地存储空间；本次阅读仍可继续。' }
  }
  function find(path: string, snapshot?: ZpaqSnapshot) { return records.value.find(row => readingKey(row.filePath, row.snapshot) === readingKey(path, snapshot)) }
  function save(value: ArchiveReadingRecord) {
    const row = copyReadingRecord(value); if (!row) return
    records.value = [row, ...records.value.filter(old => readingKey(old.filePath, old.snapshot) !== readingKey(row.filePath, row.snapshot))].slice(0, 30)
    persist()
  }
  function remove(row: ArchiveReadingRecord) { records.value = records.value.filter(old => readingKey(old.filePath, old.snapshot) !== readingKey(row.filePath, row.snapshot)); persist() }
  return {records, notice, find, save, remove}
})
