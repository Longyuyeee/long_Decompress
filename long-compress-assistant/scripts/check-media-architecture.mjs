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
