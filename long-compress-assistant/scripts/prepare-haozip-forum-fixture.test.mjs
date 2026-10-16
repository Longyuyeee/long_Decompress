import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdtempSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import test from 'node:test'
import { deflateRawSync } from 'node:zlib'
import { prepareHaozipForumFixture } from './prepare-haozip-forum-fixture.mjs'

test('prepares deterministic multi-volume-sized inputs without claiming encoder output', () => {
  const parent = mkdtempSync(path.join(tmpdir(), 'long-haozip-input-test-'))
  const first = prepareHaozipForumFixture(parent)
  const second = prepareHaozipForumFixture(parent)
  assert.notEqual(first.directory, second.directory)
  assert.deepEqual(first.manifest.expectedFiles, second.manifest.expectedFiles)
  assert.equal(first.manifest.status, 'awaiting-manual-creation')
  assert.equal(first.manifest.fixtureKind, 'manual-HaoZip-forum-creation-inputs-only')
  assert.deepEqual(first.manifest.generatedForumVolumes, [])
  assert.deepEqual(readdirSync(first.manifest.archiveOutputDirectory), [])
  assert.ok(first.manifest.expectedFiles.find(file => file.path.endsWith('.bin')).bytes > 2 * first.manifest.suggestedVolumeBytes)
  for (const file of first.manifest.expectedFiles) {
    const actual = readFileSync(path.join(first.manifest.sourceDirectory, file.path))
    assert.equal(actual.length, file.bytes)
    assert.equal(createHash('sha256').update(actual).digest('hex'), file.sha256)
    if (file.path.endsWith('.bin')) assert.ok(deflateRawSync(actual).length > 2 * first.manifest.suggestedVolumeBytes)
  }
  assert.ok(statSync(path.join(first.manifest.sourceDirectory, first.manifest.emptyDirectories[0])).isDirectory())
  assert.deepEqual(JSON.parse(readFileSync(first.manifestPath, 'utf8')), first.manifest)
})

test('a repeated preparation never overwrites an earlier input or output', () => {
  const parent = mkdtempSync(path.join(tmpdir(), 'long-haozip-preserve-test-'))
  const first = prepareHaozipForumFixture(parent)
  const earlier = path.join(first.manifest.archiveOutputDirectory, 'existing.haozip01.zip')
  writeFileSync(earlier, 'existing encoder result')
  const original = readFileSync(first.manifestPath, 'utf8')
  prepareHaozipForumFixture(parent)
  assert.equal(readFileSync(earlier, 'utf8'), 'existing encoder result')
  assert.equal(readFileSync(first.manifestPath, 'utf8'), original)
})
