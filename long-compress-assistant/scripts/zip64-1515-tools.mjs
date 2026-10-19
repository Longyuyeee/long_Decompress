import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { spawn, spawnSync } from 'node:child_process'
import { closeSync, createReadStream, existsSync, openSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const work = path.join(root, 'test-results/zip64-1515')
const manifest = JSON.parse(readFileSync(path.join(work, 'source.json'), 'utf8'))
const tool = path.join(root, 'src-tauri/resources/archive-engine/7z.exe')
const password = 'ZIP64-test-password + !'
const sha = async file => {
  const h = createHash('sha256')
  for await (const b of createReadStream(file)) h.update(b)
  return h.digest('hex')
}
const identity = {
  path: tool, sha256: await sha(tool),
  banner: spawnSync(tool, ['i'], { encoding: 'utf8', windowsHide: true }).stdout.split(/\r?\n/).filter(Boolean).slice(0, 3),
}
const run = (args, cwd) => new Promise((resolve, reject) => {
  const child = spawn(tool, args, { cwd, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] })
  let text = ''
  child.stdout.on('data', b => { text += b.toString() })
  child.stderr.on('data', b => { text += b.toString() })
  child.once('error', reject)
  child.once('close', code => code === 0 ? resolve(text) : reject(new Error(`7-Zip ${code}: ${text}`)))
})
const extractHash = (file, member, secret) => new Promise((resolve, reject) => {
  const h = createHash('sha256')
  let bytes = 0, stderr = ''
  const child = spawn(tool, ['x', '-so', '-spd', secret ? '-p' + secret : '-p-', file, member], {
    windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'],
  })
  child.stdout.on('data', b => { bytes += b.length; h.update(b) })
  child.stderr.on('data', b => { stderr += b.toString() })
  child.once('error', reject)
  child.once('close', code => {
    if (code !== 0) return reject(new Error(`Independent extraction ${code}: ${stderr}`))
    resolve({ bytes, sha256: h.digest('hex') })
  })
})

if (process.argv.includes('--prepare')) {
  const cases = []
  for (const mode of ['plain', 'aes', 'zipcrypto']) {
    const file = path.join(work, `external-${mode}.zip`)
    const reused = existsSync(file)
    if (!reused) {
      console.log(`[zip64-1515] original 7-Zip creation ${mode}`)
      const log = await run(['a', '-tzip', '-mx=1', '-mm=Deflate', '-bd', '-y',
        ...(mode === 'plain' ? [] : ['-p' + password, '-mem=' + (mode === 'aes' ? 'AES256' : 'ZipCrypto')]),
        file, ...manifest.files.map(f => f.name), manifest.emptyDirectory], path.join(work, 'source'))
      writeFileSync(path.join(work, `external-${mode}.txt`), log)
    }
    cases.push({ mode, file, bytes: statSync(file).size, sha256: await sha(file), reused })
    writeFileSync(path.join(work, 'originals.json'), JSON.stringify({ tool: identity, source: manifest, cases }, null, 2))
  }
  const file = path.join(work, 'external-plain-stream.zip')
  if (!existsSync(file)) {
    console.log('[zip64-1515] original streaming ZIP64 with data descriptors')
    const fd = openSync(file, 'wx')
    try {
      await new Promise((resolve, reject) => {
        let stderr = ''
        const child = spawn(tool, ['a', '-tzip', '-so', '-mx=1', '-mm=Deflate', '-bd', path.join(work, 'stdout-placeholder.zip'),
          ...manifest.files.map(f => f.name), manifest.emptyDirectory], {
          cwd: path.join(work, 'source'), windowsHide: true, stdio: ['ignore', fd, 'pipe'],
        })
        child.stderr.on('data', b => { stderr += b.toString() })
        child.once('error', reject)
        child.once('close', code => code === 0 ? resolve() : reject(new Error(stderr)))
      })
    } finally { closeSync(fd) }
  }
  cases.push({ mode: 'plain-stream', file, bytes: statSync(file).size, sha256: await sha(file), producerMode: 'actual stdout, no seek' })
  writeFileSync(path.join(work, 'originals.json'), JSON.stringify({ tool: identity, source: manifest, cases }, null, 2))
} else if (process.argv.includes('--read-products')) {
  const native = JSON.parse(readFileSync(path.join(work, 'native-results.json'), 'utf8'))
  const cases = []
  for (const c of native) {
    console.log(`[zip64-1515] independent full read ${c.mode}/${c.layout}`)
    const files = []
    for (const f of manifest.files) {
      const got = await extractHash(c.source, f.name, c.mode === 'plain' ? null : password)
      assert.equal(got.bytes, f.bytes)
      assert.equal(got.sha256, f.sha256)
      files.push({ name: f.name, ...got })
    }
    cases.push({ mode: c.mode, layout: c.layout, source: c.source, files, independentlyDecoded: true })
    writeFileSync(path.join(work, 'independent-progress.json'), JSON.stringify({ tool: identity, cases }, null, 2))
  }
  writeFileSync(path.join(work, 'independent-results.json'), JSON.stringify({ tool: identity, cases }, null, 2))
} else {
  assert.fail('Choose --prepare or --read-products')
}
