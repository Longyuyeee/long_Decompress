import { describe, it, expect } from 'vitest'
import { splitSizeBytes } from '../archiveSizeUnits'

describe('compression split size IPC units', () => {
  it.each([1, 128, 1024])('converts %i MiB without changing stored settings', size => {
    expect(splitSizeBytes(true, String(size))).toBe(size * 1024 * 1024)
    expect(splitSizeBytes(true, size)).toBe(size * 1024 * 1024)
  })
  it('sends no split size when disabled', () => {
    expect(splitSizeBytes(false, '128')).toBeNull()
  })
})
