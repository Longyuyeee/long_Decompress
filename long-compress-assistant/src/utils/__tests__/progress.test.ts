import { describe, expect, it } from 'vitest'
import { formatProgressPercent, normalizeProgressPercent } from '@/utils/progress'

describe('progress presentation', () => {
  it('preserves two decimal places for active archive progress', () => {
    expect(normalizeProgressPercent(0.0001)).toBe(0.01)
    expect(normalizeProgressPercent(0.123456)).toBe(12.35)
    expect(formatProgressPercent(12.35)).toBe('12.35')
    expect(formatProgressPercent(1)).toBe('1.00')
  })

  it('keeps boundary values concise and bounded', () => {
    expect(normalizeProgressPercent(-1)).toBe(0)
    expect(normalizeProgressPercent(2)).toBe(100)
    expect(formatProgressPercent(0)).toBe('0')
    expect(formatProgressPercent(100)).toBe('100')
  })
})
