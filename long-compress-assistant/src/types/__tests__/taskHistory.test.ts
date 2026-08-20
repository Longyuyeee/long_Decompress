import { describe, expect, it } from 'vitest'
import { createTaskHistoryRecord } from '../taskHistory'
import type { Task } from '@/stores/task'

describe('createTaskHistoryRecord', () => {
  it('keeps operational metadata and excludes password fields', () => {
    const task: Task = {
      id: 'task-1', name: 'archive.zip', type: 'compression', status: 'completed', progress: 100,
      sourceFiles: ['C:/source'], outputPath: 'C:/archive.zip', format: 'zip', conflicts: [],
      startTime: new Date('2026-08-20T00:00:00.000Z'), endTime: new Date('2026-08-20T00:00:02.000Z'),
      processedBytes: 1200, totalBytes: 1200, password: 'must-not-persist', currentPassword: 'attempt-secret',
      compressionOptions: { level: 6, password: 'another-secret' },
      logs: [{ task_id: 'task-1', timestamp: '2026-08-20T00:00:02.000Z', message: '完成', severity: 'success' }],
    }
    const record = createTaskHistoryRecord(task)
    expect(record.durationMs).toBe(2000)
    expect(record.processedBytes).toBe(1200)
    expect(JSON.stringify(record)).not.toContain('must-not-persist')
    expect(JSON.stringify(record)).not.toContain('attempt-secret')
    expect(JSON.stringify(record)).not.toContain('another-secret')
  })
})
