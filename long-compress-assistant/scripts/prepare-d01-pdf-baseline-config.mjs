import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const configuredOutput = process.argv.find(argument => argument.startsWith('--output='))?.slice('--output='.length)
const output = resolve(root, configuredOutput || 'test-results/d01-pdf-delta/baseline.tauri.conf.json')
const source = JSON.parse(await readFile(resolve(root, 'src-tauri/tauri.conf.json'), 'utf8'))
const resources = source?.tauri?.bundle?.resources
if (!Array.isArray(resources)) throw new Error('tauri.bundle.resources must be an array')

const prefix = 'resources/pdf-engine/'
const removed = resources.filter(resource => resource.replaceAll('\\', '/').startsWith(prefix))
const retained = resources.filter(resource => !resource.replaceAll('\\', '/').startsWith(prefix))
if (removed.length !== 10) throw new Error(`expected exactly ten PDF runtime resources, removed ${removed.length}`)
if (retained.length === 0) throw new Error('baseline must retain all non-PDF product resources')

source.tauri.bundle.resources = retained
await mkdir(dirname(output), { recursive: true })
await writeFile(output, `${JSON.stringify(source, null, 2)}\n`)
console.log(`D-01 same-commit baseline config written: removed=${removed.length}, retained=${retained.length}, output=${output}`)
