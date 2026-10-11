import { describe, expect, it } from 'vitest'
import { buildSafeTemplateDraftSettings, taskTemplateCandidatesToFiles } from '../taskTemplateDraft'
import type { CompressionProfile } from '@/types'

const profile = {
  id: 'logs',
  name: '日志归档',
  icon: '📦',
  description: '',
  config: {
    format: '7z',
    level: 7,
    password: 'must-not-copy',
    splitArchive: false,
    splitSize: null,
    keepStructure: true,
    deleteAfter: true,
    verifyAfter: true,
    createSolidArchive: true,
    filenameTemplate: '{name}-{date}-{time}',
    extraParams: { unsafe: 'must-not-copy' },
  },
  autoApply: { enabled: false, mode: 'pattern', filePatterns: ['*.log'], excludePatterns: [], sizeRange: null },
  passwordStrategy: { type: 'fixed' },
  stats: { useCount: 0, successCount: 0, failureCount: 0, totalFilesProcessed: 0, totalBytesProcessed: 0 },
  createdAt: 0,
  lastUsedAt: null,
} satisfies CompressionProfile

describe('safe task-template drafts', () => {
  it('preserves filename encryption but requires a fresh runtime password', () => {
    expect(buildSafeTemplateDraftSettings({ ...profile, config: { ...profile.config, encryptFilenames: true } }, []))
      .toMatchObject({ encryptFilenames: true, password: '', deleteAfter: false })
    expect(buildSafeTemplateDraftSettings(profile, []).encryptFilenames).toBe(false)
  })
  it('restores explicit ZIP encoding and split settings without supplying a password', () => {
    const imported: CompressionProfile = { ...profile, config: {
      ...profile.config, format: 'zip', zipPasswordEncoding: 'windows_local',
      splitArchive: true, splitSize: 128, createSolidArchive: false,
    } }
    expect(buildSafeTemplateDraftSettings(imported, [])).toMatchObject({
      zipPasswordEncoding: 'windows_local', splitArchive: true, splitSize: '128', password: '', deleteAfter: false,
    })
    expect(buildSafeTemplateDraftSettings(profile, [])).not.toHaveProperty('zipPasswordEncoding')
  })

  it('never carries passwords, source deletion, or extra engine parameters', () => {
    const candidates = [{ path: 'C:/logs/app.log', name: 'app.log', size: 12, isDirectory: false }]
    const settings = buildSafeTemplateDraftSettings(profile, candidates, new Date('2026-08-10T01:02:03'))
    expect(settings).toMatchObject({
      format: '7z',
      password: '',
      deleteAfter: false,
      filename: 'app-2026-08-10-010203',
    })
    expect(settings).not.toHaveProperty('extraParams')
    expect(taskTemplateCandidatesToFiles(candidates)).toEqual([
      expect.objectContaining({ path: 'C:/logs/app.log', isDirectory: false }),
    ])
  })
})
