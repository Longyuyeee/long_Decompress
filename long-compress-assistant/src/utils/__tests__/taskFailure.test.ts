import { describe, expect, it } from 'vitest'
import { describeTaskFailure } from '../taskFailure'

describe('task failure evidence', () => {
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
