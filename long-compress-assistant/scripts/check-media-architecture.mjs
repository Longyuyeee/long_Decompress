import { readFile, readdir } from 'node:fs/promises'
import { extname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = new URL('../', import.meta.url)
const rootPath = fileURLToPath(root)

async function read(relativePath) {
  return readFile(new URL(relativePath, root), 'utf8')
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

async function collectFiles(directory, extensions) {
  const absolute = fileURLToPath(new URL(directory, root))
  const result = []
  async function visit(path) {
    for (const entry of await readdir(path, { withFileTypes: true })) {
      const child = join(path, entry.name)
      if (entry.isDirectory()) {
        await visit(child)
      } else if (extensions.has(extname(entry.name))) {
        result.push(child)
      }
    }
  }
  await visit(absolute)
  return result
}

const transaction = await read('src-tauri/src/services/output_publish_transaction.rs')
assert(transaction.includes('publish_verified_file'), 'shared verified-file publication boundary is missing')
assert(transaction.includes('is_cancelled'), 'shared publication boundary must check cancellation')
assert(transaction.includes('TargetAppeared'), 'shared publication boundary must reject target races')
assert(transaction.includes('std::fs::rename(staged_output, final_output)'), 'shared publication must use same-directory atomic rename')
const sourceRecycle = await read('src-tauri/src/services/source_recycle.rs')
assert(sourceRecycle.includes('trash::delete_all(paths)'), 'shared source cleanup must use the Windows system Recycle Bin')

const compression = await read('src-tauri/src/services/compression_service.rs')
assert(
  compression.includes('output_publish_transaction::publish_verified_file'),
  'archive compression must exercise the shared publication boundary before media work starts',
)
assert(
  !compression.includes('std::fs::rename(working_output, final_output)'),
  'archive compression must not bypass the shared publication boundary',
)

const compressionCommands = await read('src-tauri/src/commands/compression.rs')
const imageCommandStart = compressionCommands.indexOf('pub async fn compress_image_file')
const imageCommandEnd = compressionCommands.indexOf('pub async fn cancel_compression', imageCommandStart)
assert(imageCommandStart >= 0 && imageCommandEnd > imageCommandStart, 'image compression command boundary is missing')
const imageCommand = compressionCommands.slice(imageCommandStart, imageCommandEnd)
assert(imageCommand.includes('window.emit("task-log"'), 'image stages must use the unified task log event')
assert(!imageCommand.includes('"task-progress"'), 'image stages must not emit synthetic progress percentages')

const main = await read('src-tauri/src/main.rs')
assert(
  (main.match(/commands::video_engine::preflight_video_engine/g) ?? []).length === 1,
  'C-01.2.1 must expose exactly one authoritative video-engine preflight command',
)
const videoEngine = await read('src-tauri/src/services/video_engine.rs')
for (const contract of [
  'VIDEO_ENGINE_RESOURCE_MISSING',
  'VIDEO_ENGINE_RESOURCE_HASH_MISMATCH',
  'VIDEO_ENGINE_VERSION_POLICY_MISMATCH',
  'VIDEO_ENGINE_ENCODER_MISSING',
  'VIDEO_ENGINE_FILTER_MISSING',
  'Command::new(executable)',
  '.args(arguments)',
  'hw_encoding',
  'default false',
]) {
  assert(videoEngine.includes(contract), `C-01.2.1 video preflight contract is missing: ${contract}`)
}
assert(!videoEngine.includes('cmd.exe') && !videoEngine.includes('powershell'), 'video preflight must not use a shell')
assert(
  (main.match(/commands::compression::plan_image_compression_destination/g) ?? []).length === 1,
  'the application must expose exactly one authoritative image destination planner',
)
const tauriCommands = await read('src/composables/useTauriCommands.ts')
assert(
  tauriCommands.includes("invoke<ImageDestinationPlan>('plan_image_compression_destination'"),
  'the frontend must use the backend image destination planner',
)
assert(
  tauriCommands.includes("invoke<ImageCompressionOutcome>('compress_image_file'"),
  'the frontend image command wrapper is missing',
)
const imageWorkspaceModel = await read('src/utils/imageCompressionWorkspace.ts')
assert(imageWorkspaceModel.includes('class ImageCompressionBatchRunner'), 'the image batch runner is missing')
assert(imageWorkspaceModel.includes('commands.cancel(this.activeTaskId)'), 'image batch cancellation must target the active child task')
assert(imageWorkspaceModel.includes('results.length / jobs.length * 100'), 'image batch progress must derive from settled file count')
assert(!imageWorkspaceModel.includes('task-progress'), 'the image batch runner must not manufacture encoder byte progress')
const imageBatchTracking = await read('src/composables/useImageCompressionBatch.ts')
assert(imageBatchTracking.includes("workloadKind: 'image'"), 'image child tasks must use the unified image workload identity')
assert(imageBatchTracking.includes('createVerifiedImageTaskMetricsV1'), 'published image history must use verified backend facts')
assert(imageBatchTracking.includes('waitForHistoryPersistence'), 'image batch completion must await history persistence')
assert(!imageBatchTracking.includes("invoke('save_task_history'"), 'image orchestration must not bypass the unified task store history writer')
const imageWorkspaceView = await read('src/components/compression/ImageCompressionWorkspace.vue')
assert(imageWorkspaceView.includes('useImageCompressionBatch'), 'the image workspace must use the audited batch composable')
assert(imageWorkspaceView.includes('taskForItem(item)?.metrics'), 'the image result UI must read verified unified task metrics')
assert(!imageWorkspaceView.includes('B-02 前端'), 'the enabled image workspace must not retain the B-02 placeholder badge')
assert(!imageWorkspaceView.includes('B-03 实际编码后显示'), 'the result preview must not retain the B-03 placeholder')
assert(!imageWorkspaceView.includes("invoke('compress_image_file'"), 'the image workspace must not bypass the audited batch composable')
const packageManifest = JSON.parse(await read('package.json'))
assert(
  packageManifest.scripts?.['test:video-runtime-package:real'] === 'node scripts/check-video-runtime-package.mjs',
  'C-01.2.1 real packaged-runtime command is missing',
)
const mediaFixtureManifest = JSON.parse(await read('tests/fixtures/media/manifest.json'))
assert(mediaFixtureManifest.videoFixtureSource?.kind === 'tracked-frozen-real-containers', 'video fixtures must be frozen real containers')
assert(mediaFixtureManifest.videoFixtureSource?.probeRuntime === 'src-tauri/resources/video-engine/ffprobe.exe', 'video fixtures must use the admitted product ffprobe')
assert(mediaFixtureManifest.videos?.length === 2, 'C-01.2.1 must retain two frozen real video fixtures')
assert(
  mediaFixtureManifest.videos.every(item => item.bytes > 0 && /^[a-f0-9]{64}$/.test(item.sha256)),
  'video fixtures must retain exact byte identities',
)
const mediaFixturePreparation = await read('scripts/prepare-media-test-fixtures.mjs')
assert(mediaFixturePreparation.includes("'src-tauri', 'resources', 'video-engine', 'ffprobe.exe'"), 'fixture preparation must probe with the product ffprobe')
for (const forbidden of ['BtbN', 'downloadTestTool', 'generateVideoFixtures']) {
  assert(!mediaFixturePreparation.includes(forbidden), `mutable/generated video fixture path returned: ${forbidden}`)
}
assert(
  packageManifest.scripts?.['test:image-matrix:real'] === 'npm run test:fixtures:media:images && node scripts/run-b05-image-format-matrix.mjs',
  'B-05.1 real production matrix command is missing or bypasses fixture preparation',
)
assert(
  packageManifest.scripts?.['test:e2e:desktop:image-batch'] === 'npm run test:fixtures:media:images && node scripts/test-tauri-desktop.mjs --image-batch-only',
  'B-05.2.1 real desktop batch command is missing or bypasses fixture preparation',
)
const desktopE2e = await read('scripts/test-tauri-desktop.mjs')
assert(desktopE2e.includes("const imageBatchOnly = process.argv.includes('--image-batch-only')"), 'B-05.2.1 focused desktop gate is missing')
assert(desktopE2e.includes('const expectedBatchSize = 100'), 'B-05.2.1 batch size contract must remain fixed at 100')
assert(desktopE2e.includes('source bytes must remain unchanged'), 'B-05.2.1 must verify source byte identity after execution')
assert(desktopE2e.includes('every published image must persist one unified history row'), 'B-05.2.1 must verify all history rows')
assert(
  packageManifest.scripts?.['test:image-boundaries:real'] === 'npm run test:fixtures:media:images && node scripts/run-b05-image-boundaries.mjs',
  'B-05.2.2 real production boundary command is missing or bypasses fixture preparation',
)
const imageBoundaryRunner = await read('scripts/run-b05-image-boundaries.mjs')
assert(imageBoundaryRunner.includes('resourceBelowLimitPixels: 96_000_000'), 'B-05.2.2 must lock the real below-limit image')
assert(imageBoundaryRunner.includes('resourceAboveLimitRejected: true'), 'B-05.2.2 must reject a valid image above 100 MP')
assert(imageBoundaryRunner.includes("storageFullKind: 'storage-full'"), 'B-05.2.2 must exercise standard StorageFull failure injection')
assert(imageBoundaryRunner.includes('cancelledAfterEncodingStarted: true'), 'B-05.2.2 must cancel after real encoding starts')
const imageMatrix = JSON.parse(await read('tests/fixtures/media/b05-image-format-matrix.json'))
assert(imageMatrix.expected?.samplesPerFormat === 3, 'B-05.1 must freeze three samples per public image format')
assert(imageMatrix.cases?.length === 9, 'B-05.1 must retain nine real image cases')
for (const format of ['jpeg', 'png', 'webp']) {
  assert(imageMatrix.cases.filter(item => item.format === format).length === 3, `B-05.1 ${format} sample count drifted`)
}
assert(
  imageMatrix.cases.every(item => item.bytes > 0 && /^[a-f0-9]{64}$/.test(item.sha256)),
  'B-05.1 inputs must retain frozen real byte identities',
)
const imageService = await read('src-tauri/src/services/image_compression_service.rs')
assert(
  imageService.includes('fn b05_public_format_matrix_uses_real_production_compression()'),
  'B-05.1 must execute the real production image service',
)
assert(
  imageService.includes('fn b05_2_2_real_resource_and_failure_boundaries()'),
  'B-05.2.2 must execute the real production image service',
)
assert(
  imageService.includes('compress_single_image_with_writer'),
  'B-05.2.2 StorageFull injection must remain at the production file-write boundary',
)
assert(
  imageService.includes('if read_metadata(path, false)? == expected'),
  'matching WebP metadata must not be destructively rewritten',
)
assert(
  (main.match(/commands::task_history::save_task_history/g) ?? []).length === 1,
  'the application must expose exactly one history write command',
)

const sourceFiles = await collectFiles('src/', new Set(['.ts', '.vue']))
const historyWriters = []
for (const file of sourceFiles) {
  const source = await readFile(file, 'utf8')
  if (source.includes("invoke('save_task_history'")) {
    historyWriters.push(relative(rootPath, file).replaceAll('\\', '/'))
  }
}
assert(
  historyWriters.length === 1 && historyWriters[0].endsWith('src/stores/task.ts'),
  `history writes must remain centralized in src/stores/task.ts; found: ${historyWriters.join(', ') || 'none'}`,
)

const productionFiles = [
  ...(await collectFiles('src/', new Set(['.ts', '.vue']))),
  ...(await collectFiles('src-tauri/src/', new Set(['.rs']))),
]
const mediaFiles = productionFiles.filter((file) =>
  /(?:^|[\\/])media(?:[_.\\/-]|$)/i.test(file)
  || /image[_-]?compression/i.test(file),
)
const forbiddenMediaBypasses = [
  'std::fs::rename(',
  'tokio::fs::rename(',
  'trash::delete(',
  'trash::delete_all(',
  "invoke('save_task_history'",
  'defineStore(\'media',
  'defineStore("media',
  'defineStore(\'image',
  'defineStore("image',
]
for (const file of mediaFiles) {
  const source = await readFile(file, 'utf8')
  const bypass = forbiddenMediaBypasses.find((needle) => source.includes(needle))
  assert(!bypass, `media production code bypasses a shared boundary in ${file}: ${bypass}`)
}

console.log(`Media architecture gate passed (${mediaFiles.length} media production files inspected).`)
