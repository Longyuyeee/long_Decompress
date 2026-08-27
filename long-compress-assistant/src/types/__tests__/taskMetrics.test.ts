import { describe, expect, it } from 'vitest'
import { createMeasuredTaskMetricsV1 } from '../taskMetrics'

describe('createMeasuredTaskMetricsV1', () => {
  it('derives final savings only from normalized filesystem byte facts', () => {
    expect(createMeasuredTaskMetricsV1(1_000.4, 600.4)).toEqual({
      schemaVersion: 1,
      inputBytes: 1000,
      outputBytes: 600,
      savingsRatio: 0.4,
    })
  })

  it('does not serialize non-finite or negative values as measured facts', () => {
    expect(createMeasuredTaskMetricsV1(Number.POSITIVE_INFINITY, Number.NaN)).toEqual({
      schemaVersion: 1,
      inputBytes: 0,
      outputBytes: 0,
      savingsRatio: 0,
    })
    expect(createMeasuredTaskMetricsV1(-1, -20).inputBytes).toBe(0)
  })
})
