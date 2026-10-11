import { describe, expect, it } from 'vitest'
import { createTaskHistoryRecord } from '../taskHistory'
import type { Task } from '@/stores/task'

describe('createTaskHistoryRecord', () => {
  it('keeps source metrics separate from volume copying and discards failed output metrics', () => {
    const task: Task = {
      id: 'split', name: 'archive.zip', type: 'compression', status: 'completed', progress: 100,
      sourceFiles: ['C:/source'], outputPath: 'C:/archive.zip', conflicts: [], logs: [],
      compressionSourceProgress: { processedBytes: 10000, totalBytes: 10000 },
      processedBytes: 1000, totalBytes: 1000, outputBytes: 1000,
    }
    const record = createTaskHistoryRecord(task)
    expect(record).toMatchObject({ processedBytes: 10000, totalBytes: 10000 })
    expect(record.metrics).toMatchObject({ inputBytes: 10000, outputBytes: 1000, savingsRatio: 0.9 })
    for (const status of ['cancelled', 'failed'] as const) {
      expect(createTaskHistoryRecord({ ...task, status }).metrics).toBeNull()
    }
    expect(createTaskHistoryRecord({ ...task, compressionSourceProgress: { processedBytes: 0, totalBytes: 0 } }).metrics)
      .toMatchObject({ inputBytes: 0, outputBytes: 1000, savingsRatio: 0 })
  })

  it('keeps operational metadata and excludes password fields', () => {
    const task: Task = {
      id: 'task-1', name: 'archive.zip', type: 'compression', status: 'completed', progress: 100,
      sourceFiles: ['C:/source'], outputPath: 'C:/archive.zip', format: 'zip', conflicts: [],
      startTime: new Date('2026-08-20T00:00:00.000Z'), endTime: new Date('2026-08-20T00:00:02.000Z'),
      processedBytes: 1200, totalBytes: 1200, outputBytes: 600, password: 'must-not-persist', currentPassword: 'attempt-secret',
      compressionOptions: { level: 6, password: 'another-secret' },
      logs: [{ task_id: 'task-1', timestamp: '2026-08-20T00:00:02.000Z', message: '完成', severity: 'success' }],
    }
    const record = createTaskHistoryRecord(task)
    expect(record.durationMs).toBe(2000)
    expect(record.processedBytes).toBe(1200)
    expect(record.workloadKind).toBe('archive')
    expect(record.metrics).toEqual({
      schemaVersion: 1,
      inputBytes: 1200,
      outputBytes: 600,
      savingsRatio: 0.5,
    })
    expect(JSON.stringify(record)).not.toContain('must-not-persist')
    expect(JSON.stringify(record)).not.toContain('attempt-secret')
    expect(JSON.stringify(record)).not.toContain('another-secret')
  })

  it('does not persist estimated output as a measured metric', () => {
    const task: Task = {
      id: 'task-estimate', name: 'archive.zip', type: 'compression', status: 'completed', progress: 100,
      sourceFiles: ['C:/source'], outputPath: 'C:/archive.zip', conflicts: [], logs: [],
      totalBytes: 1200, outputBytes: 500, outputBytesEstimated: true,
    }
    expect(createTaskHistoryRecord(task).metrics).toBeNull()
  })
})
