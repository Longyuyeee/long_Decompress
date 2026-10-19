import { describe, expect, it } from 'vitest'
import { describeTaskFailure, getTaskRecoveryGuidance, archiveInspectionMessageForDisplay } from '../taskFailure'
import { isPasswordRelatedError } from '..'

describe('processing I/O failures', () => {
  it.each(['disk-full', 'target-busy', 'permission', 'io'] as const)('preserves %s without a guessed inspection stage or password retry', category => {
    const errorMessage = `[archive-io:${category}] writing password.zip: original OS detail`
    expect(describeTaskFailure({ status: 'failed', errorMessage })).toMatchObject({ category, stage: 'unknown' })
    expect(isPasswordRelatedError(errorMessage + '; wrong password from nested detail')).toBe(false)
    expect(archiveInspectionMessageForDisplay(errorMessage)).not.toContain('[archive-io:')
    expect(getTaskRecoveryGuidance({ status: 'failed', errorMessage })?.steps.join('')).toContain('新任务')
  })
  it('keeps incomplete rollback ahead of any I/O category', () => {
    const record = { status: 'failed' as const, errorMessage: '[archive-output:Publishing:rollback-incomplete] [archive-io:disk-full] backup retained' }
    expect(describeTaskFailure(record)?.category).toBe('rollback-incomplete')
  })
  it('does not treat damaged content as a disk failure or rewrite a completed task', () => {
    expect(describeTaskFailure({ status: 'failed', errorMessage: '[archive-inspection:damaged] EOF' })?.category).toBe('damaged')
    expect(describeTaskFailure({ status: 'completed', errorMessage: '[archive-io:disk-full]' })).toBeNull()
  })
})
