import { describe, expect, it } from 'vitest'
import { createContextCompressionEntry, createQuickPackCandidate, createQuickPackPlan, groupContextActions } from '../contextActions'

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

  it('groups quick-pack launches only within the same Explorer directory', () => {
    expect(groupContextActions([
      { action: 'context-quick-pack', files: ['C:/work/one.txt'] },
      { action: 'context-quick-pack', files: ['C:/work/two.txt'] },
      { action: 'context-quick-pack', files: ['D:/other/three.txt'] },
    ])).toEqual([
      { action: 'context-quick-pack', files: ['C:/work/one.txt', 'C:/work/two.txt'] },
      { action: 'context-quick-pack', files: ['D:/other/three.txt'] },
    ])
  })

  it('builds a quick-pack name and output directory from the selection', () => {
    expect(createQuickPackPlan(['C:\\Users\\me\\Downloads\\one.txt', 'C:\\Users\\me\\Downloads\\two.txt']))
      .toEqual({ outputDirectory: 'C:\\Users\\me\\Downloads', archiveName: 'Downloads' })
    expect(createQuickPackPlan(['C:/work/report.final.txt']))
      .toEqual({ outputDirectory: 'C:/work', archiveName: 'report.final' })
  })

  it('builds non-destructive names when a quick-pack output already exists', () => {
    const plan = { outputDirectory: 'C:\\work', archiveName: 'work' }
    expect(createQuickPackCandidate(plan)).toEqual({
      archiveName: 'work',
      outputPath: 'C:\\work\\work.zip',
    })
    expect(createQuickPackCandidate(plan, 2)).toEqual({
      archiveName: 'work (2)',
      outputPath: 'C:\\work\\work (2).zip',
    })
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
