import { describe, expect, it } from 'vitest'
import { describeTaskFailure, getTaskRecoveryGuidance } from '../taskFailure'

describe('task failure evidence', () => {
  it.each([
    ['damaged', 'inspection', '重新获取完整文件'],
    ['missing-volume', 'inspection', '不猜测缺失分卷名称'],
    ['permission', 'inspection', '来源读取'],
    ['permission', 'Publishing', '输出目录'],
    ['permission', 'Extracting', '无法仅凭阶段确定'],
    ['output-conflict', 'Publishing', '不要覆盖'],
    ['unknown', 'unknown', '导出诊断报告'],
  ])('失败分类 %s / %s 给出有依据的下一步', (category, stage, expected) => {
    const record = { status: 'failed' as const, errorMessage: '原始错误保持不变', failure: {
      schemaVersion: 1 as const, category, stage, evidence: 'recorded-marker' as const,
    } }
    const before = JSON.stringify(record)
    expect(getTaskRecoveryGuidance(record)?.steps.join(' ')).toContain(expected)
    expect(JSON.stringify(record)).toBe(before)
  })
  it('仅观测到发布阶段不能把权限错误断定为输出权限', () => {
    expect(getTaskRecoveryGuidance({ status: 'failed', failure: {
      schemaVersion: 1, category: 'permission', stage: 'Publishing', evidence: 'observed-stage',
    } })?.steps.join(' ')).toContain('无法仅凭阶段确定')
  })
  it('旧记录和未来分类给出未知指引，不从自然语言猜测损坏', () => {
    for (const errorMessage of ['文件损坏', '[archive-inspection:future-code]']) {
      expect(getTaskRecoveryGuidance({ status: 'failed', errorMessage })?.title).toContain('原因尚未确定')
    }
    expect(getTaskRecoveryGuidance({ status: 'failed', errorMessage: '[archive-inspection:damaged]' })?.title).toContain('损坏')
  })
  it('reserves rollback-specific guidance for confirmed rollback failures', () => {
    const failure = { schemaVersion: 1 as const, category: 'rollback-incomplete', stage: 'Publishing', evidence: 'recorded-marker' as const }
    expect(getTaskRecoveryGuidance({ status: 'failed', failure })?.steps.join(' ')).toContain('旧记录缺少恢复路径时不要猜测目录')
    expect(getTaskRecoveryGuidance({ status: 'completed', failure })).toBeNull()
    expect(getTaskRecoveryGuidance({ status: 'failed', errorMessage: '回滚失败' })?.title).toContain('原因尚未确定')
    expect(getTaskRecoveryGuidance({ status: 'failed', errorMessage: '[archive-output:Publishing:rollback-incomplete]' })).not.toBeNull()
  })
  it.each(['output-conflict', 'publication-failed', 'rollback-incomplete'] as const)('describes confirmed %s without guessing corruption', category => {
    expect(describeTaskFailure({ status: 'failed', errorMessage: `[archive-output:Publishing:${category}] detail` }))
      .toMatchObject({ category, stage: 'Publishing', evidence: 'recorded-marker' })
    expect(describeTaskFailure({ status: 'completed', errorMessage: `[archive-output:Publishing:${category}]` })).toBeNull()
  })
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
