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
const imageCommandEnd = compressionCommands.indexOf('pub struct VideoCompressionExecutionRequest', imageCommandStart)
assert(imageCommandStart >= 0 && imageCommandEnd > imageCommandStart, 'image compression command boundary is missing')
const imageCommand = compressionCommands.slice(imageCommandStart, imageCommandEnd)
assert(imageCommand.includes('window.emit("task-log"'), 'image stages must use the unified task log event')
assert(!imageCommand.includes('"task-progress"'), 'image stages must not emit synthetic progress percentages')

const videoCommandStart = imageCommandEnd
const videoCommandEnd = compressionCommands.indexOf('pub fn plan_image_compression_destination', videoCommandStart)
assert(videoCommandEnd > videoCommandStart, 'video compression command boundary is missing')
const videoCommand = compressionCommands.slice(videoCommandStart, videoCommandEnd)
for (const contract of [
  'register_task_cancellation(&task_id)',
  'CompressionOutputGuard::acquire(&task_id, &output_path)',
  'probe_video_file(&ffprobe, &source)',
  'request.confirmed_stream_changes != plan.stream_changes',
  'encode_video_to_staging',
  'validate_staged_video_output',
  'publish_validated_video_output',
  'window.emit("task-log"',
  'window.emit("task-progress"',
  '"still-encoding"',
]) {
  assert(videoCommand.includes(contract), `C-03.3 video command contract is missing: ${contract}`)
}

const main = await read('src-tauri/src/main.rs')
assert(
  (main.match(/commands::compression::compress_video_file/g) ?? []).length === 1,
  'C-03.3 must expose exactly one safe video execution command',
)
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
  (main.match(/commands::video_engine::probe_video_input/g) ?? []).length === 1,
  'C-02.1 must expose exactly one authoritative video probe command',
)
assert(
  (main.match(/commands::video_engine::plan_video_compression/g) ?? []).length === 1,
  'C-02.2 must expose exactly one authoritative video compression planner',
)
const videoProbe = await read('src-tauri/src/services/video_probe.rs')
for (const contract of [
  'VIDEO_PROBE_SOURCE_EMPTY',
  'VIDEO_PROBE_TIMEOUT',
  'VIDEO_PROBE_OUTPUT_TOO_LARGE',
  'VIDEO_PROBE_NO_VIDEO_STREAM',
  'drop-with-explicit-warning',
  'refuse-before-encoding',
  'preserve-input-timestamps',
]) {
  assert(videoProbe.includes(contract), `C-02.1 video probe contract is missing: ${contract}`)
}
assert(videoProbe.includes('.args(['), 'video probe arguments must be passed as an argument array')
assert(videoProbe.includes('.kill_on_drop(true)'), 'timed-out video probes must terminate the child process')
assert(!videoProbe.includes('cmd.exe') && !videoProbe.includes('powershell'), 'video probe must not use a shell')
const videoCommands = await read('src-tauri/src/commands/video_engine.rs')
assert(videoCommands.includes('validate_video_engine(&validation_root)'), 'video probe must validate the admitted runtime first')
assert(videoCommands.includes('probe_video_file(&ffprobe'), 'video probe command must use the bounded production service')
const videoPlan = await read('src-tauri/src/services/video_compression_plan.rs')
for (const contract of [
  'VideoCompressionPreset',
  'Clear',
  'Balanced',
  'Small',
  'will_upscale: false',
  'preserve-within-even-dimension-rounding',
  'is_estimate: true',
  'estimate-only;',
  'MAX_OUTPUT_PIXELS',
]) {
  assert(videoPlan.includes(contract), `C-02.2 video planning contract is missing: ${contract}`)
}
const videoEncoding = await read('src-tauri/src/services/video_encoding.rs')
for (const contract of [
  'build_ffmpeg_arguments',
  'OsString',
  'pipe:1',
  'out_time_us',
  'total_size',
  'valid_timeline_samples >= 2',
  'fps_mode',
  'rate_control',
  'JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE',
  'AssignProcessToJobObject',
  'TerminateJobObject',
  'encode_video_to_staging',
  'preflight_operation_resources',
  'cleanup_staged_output_family',
  'VideoEncodingEvent::Heartbeat',
  '.kill_on_drop(true)',
]) {
  assert(videoEncoding.includes(contract), `C-03.1 video execution contract is missing: ${contract}`)
}
assert(!videoEncoding.includes('Command::new("cmd.exe")'), 'video encoding must not launch through cmd.exe')
assert(!videoEncoding.includes('Command::new("powershell")'), 'video encoding must not launch through PowerShell')
assert(!main.includes('commands::video_engine::encode_video'), 'C-03.2 internal staging must not bypass the C-04 publication gate')

const pdfEngine = await read('src-tauri/src/services/pdf_engine.rs')
const pdfCommands = await read('src-tauri/src/commands/pdf_engine.rs')
const pdfAnalysis = await read('src-tauri/src/services/pdf_analysis.rs')
const pdfContract = JSON.parse(await read('config/pdf-optimization-contract.json'))
assert(
  (main.match(/commands::pdf_engine::preflight_pdf_engine/g) ?? []).length === 1,
  'D-02.1 must preserve exactly one qpdf identity preflight command',
)
assert(
  (main.match(/commands::pdf_engine::analyze_pdf_input/g) ?? []).length === 1,
  'D-02.1 must expose exactly one read-only PDF analysis command',
)
assert(
  main.includes('--internal-pdf-engine-preflight-report')
    && main.includes('write_installed_pdf_engine_preflight_report'),
  'D-01.2.1 installed qpdf preflight report entry point is missing',
)
assert(
  pdfCommands.includes('validate_pdf_engine(&resource_root)')
    && pdfCommands.includes('bundled_pdf_resource_root(&app_resource_dir)'),
  'qpdf command must resolve and validate the packaged production runtime',
)
assert(
  pdfEngine.includes('PDF_ENGINE_RESOURCE_HASH_MISMATCH')
    && pdfEngine.includes('qpdf version 12.4.0')
    && pdfEngine.includes('--show-crypto')
    && pdfEngine.includes('--help=--json')
    && pdfEngine.includes('--help=--optimize-images'),
  'qpdf production identity/capability refusal contract is incomplete',
)
assert(pdfContract.engine?.productionPreflightOnly === false, 'D-02.1 must enable product read-only analysis')
assert(pdfContract.analysisBoundary?.readOnly === true, 'D-02.1 analysis must remain read-only')
assert(pdfContract.executionBoundary?.enableProductUi === true, 'D-02.2 read-only PDF configuration UI must be enabled')
assert(pdfContract.executionBoundary?.executionEnabled === true, 'D-04.1 PDF product command must be enabled')
const compressionView = await read('src/views/CompressionView.vue')
const specialCompressionView = await read('src/views/SpecialCompressionView.vue')
const pdfWorkspace = await read('src/components/compression/PdfCompressionWorkspace.vue')
for (const workspace of ['ImageCompressionWorkspace', 'VideoCompressionWorkspace', 'PdfCompressionWorkspace']) {
  assert(
    specialCompressionView.includes(`import ${workspace}`) && specialCompressionView.includes(`<${workspace}`),
    `special compression workspace routing is missing: ${workspace}`,
  )
  assert(!compressionView.includes(workspace), `archive compression center must not route ${workspace}`)
}
assert(
  specialCompressionView.includes("type SpecialCompressionMode = 'image' | 'video' | 'pdf'")
    && !specialCompressionView.includes("'archive'"),
  'special compression must expose exactly the image/video/PDF product modes',
)
for (const boundary of ['usePdfOptimizationBatch', 'pdf-risk-confirmation', 'pdf-allow-larger-output', '锁定执行配置', '禁止覆盖源文件']) {
  assert(pdfWorkspace.includes(boundary), `D-04.2 PDF UI boundary is missing: ${boundary}`)
}
const pdfBatch = await read('src/composables/usePdfOptimizationBatch.ts')
for (const boundary of ["workloadKind: 'pdf'", 'createMeasuredTaskMetricsV1', 'waitForHistoryPersistence', 'cancelTask', 'pageCount']) {
  assert(pdfBatch.includes(boundary), `D-04.2 unified PDF task boundary is missing: ${boundary}`)
}
assert(!pdfWorkspace.includes('useCompressionStore'), 'PDF execution must use the unified task model instead of a media-specific task store')
const pdfTransform = await read('src-tauri/src/services/pdf_transform.rs')
for (const boundary of [
  'transform_arguments',
  'staged_output_path(&request.destination, "pdf-transform")',
  'cleanup_staged_output_family(&self.path)',
  'preflight_operation_resources(',
  'PDF_TRANSFORM_SIGNED_DOCUMENT_BLOCKED',
  'PDF_TRANSFORM_ENCRYPTED_DOCUMENT_BLOCKED',
  'PDF_TRANSFORM_LOSSY_CONFIRMATION_REQUIRED',
  'PDF_TRANSFORM_SOURCE_CHANGED_DURING_EXECUTION',
  'child.kill().await',
]) {
  assert(pdfTransform.includes(boundary), `D-03.1 PDF staging boundary is missing: ${boundary}`)
}
assert(pdfContract.executionBoundary?.stagingTransformEnabled === true, 'D-03.1 owned staging must be enabled')
const pdfValidation = await read('src-tauri/src/services/pdf_output_validation.rs')
for (const boundary of [
  'validate_staged_pdf_output',
  'PDF_OUTPUT_QPDF_CHECK_FAILED',
  'PDF_OUTPUT_LARGER_THAN_SOURCE',
  'PDF_OUTPUT_TARGET_APPEARED',
  'PDF_OUTPUT_CHANGED_DURING_VALIDATION',
  '--show-attachment=',
  'Sha256::digest',
  'compare_facts',
  'child.kill().await',
]) {
  assert(pdfValidation.includes(boundary), `D-03.2 PDF validation boundary is missing: ${boundary}`)
}
assert(pdfContract.executionBoundary?.validationEnabled === true && pdfContract.executionBoundary?.validationApiExposure === 'internal-library-only', 'D-03.2 internal validation must be enabled')
const pdfPublish = await read('src-tauri/src/services/pdf_publish.rs')
for (const boundary of [
  'execute_pdf_publication_transaction',
  'ACTIVE_PDF_OUTPUTS',
  'OutputLocked',
  'validate_staged_pdf_output',
  'source_sha256()',
  'verified.output_sha256',
  'mark_of_web::propagate_to_tree',
  'publish_verified_file(staged.path()',
  'FinalIdentityFailed',
]) {
  assert(pdfPublish.includes(boundary), `D-03.3 PDF publication boundary is missing: ${boundary}`)
}
assert(pdfContract.executionBoundary?.publicationEnabled === true && pdfContract.executionBoundary?.publicationApiExposure === 'internal-library-only', 'D-03.3 internal publication must be enabled')
assert(
  pdfAnalysis.includes('"--password-file=-"')
    && pdfAnalysis.includes('Stdio::piped()')
    && !pdfAnalysis.includes('OsString::from("--password=")'),
  'D-02.1 password must use qpdf stdin and stay off the process argument list',
)
assert(
  (main.match(/commands::pdf_engine::compress_pdf_file/g) ?? []).length === 1
    && (main.match(/commands::pdf_engine::plan_pdf_optimization_destination/g) ?? []).length === 1,
  'D-04.1 must expose exactly one PDF product command and one safe destination planner',
)
assert(
  pdfCommands.includes('execute_pdf_publication_transaction_observed')
    && pdfCommands.includes('register_task_cancellation(&task_id)')
    && pdfCommands.includes('validate_pdf_engine(&validation_root)')
    && pdfCommands.includes('allow_larger_output'),
  'D-04.1 product command must revalidate and use the shared cancellable publication transaction',
)
const videoValidation = await read('src-tauri/src/services/video_output_validation.rs')
for (const contract of [
  'validate_staged_video_output',
  'probe_video_file(ffprobe, staged.path())',
  '-count_frames',
  'duration_tolerance_ms',
  'DecodedFrameCountTooLow',
  'DecodedAudioFrameCountTooLow',
  'SizeChanged',
  'RotationNotNormalized',
  'LossyStreamsRemain',
]) {
  assert(videoValidation.includes(contract), `C-04.1 video validation contract is missing: ${contract}`)
}
assert(!videoValidation.includes('publish_verified_file'), 'C-04.1 validation must not publish before its own audit closes')
const videoPublish = await read('src-tauri/src/services/video_publish.rs')
for (const contract of [
  'publish_validated_video_output',
  'mark_of_web::read_from(source)',
  'mark_of_web::propagate_to_tree',
  'publish_verified_file(staged.path(), final_output',
  'TargetAppeared',
  'final_metadata.len() != verified.encoded_bytes',
  'savings_ratio',
]) {
  assert(videoPublish.includes(contract), `C-04.2 video publication contract is missing: ${contract}`)
}
assert(!videoPublish.includes('move_paths_to_system_recycle_bin'), 'video publication must not recycle sources before command-level opt-in')
for (const contract of [
  'real_ffmpeg_nonzero_exit_publishes_nothing_and_leaves_no_staging',
  'zeroed_output_after_encoding_is_rejected_and_cleaned_on_drop',
  'source_change_after_validation_prevents_publication',
]) {
  assert(
    videoEncoding.includes(contract) || videoValidation.includes(contract) || videoPublish.includes(contract),
    `C-04.3 failure-matrix evidence is missing: ${contract}`,
  )
}
const videoMatrixPackageManifest = JSON.parse(await read('package.json'))
assert(
  videoMatrixPackageManifest.scripts['test:video-matrix:real'] === 'node scripts/run-c05-video-format-matrix.mjs',
  'C-05.2.1 must expose one reproducible real video matrix command',
)
const videoMatrixManifest = JSON.parse(await read('tests/fixtures/media/c05-video-format-matrix.json'))
assert(videoMatrixManifest.fixtureTool.productIntegrationAllowed === false, 'C-05 fixture tooling must stay test-only')
assert(
  videoMatrixManifest.fixtureTool.archiveSha256 === '2e8e28af97c2ae338ccef92e36da9b2a4cd21d0cad9dde093545606cb07f5b00',
  'C-05 fixture-tool identity must stay pinned',
)
assert(videoMatrixManifest.sources.length === 5, 'C-05.2.1 must cover five declared input formats')
assert(videoMatrixManifest.executions.length === 7, 'C-05.2.1 must retain seven product-pipeline executions')
assert(
  new Set(videoMatrixManifest.executions.map(item => item.preset)).size === 3,
  'C-05.2.1 must exercise all three presets',
)
const videoMatrixRunner = await read('scripts/run-c05-video-format-matrix.mjs')
for (const contract of [
  "join(root, 'test-results', 'c05-fixture-tool')",
  "join(root, 'src-tauri', 'resources', 'video-engine', 'ffprobe.exe')",
  'c05_real_format_resolution_preset_matrix',
  "'-count_frames'",
  'productIntegrationAllowed === false',
]) {
  assert(videoMatrixRunner.includes(contract), `C-05.2.1 matrix contract is missing: ${contract}`)
}
assert(
  compressionCommands.includes('async fn c05_real_format_resolution_preset_matrix()'),
  'C-05.2.1 must run the private product compression pipeline',
)
assert(
  videoMatrixPackageManifest.scripts['test:video-long-large:real'] === 'node scripts/run-c05-video-long-large-matrix.mjs',
  'C-05.2.2 must expose one reproducible long/large video command',
)
const videoLongLargeManifest = JSON.parse(await read('tests/fixtures/media/c05-video-long-large-matrix.json'))
assert(videoLongLargeManifest.fixtureTool.productIntegrationAllowed === false, 'C-05.2.2 fixture tooling must stay test-only')
assert(videoLongLargeManifest.expected.longDurationMs === 600_000, 'C-05.2.2 must retain a ten-minute input')
assert(videoLongLargeManifest.expected.largeInputMinBytes === 100 * 1024 * 1024, 'C-05.2.2 large input must remain at least 100 MiB')
assert(videoLongLargeManifest.cases.length === 2, 'C-05.2.2 must keep independent long and large product executions')
const videoLongLargeRunner = await read('scripts/run-c05-video-long-large-matrix.mjs')
for (const contract of [
  "join(root, 'test-results', 'c05-fixture-tool')",
  "join(root, 'src-tauri', 'resources', 'video-engine', 'ffprobe.exe')",
  'c05_real_long_duration_and_large_input_matrix',
  "'-count_frames'",
  'productIntegrationAllowed === false',
]) {
  assert(videoLongLargeRunner.includes(contract), `C-05.2.2 matrix contract is missing: ${contract}`)
}
assert(
  compressionCommands.includes('async fn c05_real_long_duration_and_large_input_matrix()'),
  'C-05.2.2 must run long and large inputs through the private product pipeline',
)
const videoWorkspace = await read('src/components/compression/VideoCompressionWorkspace.vue')
assert(videoWorkspace.includes('commands.planVideoCompression'), 'C-02.3 workspace must consume the authoritative backend plan')
assert(videoWorkspace.includes('estimatedOutput.lowBytes'), 'C-02.3 workspace must display the labeled backend estimate')
assert(videoWorkspace.includes('useVideoCompressionBatch'), 'C-03.3.2 workspace must use the audited unified-task batch adapter')
assert(videoWorkspace.includes('confirmStreamChanges'), 'C-03.3.2 must confirm lossy stream changes before task creation')
assert(videoWorkspace.includes('taskForItem(item)'), 'C-03.3.2 result UI must read the unified task facts')
assert(!videoWorkspace.includes("invoke('compress_video_file'"), 'video workspace must not bypass the typed command adapter')
const compressionStore = await read('src/stores/compression.ts')
assert(compressionStore.includes('videoItems'), 'C-02.3 video drafts must reuse the existing compression store')
assert(compressionStore.includes('planRevision'), 'C-02.3 must reject stale asynchronous video plans')
assert(
  (main.match(/commands::compression::plan_image_compression_destination/g) ?? []).length === 1,
  'the application must expose exactly one authoritative image destination planner',
)
assert(
  (main.match(/commands::compression::plan_video_compression_destination/g) ?? []).length === 1,
  'the application must expose exactly one authoritative video destination planner',
)
const tauriCommands = await read('src/composables/useTauriCommands.ts')
assert(
  tauriCommands.includes("invoke<VideoProbeReport>('probe_video_input', { path })"),
  'the frontend video probe wrapper is missing or bypasses the typed contract',
)
assert(
  tauriCommands.includes("invoke<VideoCompressionPlan>('plan_video_compression', { request })"),
  'the frontend video planner wrapper is missing or bypasses the typed contract',
)
assert(tauriCommands.includes("invoke<PublishedVideoOutput>('compress_video_file'"), 'the frontend video execution wrapper is missing')
assert(tauriCommands.includes("invoke<VideoCompressionDestinationPlan>('plan_video_compression_destination'"), 'the frontend video destination planner is missing')
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
const videoBatchTracking = await read('src/composables/useVideoCompressionBatch.ts')
assert(videoBatchTracking.includes("workloadKind: 'video'"), 'video child tasks must use the unified video workload identity')
assert(videoBatchTracking.includes('createMeasuredTaskMetricsV1'), 'published video history must use verified backend facts')
assert(videoBatchTracking.includes('taskStore.cancelTask(activeTaskId)'), 'video cancellation must use the unified cancellation entry point')
assert(videoBatchTracking.includes('waitForHistoryPersistence'), 'video batch completion must await history persistence')
assert(!videoBatchTracking.includes("invoke('save_task_history'"), 'video orchestration must not bypass the unified task store history writer')
const imageWorkspaceView = await read('src/components/compression/ImageCompressionWorkspace.vue')
assert(imageWorkspaceView.includes('useImageCompressionBatch'), 'the image workspace must use the audited batch composable')
assert(imageWorkspaceView.includes('taskForItem(item)?.metrics'), 'the image result UI must read verified unified task metrics')
assert(!imageWorkspaceView.includes('B-02 前端'), 'the enabled image workspace must not retain the B-02 placeholder badge')
assert(!imageWorkspaceView.includes('B-03 实际编码后显示'), 'the result preview must not retain the B-03 placeholder')
assert(!imageWorkspaceView.includes("invoke('compress_image_file'"), 'the image workspace must not bypass the audited batch composable')
const packageManifest = JSON.parse(await read('package.json'))
assert(
  packageManifest.scripts?.['test:installed-video-workspace'] === 'npm run test:video-long-large:real && node scripts/test-installed-video-workspace.mjs',
  'C-05.4 formal installed video workspace gate is missing',
)
const installedVideoWorkspaceGate = await read('scripts/test-installed-video-workspace.mjs')
for (const contract of [
  'formal installed executable excludes desktop E2E bridge',
  'videoFfmpegProcessIds',
  'cancelled installed task does not publish output',
  'published MP4 exposes an enabled default-application action without launching an external window',
  'history survives complete installed-app restart',
  'productFfprobe',
]) {
  assert(installedVideoWorkspaceGate.includes(contract), `C-05.4 installed video contract is missing: ${contract}`)
}
assert(videoEngine.includes('WINDOWS_STATUS_DLL_INIT_FAILED'), 'installed video preflight must recognize the observed Windows DLL initialization transient')
assert(videoEngine.includes('const MAX_ATTEMPTS: usize = 2'), 'installed video preflight retry must remain bounded to one retry')
assert(
  videoEngine.includes('app_resource_dir.join("resources")'),
  'packaged video commands must preserve the configured resources/ prefix',
)
const videoEngineCommands = await read('src-tauri/src/commands/video_engine.rs')
assert(
  (videoEngineCommands.match(/bundled_resource_root\(&app_resource_dir\)/g) ?? []).length === 3,
  'all video probe and planning commands must resolve the packaged resource root consistently',
)
assert(
  compressionCommands.includes('bundled_resource_root(&app_resource_dir)'),
  'video execution must resolve the same packaged resource root as probe and planning',
)
const installedReleaseGate = await read('scripts/test-installed-release.ps1')
assert(installedReleaseGate.includes('[switch]$RunVideoWorkspaceMatrix'), 'installed lifecycle cannot run the C-05.4 video workspace matrix')
assert(installedReleaseGate.includes('test-installed-video-workspace.mjs'), 'installed lifecycle does not invoke the C-05.4 video workspace matrix')
assert(packageManifest.scripts?.['test:installed-pdf-workspace'] === 'node scripts/test-installed-pdf-workspace.mjs', 'D-04.3 installed PDF workspace command is missing')
assert(installedReleaseGate.includes('[switch]$RunPdfWorkspaceMatrix'), 'installed lifecycle cannot run the D-04.3 PDF workspace matrix')
assert(installedReleaseGate.includes('test-installed-pdf-workspace.mjs'), 'installed lifecycle does not invoke the D-04.3 production PDF workspace matrix')
const installedPdfWorkspace = await read('scripts/test-installed-pdf-workspace.mjs')
assert(installedPdfWorkspace.includes('formal installed executable excludes desktop E2E bridge'), 'installed PDF workspace must verify the production bridge boundary')
assert(installedPdfWorkspace.includes('__TAURI_IPC__/Event.emit'), 'installed PDF workspace must use production Tauri file-drop IPC')
assert(
  installedReleaseGate.includes('$baselineAutoStartRegistration = Get-AutoStartRegistration'),
  'installed lifecycle must snapshot the user auto-start registration before mutation',
)
assert(
  (installedReleaseGate.match(/Restore-AutoStartRegistration \$baselineAutoStartRegistration/g) ?? []).length === 2,
  'installed lifecycle must restore auto-start after both success and recovery',
)
assert(
  installedReleaseGate.includes("Add-Check 'baseline auto-start registration is restored'"),
  'installed lifecycle must audit the exact restored auto-start registration',
)
assert(
  installedReleaseGate.includes('$sourceBefore -eq $sourceAfter -and $sourceBefore -eq $backupFingerprint'),
  'installed lifecycle must prove its user-data backup is a stable byte snapshot',
)
assert(
  installedReleaseGate.includes('User-data target was recreated during restore'),
  'installed lifecycle must refuse a restore race instead of nesting backup data',
)
assert(
  installedReleaseGate.includes('Move-Item -LiteralPath $staging -Destination $path'),
  'installed lifecycle must replace restored user data from verified staging',
)
assert(
  packageManifest.scripts?.['test:e2e:desktop:video-workspace'] === 'npm run test:video-long-large:real && node scripts/test-tauri-desktop.mjs --video-workspace-only',
  'C-05.1/C-05.3 real desktop video gate is missing its reproducible cancellation fixture',
)
const desktopVideoGate = await read('scripts/test-tauri-desktop.mjs')
assert(desktopVideoGate.includes('runVideoWorkspaceDesktopGate'), 'C-05.1 desktop video gate implementation is missing')
assert(desktopVideoGate.includes('real video batch started'), 'C-05.1 desktop gate must start the visible product batch')
assert(desktopVideoGate.includes("record.workloadKind === 'video'"), 'C-05.1 desktop gate must verify unified video history')
assert(desktopVideoGate.includes("const productFfprobe = path.join"), 'C-05.1 desktop gate must independently probe published outputs')
assert(desktopVideoGate.includes("assert.equal(video?.codec_name, 'h264')"), 'C-05.1 desktop gate must verify H.264 publication')
assert(desktopVideoGate.includes("assert.equal(audio?.codec_name, 'aac')"), 'C-05.1 desktop gate must verify AAC publication')
assert(desktopVideoGate.includes('planning must not write task history'), 'video planning must still enforce zero history writes')
assert(desktopVideoGate.includes('videoFfmpegProcessIds(cancellationVideo)'), 'C-05.3 must observe and terminate the real product FFmpeg process')
assert(desktopVideoGate.includes("candidate.status === 'cancelled'"), 'C-05.3 must persist cancelled video history')
assert(desktopVideoGate.includes('await restartDesktopSession()'), 'C-05.3 must perform a complete application restart')
assert(desktopVideoGate.includes('measured video history must survive a complete app restart'), 'C-05.3 must compare measured history across restart')
assert(desktopVideoGate.includes('video-open-default-app'), 'C-05.3 must invoke default-application playback from the visible result')
assert(
  (main.match(/commands::system_integration::open_video_output_with_default_application/g) ?? []).length === 1,
  'C-05.3 must expose exactly one restricted video default-application command',
)
const systemIntegrationCommands = await read('src-tauri/src/commands/system_integration.rs')
for (const contract of [
  'VIDEO_DEFAULT_OPEN_PATH_MUST_BE_ABSOLUTE',
  'VIDEO_DEFAULT_OPEN_REQUIRES_REGULAR_FILE',
  'VIDEO_DEFAULT_OPEN_REQUIRES_MP4',
  'archive_entry_open::open_with_default_application(&path)',
]) {
  assert(systemIntegrationCommands.includes(contract), `C-05.3 default-open contract is missing: ${contract}`)
}
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
  || /image[_-]?compression/i.test(file)
  || /video[_-]?(?:engine|probe|compression)/i.test(file)
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
