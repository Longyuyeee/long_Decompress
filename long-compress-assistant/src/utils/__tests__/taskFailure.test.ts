import { describe, expect, it } from 'vitest'
import { describeTaskFailure } from '../taskFailure'

describe('task failure evidence', () => {
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
