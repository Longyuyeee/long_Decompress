import { expect, it } from 'vitest'
import { archiveOutputLocationError } from '../archiveOutputError'
it('distinguishes missing and denied paths without guessing unknown failures', () => {
  expect(archiveOutputLocationError('EXPLORER_PATH_UNAVAILABLE:not-found: OS detail')).toContain('已移动或删除')
  expect(archiveOutputLocationError('EXPLORER_PATH_UNAVAILABLE:permission-denied: OS detail')).toContain('没有权限')
  expect(archiveOutputLocationError('network error')).not.toContain('已移动或删除')
})
