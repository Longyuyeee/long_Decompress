import { createHash } from 'node:crypto'
import { mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { basename, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const manifest = JSON.parse(await readFile(join(root, 'config', 'media-dependencies.json'), 'utf8'))
const ffmpegDependency = manifest.dependencies.find(dependency => dependency.id === 'ffmpeg')
const runtimeDirectory = resolve(argument('--runtime') || join(root, 'test-results', 'video-c01-audit', 'formal-a', 'bin'))
const input = resolve(argument('--input') || join(root, 'test-results', 'video-c01-audit', 'real', 'input-h264-aac.mp4'))
const runtimeIdentity = basename(resolve(runtimeDirectory, '..')).replaceAll(/[^A-Za-z0-9._-]/g, '_')
const auditDirectory = join(root, 'test-results', 'video-c01-audit', 'verification', runtimeIdentity)
const output = join(auditDirectory, 'output-h264-mf-aac.mp4')
const ffmpeg = join(runtimeDirectory, 'ffmpeg.exe')
const ffprobe = join(runtimeDirectory, 'ffprobe.exe')

function argument(name) {
  const index = process.argv.indexOf(name)
  return index < 0 ? null : process.argv[index + 1]
}

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function run(command, args, label) {
  const result = spawnSync(command, args, { encoding: 'utf8', windowsHide: true, maxBuffer: 32 * 1024 * 1024 })
  assert(result.status === 0, `${label} failed (${result.status}): ${result.error?.message || result.stderr || result.stdout}`)
  return { stdout: result.stdout || '', stderr: result.stderr || '' }
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex')
}

function toWslPath(path) {
  const match = /^([A-Za-z]):\\(.*)$/.exec(resolve(path))
  assert(match, `cannot translate Windows path to WSL: ${path}`)
  return `/mnt/${match[1].toLowerCase()}/${match[2].replaceAll('\\', '/')}`
}

assert(process.platform === 'win32', 'C-01 runtime verification requires Windows x64')
assert(ffmpegDependency?.runtimeCandidate, 'FFmpeg runtimeCandidate identity is missing from the dependency manifest')
await mkdir(auditDirectory, { recursive: true })
await rm(output, { force: true })

const identities = {}
for (const path of [ffmpeg, ffprobe]) {
  const name = basename(path)
  const expected = ffmpegDependency.runtimeCandidate.files.find(file => file.name === name)
  assert(expected, `${name}: frozen runtime identity is missing`)
  const actual = { name, bytes: (await stat(path)).size, sha256: await sha256(path) }
  assert(actual.bytes === expected.bytes, `${name}: byte size mismatch; expected=${expected.bytes} actual=${actual.bytes}`)
  assert(actual.sha256 === expected.sha256, `${name}: SHA-256 mismatch; expected=${expected.sha256} actual=${actual.sha256}`)
  identities[name] = actual
}

const version = run(ffmpeg, ['-version'], 'ffmpeg -version')
assert(version.stdout.includes('ffmpeg version 9.0.1'), 'unexpected FFmpeg version')
assert(version.stdout.includes('--disable-everything'), 'minimal configure policy is missing')
assert(version.stdout.includes('--disable-hwaccels'), 'hardware accelerators must remain disabled in C-01')
assert(version.stdout.includes("--extra-ldflags='-static -static-libgcc -Wl,--no-insert-timestamp'"), 'static reproducible linker flags are missing')
for (const forbidden of ['--enable-gpl', '--enable-nonfree', 'libx264', 'libx265']) {
  assert(!version.stdout.includes(forbidden), `forbidden FFmpeg feature detected: ${forbidden}`)
}

const encoders = run(ffmpeg, ['-hide_banner', '-encoders'], 'ffmpeg -encoders').stdout
assert(/\bh264_mf\b/.test(encoders), 'h264_mf encoder is missing')
assert(/\baac\b/.test(encoders), 'AAC encoder is missing')
assert(!/libx26[45]|libopenh264/.test(encoders), 'forbidden external H.264/H.265 encoder is present')
const encoderHelp = run(ffmpeg, ['-hide_banner', '-h', 'encoder=h264_mf'], 'h264_mf option probe').stdout
assert(encoderHelp.includes('hw_encoding') && encoderHelp.includes('default false'), 'software-default Media Foundation contract is missing')

const filters = run(ffmpeg, ['-hide_banner', '-filters'], 'ffmpeg -filters').stdout
for (const required of ['scale', 'format', 'fps', 'transpose', 'aresample']) {
  assert(new RegExp(`\\b${required}\\b`).test(filters), `required filter is missing: ${required}`)
}

const importsOutput = run('wsl.exe', ['-d', 'Ubuntu', '--', 'x86_64-w64-mingw32-objdump', '-p', toWslPath(ffmpeg)], 'PE import audit').stdout
const imports = [...importsOutput.matchAll(/DLL Name:\s*([^\r\n]+)/g)].map(match => match[1].trim().toLowerCase())
for (const forbiddenImport of ['libwinpthread-1.dll', 'libgcc_s_seh-1.dll', 'libstdc++-6.dll']) {
  assert(!imports.includes(forbiddenImport), `non-system runtime dependency detected: ${forbiddenImport}`)
}

const encode = run(ffmpeg, [
  '-hide_banner', '-nostats', '-loglevel', 'error', '-y', '-i', input,
  '-map', '0:v:0', '-map', '0:a:0?', '-vf', 'scale=480:-2,format=nv12',
  '-c:v', 'h264_mf', '-hw_encoding', '0', '-rate_control', 'quality', '-quality', '70',
  '-c:a', 'aac', '-b:a', '128k', '-movflags', '+faststart',
  '-progress', 'pipe:1', output,
], 'real Media Foundation software transcode')
assert(encode.stdout.includes('progress=end'), 'FFmpeg progress pipe did not reach end')
assert(/out_time_us=5000000/.test(encode.stdout), 'FFmpeg progress pipe did not report the real five-second timeline')

const probe = JSON.parse(run(ffprobe, [
  '-v', 'error', '-show_entries', 'format=format_name,duration,size:stream=index,codec_type,codec_name,width,height',
  '-of', 'json', output,
], 'ffprobe output verification').stdout)
const video = probe.streams.find(stream => stream.codec_type === 'video')
const audio = probe.streams.find(stream => stream.codec_type === 'audio')
assert(video?.codec_name === 'h264' && video.width === 480 && video.height === 270, 'output video facts differ from the expected H.264 480x270 result')
assert(audio?.codec_name === 'aac', 'output AAC stream is missing')
assert(Math.abs(Number(probe.format.duration) - 5) <= 0.05, `output duration differs from five seconds: ${probe.format.duration}`)

const result = {
  passed: true,
  expected: { version: '9.0.1', license: 'LGPL-2.1-or-later', videoCodec: 'h264', audioCodec: 'aac', width: 480, height: 270, durationSeconds: 5, hardwareEncoding: false },
  actual: {
    identities,
    imports,
    input: { path: input, bytes: (await stat(input)).size, sha256: await sha256(input) },
    output: { path: output, bytes: (await stat(output)).size, sha256: await sha256(output), probe },
    progress: encode.stdout.trim().split(/\r?\n/),
  },
  differences: [],
}
await writeFile(join(auditDirectory, 'result.json'), `${JSON.stringify(result, null, 2)}\n`)
console.log(`C-01 real FFmpeg baseline passed: ${identities['ffmpeg.exe'].sha256}, output=${result.actual.output.bytes} bytes, differences=0.`)
