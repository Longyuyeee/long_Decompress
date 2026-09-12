import { describe, expect, it } from 'vitest'
import { describeTaskFailure } from '../taskFailure'

describe('task failure evidence', () => {
  it('identifies output verification without claiming the archive is damaged', () => {
    expect(describeTaskFailure({ status: 'failed', errorMessage: '[archive-output:Verifying:verification-failed] engine unavailable' }))
      .toMatchObject({ category: 'verification-failed', stage: 'Verifying', evidence: 'recorded-marker' })
    expect(describeTaskFailure({ status: 'failed', failure: {
      schemaVersion: 1, category: 'verification-failed', stage: 'Verifying', evidence: 'recorded-marker',
    } })?.categoryLabel).toBe('压缩产物校验未通过')
    expect(describeTaskFailure({ status: 'cancelled', errorMessage: '[archive-output:Verifying:verification-failed]' })).toBeNull()
  })
  it.each(['source-missing', 'source-unavailable', 'source-invalid'])('preserves %s across both confirmed stages', category => {
    for (const stage of ['Pre-checking', 'Extracting']) {
      expect(describeTaskFailure({ status: 'failed', errorMessage: `[archive-source:${stage}:${category}] detail` }))
        .toMatchObject({ category, stage, evidence: 'recorded-marker' })
    }
  })
  it.each(['Pre-checking', 'Extracting'])('describes confirmed source changes during %s', stage => {
    expect(describeTaskFailure({ status: 'failed', errorMessage: `[archive-source:${stage}:source-changed] source changed` }))
      .toMatchObject({ category: 'source-changed', categoryLabel: '源文件已变化', stage, evidence: 'recorded-marker' })
    expect(describeTaskFailure({ status: 'failed', errorMessage: 'source changed', failure: {
      schemaVersion: 1, category: 'source-changed', stage, evidence: 'recorded-marker',
    } })?.stageLabel).not.toContain('最后观测')
  })
  it('does not infer a source-change stage from legacy text or unsupported markers', () => {
    for (const errorMessage of ['源压缩包仍在写入', '[archive-source:Publishing:source-changed]']) {
      expect(describeTaskFailure({ status: 'failed', errorMessage })).toMatchObject({ category: 'unknown', stage: 'unknown' })
    }
  })
  it('uses persisted stage and category without reparsing the message', () => {
    expect(describeTaskFailure({ status: 'failed', errorMessage: 'engine details', failure: {
      schemaVersion: 1, category: 'timeout', stage: 'inspection', evidence: 'recorded-marker',
    } })).toMatchObject({ category: 'timeout', stage: 'inspection' })
    expect(describeTaskFailure({ status: 'failed', errorMessage: 'output error', failure: {
      schemaVersion: 1, category: 'unknown', stage: 'Publishing', evidence: 'observed-stage',
    } })?.stageLabel).toBe('发布输出（最后观测阶段）')
  })
  it.each(['damaged', 'missing-volume', 'permission', 'timeout', 'engine-unavailable', 'engine-start', 'io', 'ambiguous-data'])('uses recorded %s classification', category => {
    expect(describeTaskFailure({ status: 'failed', errorMessage: `归档检测失败：[archive-inspection:${category}] raw detail` }))
      .toMatchObject({ category, stage: 'inspection', evidence: 'recorded-marker' })
  })
  it('keeps absent, legacy and future categories unknown', () => {
    expect(describeTaskFailure({ status: 'failed' })).toMatchObject({ category: 'unknown', stage: 'unknown', message: '原记录未保存具体失败原因' })
    expect(describeTaskFailure({ status: 'failed', errorMessage: '数据错误' })).toMatchObject({ category: 'unknown', stage: 'unknown' })
    expect(describeTaskFailure({ status: 'failed', errorMessage: '[archive-inspection:future-code] detail' })).toMatchObject({ category: 'unknown', stage: 'inspection' })
    expect(describeTaskFailure({ status: 'completed', errorMessage: '[archive-inspection:damaged]' })).toBeNull()
  })
})
