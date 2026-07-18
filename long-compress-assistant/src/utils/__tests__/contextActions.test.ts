import { describe, expect, it } from 'vitest'
import { createContextCompressionEntry, groupContextActions } from '../contextActions'

describe('context action helpers', () => {
  it('merges repeated shell invocations and removes duplicate paths', () => {
    expect(groupContextActions([
      { action: 'context-compress-zip', files: ['C:/one.txt'] },
      { action: 'context-compress-zip', files: ['C:/two.txt', 'C:/one.txt'] },
      { action: 'context-extract-here', files: ['C:/archive.zip'] },
    ])).toEqual([
      { action: 'context-compress-zip', files: ['C:/one.txt', 'C:/two.txt'] },
      { action: 'context-extract-here', files: ['C:/archive.zip'] },
    ])
  })

  it('preserves directory metadata for right-click compression', () => {
    expect(createContextCompressionEntry('C:/Project', { size: 4096, is_dir: true })).toEqual({
      name: 'Project',
      path: 'C:/Project',
      size: 4096,
      type: 'folder',
      isDirectory: true,
    })
  })
})
