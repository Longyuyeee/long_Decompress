import type { CompressionOptions, FileObject } from '@/stores/compression'
import type { CompressionProfile, TaskTemplateDraftCandidate } from '@/types'

const resolveDraftFilename = (
  profile: CompressionProfile,
  candidates: TaskTemplateDraftCandidate[],
  now = new Date(),
) => {
  const firstName = candidates[0]?.name.replace(/\.[^/.]+$/, '') || profile.name
  const sourceName = candidates.length === 1 ? firstName : profile.name
  const date = [now.getFullYear(), now.getMonth() + 1, now.getDate()]
    .map((value, index) => String(value).padStart(index === 0 ? 4 : 2, '0'))
    .join('-')
  const time = [now.getHours(), now.getMinutes(), now.getSeconds()]
    .map(value => String(value).padStart(2, '0'))
    .join('')
  return (profile.config.filenameTemplate || sourceName)
    .replaceAll('{name}', sourceName)
    .replaceAll('{date}', date)
    .replaceAll('{time}', time)
}

export const buildSafeTemplateDraftSettings = (
  profile: CompressionProfile,
  candidates: TaskTemplateDraftCandidate[],
  now = new Date(),
): CompressionOptions => ({
  format: profile.config.format as CompressionOptions['format'],
  level: profile.config.level,
  password: '',
  filename: resolveDraftFilename(profile, candidates, now),
  splitArchive: profile.config.splitArchive,
  splitSize: String(profile.config.splitSize || 1024),
  keepStructure: profile.config.keepStructure,
  deleteAfter: false,
  verifyAfter: profile.config.verifyAfter,
  createSolidArchive: profile.config.createSolidArchive,
})

export const taskTemplateCandidatesToFiles = (
  candidates: TaskTemplateDraftCandidate[],
): FileObject[] => candidates.map(candidate => ({
  name: candidate.name,
  path: candidate.path,
  size: candidate.size,
  type: candidate.isDirectory ? 'directory' : 'file',
  isDirectory: candidate.isDirectory,
}))
