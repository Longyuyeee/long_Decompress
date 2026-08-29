import { mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const contract = JSON.parse(await readFile(join(root, 'config', 'pdf-optimization-contract.json'), 'utf8'))
const manifest = JSON.parse(await readFile(join(root, 'tests', 'fixtures', 'media', 'manifest.json'), 'utf8'))
const qpdf = join(root, 'test-results', 'media-dependency-audit', 'real-qpdf', `qpdf-${contract.engine.version}-mingw64`, 'bin', 'qpdf.exe')
const fixtureRoot = join(root, 'test-results', 'media-fixture-audit', 'fixtures', 'pdfs')
const outputRoot = join(root, 'test-results', 'd01-pdf-baseline')
const python = process.env.LONG_MEDIA_FIXTURE_PYTHON || 'python'
const pythonPackages = join(root, 'test-results', 'media-fixture-audit', 'python-packages')
const pythonInspector = join(root, 'scripts', 'inspect-d01-pdf.py')

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function run(command, args, label, acceptedStatuses = [0]) {
  const result = spawnSync(command, args, { encoding: 'utf8', windowsHide: true })
  assert(acceptedStatuses.includes(result.status), `${label} failed (${result.status}): ${result.stderr || result.stdout || result.error?.message || 'no output'}`)
  return { status: result.status, output: `${result.stdout ?? ''}${result.stderr ?? ''}` }
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex')
}

function jsonFacts(path, password) {
  const args = []
  if (password) args.push(`--password=${password}`)
  args.push('--json=2', '--json-stream-data=none', path)
  const parsed = JSON.parse(run(qpdf, args, `${path} qpdf JSON`).output)
  const objects = parsed.qpdf?.[1] ?? {}
  const annotationSubtypes = []
  for (const item of Object.values(objects)) {
    const value = item?.value
    if (value?.['/Type'] === '/Annot') annotationSubtypes.push(value['/Subtype'])
  }
  const outlineTitles = (parsed.outlines ?? []).map(item => item.title).filter(Boolean).sort()
  const fields = (parsed.acroform?.fields ?? []).map(item => item.fullname).filter(Boolean).sort()
  const signatureFields = (parsed.acroform?.fields ?? []).filter(item => item.fieldtype === '/Sig').map(item => item.fullname).sort()
  return {
    pages: parsed.pages?.length ?? 0,
    fields,
    annotationSubtypes: annotationSubtypes.sort(),
    outlineTitles,
    attachments: Object.values(parsed.attachments ?? {}).map(item => item.preferredname).filter(Boolean).sort(),
    signatureFields,
  }
}

function equalFacts(expected, actual, label) {
  assert(JSON.stringify(expected) === JSON.stringify(actual), `${label} structural facts changed:\nexpected=${JSON.stringify(expected)}\nactual=${JSON.stringify(actual)}`)
}

function preservationFacts(path, password) {
  const args = [pythonInspector, path]
  if (password) args.push('--password', password)
  const result = spawnSync(python, args, {
    encoding: 'utf8',
    windowsHide: true,
    env: { ...process.env, PYTHONPATH: pythonPackages },
  })
  assert(result.status === 0, `${path} independent preservation inspection failed (${result.status}): ${result.stderr || result.stdout || result.error?.message || 'no output'}`)
  return JSON.parse(result.stdout)
}

await stat(qpdf).catch(() => { throw new Error('qpdf candidate is missing; run npm.cmd run test:media-dependencies:real first') })
await stat(fixtureRoot).catch(() => { throw new Error('PDF fixtures are missing; run npm.cmd run test:fixtures:media first') })
await rm(outputRoot, { recursive: true, force: true })
await mkdir(outputRoot, { recursive: true })

const versionOutput = run(qpdf, ['--version'], 'qpdf version').output
assert(versionOutput.includes(`qpdf version ${contract.engine.version}`), 'qpdf candidate version mismatch')
const cryptoOutput = run(qpdf, ['--show-crypto'], 'qpdf crypto').output
assert(cryptoOutput.toLowerCase().includes('openssl'), 'qpdf OpenSSL provider is missing')

const results = []
for (const fixture of manifest.pdfs) {
  const input = join(fixtureRoot, fixture.file)
  const password = fixture.kind === 'encrypted-refusal' ? fixture.password : undefined
  const size = (await stat(input)).size
  const digest = await sha256(input)
  if (fixture.kind === 'encrypted-refusal') {
    const unauthorised = run(qpdf, ['--check', input], `${fixture.file} unauthorised refusal`, [2])
    assert(unauthorised.output.toLowerCase().includes('invalid password'), 'encrypted PDF did not refuse unauthorised inspection')
    run(qpdf, [`--password=${password}`, '--check', input], `${fixture.file} authorised check`)
    results.push({ file: fixture.file, kind: fixture.kind, inputBytes: size, inputSha256: digest, decision: 'password-required-before-planning' })
    continue
  }

  run(qpdf, ['--check', input], `${fixture.file} input check`)
  const inputFacts = jsonFacts(input)
  const inputPreservationFacts = preservationFacts(input)
  assert(inputFacts.pages === fixture.pages, `${fixture.file}: qpdf page count differs from fixture contract`)
  if (fixture.kind === 'digitally-signed') {
    assert(inputFacts.signatureFields.includes(fixture.signatureField), 'signed PDF signature field was not detected')
    results.push({ file: fixture.file, kind: fixture.kind, inputBytes: size, inputSha256: digest, facts: inputFacts, preservationFacts: inputPreservationFacts, decision: 'analysis-only-execution-blocked' })
    continue
  }

  const modeResults = {}
  for (const [modeName, mode] of Object.entries(contract.modes)) {
    const output = join(outputRoot, `${fixture.file.replace(/\.pdf$/i, '')}-${modeName}.pdf`)
    run(qpdf, [...mode.arguments, input, output], `${fixture.file} ${modeName}`)
    run(qpdf, ['--check', output], `${fixture.file} ${modeName} output check`)
    const outputFacts = jsonFacts(output)
    equalFacts(inputFacts, outputFacts, `${fixture.file} ${modeName}`)
    const outputPreservationFacts = preservationFacts(output)
    equalFacts(inputPreservationFacts, outputPreservationFacts, `${fixture.file} ${modeName} independent preservation`)
    modeResults[modeName] = {
      outputBytes: (await stat(output)).size,
      outputSha256: await sha256(output),
      deltaBytes: (await stat(output)).size - size,
      facts: outputFacts,
      preservationFacts: outputPreservationFacts,
    }
  }
  results.push({ file: fixture.file, kind: fixture.kind, inputBytes: size, inputSha256: digest, facts: inputFacts, preservationFacts: inputPreservationFacts, modes: modeResults, decision: 'candidate-transform-baseline-passed' })
}

const evidence = {
  schemaVersion: 1,
  node: contract.baselineNode,
  engine: { id: contract.engine.id, version: contract.engine.version, versionOutput: versionOutput.trim(), cryptoOutput: cryptoOutput.trim() },
  fixtureRevision: manifest.fixtureRevision,
  productRuntimeIntegrated: contract.executionBoundary.publishProductRuntime,
  productTransformationIntegrated: false,
  productUiEnabled: false,
  results,
}
await writeFile(join(outputRoot, 'result.json'), JSON.stringify(evidence, null, 2), 'utf8')
console.log(`D-01.1 real qpdf baseline passed (${results.length} PDFs; runtime preflight admitted, transformation remains blocked).`)
