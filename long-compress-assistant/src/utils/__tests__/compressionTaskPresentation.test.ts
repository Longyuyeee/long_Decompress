import { describe, expect, it } from 'vitest'
import type { Task, TaskStatus } from '@/stores/task'
import {
  compressionLogSeverityClass,
  compressionStageTranslationKey,
  compressionStatusClass,
  compressionStatusIcon,
  compressionStatusTranslationKey,
  emptyCompressionLogTranslationKey,
  isActiveCompressionStatus,
  isFinishedCompressionStatus,
  showsCompressionProgress,
} from '@/utils/compressionTaskPresentation'

const task = (status: TaskStatus = 'pending', stage?: string) => ({
  id: 'task-1',
  name: 'sample.zip',
  type: 'compression',
  status,
  progress: 0,
  logs: [],
  sourceFiles: ['C:/input/sample.txt'],
  outputPath: 'C:/output/sample.zip',
  conflicts: [],
  stage,
} as Task)

describe('compression task presentation', () => {
  it('separates active and terminal states', () => {
    expect(isActiveCompressionStatus()).toBe(false)
    expect(isFinishedCompressionStatus()).toBe(false)

    for (const status of ['pending', 'preparing', 'running', 'compressing', 'finalizing', 'cancelling'] as TaskStatus[]) {
      expect(isActiveCompressionStatus(status)).toBe(true)
      expect(isFinishedCompressionStatus(status)).toBe(false)
    }

    for (const status of ['completed', 'failed', 'cancelled'] as TaskStatus[]) {
      expect(isActiveCompressionStatus(status)).toBe(false)
      expect(isFinishedCompressionStatus(status)).toBe(true)
    }
  })

  it('maps statuses and engine stages to translation keys', () => {
    expect(compressionStatusTranslationKey()).toBe('compress.status.pending')
    expect(compressionStatusTranslationKey('completed')).toBe('compress.status.completed')
    expect(compressionStageTranslationKey()).toBe('compress.status.pending')
    expect(compressionStageTranslationKey(task('running'))).toBe('compress.status.running')
    expect(compressionStageTranslationKey(task('running', 'Finalizing'))).toBe('compress.status.finalizing')
    expect(compressionStageTranslationKey(task('running', 'Compressing files'))).toBe('compress.status.compressing')
    expect(compressionStageTranslationKey(task('running', 'Writing archive'))).toBe('compress.status.compressing')
    expect(compressionStageTranslationKey(task('running', 'Pre-checking'))).toBe('compress.status.preparing')
    expect(compressionStageTranslationKey(task('running', 'Extracting'))).toBe('compress.status.running')
  })

  it('maps every status to a stable color, icon, and progress policy', () => {
    expect(compressionStatusClass()).toBe('text-muted')
    expect(compressionStatusIcon()).toBe('pi-clock')

    expect(compressionStatusClass('completed')).toBe('text-green-500')
    expect(compressionStatusIcon('completed')).toBe('pi-check-circle')
    expect(compressionStatusClass('failed')).toBe('text-red-500')
    expect(compressionStatusIcon('failed')).toBe('pi-exclamation-circle')
    expect(compressionStatusClass('cancelled')).toBe('text-orange-500')
    expect(compressionStatusIcon('cancelled')).toBe('pi-ban')
    expect(compressionStatusClass('cancelling')).toBe('text-orange-400')
    expect(compressionStatusIcon('cancelling')).toBe('pi-spin pi-spinner')

    for (const status of ['preparing', 'running', 'compressing', 'finalizing'] as TaskStatus[]) {
      expect(compressionStatusClass(status)).toBe('text-primary')
      expect(compressionStatusIcon(status)).toBe('pi-spin pi-spinner')
      expect(showsCompressionProgress(status)).toBe(true)
    }

    expect(showsCompressionProgress('cancelling')).toBe(true)
    expect(showsCompressionProgress('pending')).toBe(false)
    expect(showsCompressionProgress('completed')).toBe(false)
  })

  it('maps log severities and empty states without leaking view logic', () => {
    expect(compressionLogSeverityClass('error')).toBe('text-red-400')
    expect(compressionLogSeverityClass('warning')).toBe('text-yellow-400')
    expect(compressionLogSeverityClass('success')).toBe('text-green-400')
    expect(compressionLogSeverityClass('info')).toBe('text-muted')
    expect(emptyCompressionLogTranslationKey()).toBe('compress.pending_log')
    expect(emptyCompressionLogTranslationKey(task())).toBe('compress.pending_log')
    expect(emptyCompressionLogTranslationKey(task('running'))).toBe('compress.no_logs')
  })
})
