import { createHash } from 'node:crypto'
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex')

// Prepare our own inputs only. No encoder is launched, no archive is created,
// and this manifest must never be counted as genuine HaoZip output evidence.
export function prepareHaozipForumFixture(parent = path.join(projectRoot, 'test-results')) {
  mkdirSync(parent, { recursive: true })
  const directory = mkdtempSync(path.join(parent, 'haozip-forum-manual-'))
  const source = path.join(directory, '原文件')
  const output = path.join(directory, '好压生成的论坛卷')
  mkdirSync(path.join(source, '中文目录', '空目录'), { recursive: true })
  mkdirSync(output)
  const payload = Buffer.alloc(163871)
  // Deterministic, non-repeating blocks keep normal compression from turning
  // the source into a single tiny volume; every computer gets the same bytes.
  for (let offset = 0; offset < payload.length; offset += 32) {
    const block = createHash('sha256').update(`Long HaoZip forum fixture block ${offset / 32}`).digest()
    block.copy(payload, offset, 0, Math.min(block.length, payload.length - offset))
  }
  const contents = [
    ['说明.txt', Buffer.from('我收到好压的加密论坛分卷，从第二卷打开，先输错再改密码，然后解压、转换并补卷重试。\n')],
    ['中文目录/原文件.bin', payload],
  ]
  const manifest = {
    schemaVersion: 1,
    fixtureKind: 'manual-HaoZip-forum-creation-inputs-only',
    status: 'awaiting-manual-creation',
    sourceDirectory: source,
    archiveOutputDirectory: output,
    suggestedArchiveName: '论坛密码验收.zip',
    suggestedVolumeBytes: 65536,
    testPassword: 'ForumTest-1411',
    expectedFiles: contents.map(([name, bytes]) => {
      writeFileSync(path.join(source, name), bytes, { flag: 'wx' })
      return { path: name, bytes: bytes.length, sha256: sha256(bytes) }
    }),
    emptyDirectories: ['中文目录/空目录'],
    generatedForumVolumes: [],
  }
  const manifestPath = path.join(directory, 'fixture-inputs.json')
  writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' })
  return { directory, manifestPath, manifest }
}

if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) {
  const { directory, manifestPath, manifest } = prepareHaozipForumFixture()
  console.log(JSON.stringify({ directory, manifestPath, ...manifest }, null, 2))
}
