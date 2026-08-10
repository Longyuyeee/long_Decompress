param(
  [string]$TargetRoot,

  [string]$CrossSourceRoot,

  [ValidateRange(1, 20)]
  [int]$Iterations = 10,

  [ValidateRange(16, 2048)]
  [int]$LargeFileMiB = 100,

  [ValidateRange(1000, 50000)]
  [int]$SmallFileCount = 10000,

  [string]$OutputDirectory,

  [string]$ExistingSameVolumeResult,

  [string]$ExistingCrossPhysicalDiskResult
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$baselineScript = Join-Path $PSScriptRoot 'run-performance-baseline.ps1'
if (-not (Test-Path -LiteralPath $baselineScript -PathType Leaf)) {
  throw "Performance baseline script does not exist: $baselineScript"
}
if (-not $OutputDirectory) {
  $runId = Get-Date -Format 'yyyyMMdd-HHmmss'
  $OutputDirectory = Join-Path $projectRoot "test-results\performance-baseline\io-matrix\$runId"
}
$analyzeExisting = ($ExistingSameVolumeResult -or $ExistingCrossPhysicalDiskResult)
if ($analyzeExisting -and -not ($ExistingSameVolumeResult -and $ExistingCrossPhysicalDiskResult)) {
  throw 'ExistingSameVolumeResult and ExistingCrossPhysicalDiskResult must be provided together.'
}
if (-not $analyzeExisting -and (-not $TargetRoot -or -not $CrossSourceRoot)) {
  throw 'TargetRoot and CrossSourceRoot are required when collecting a new matrix.'
}
$resolvedOutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
$sameVolumePath = if ($analyzeExisting) {
  [IO.Path]::GetFullPath($ExistingSameVolumeResult)
} else {
  Join-Path $resolvedOutputDirectory 'same-volume.json'
}
$crossPhysicalPath = if ($analyzeExisting) {
  [IO.Path]::GetFullPath($ExistingCrossPhysicalDiskResult)
} else {
  Join-Path $resolvedOutputDirectory 'cross-physical-disk.json'
}
$matrixPath = Join-Path $resolvedOutputDirectory 'matrix.json'

$pathsToCreate = @($matrixPath)
if (-not $analyzeExisting) {
  $pathsToCreate += @($sameVolumePath, $crossPhysicalPath)
}
foreach ($path in $pathsToCreate) {
  if (Test-Path -LiteralPath $path) {
    throw "Refusing to overwrite an existing matrix result: $path"
  }
}

$initialDirty = @(& git -C $projectRoot status --porcelain).Count -gt 0
if ($Iterations -ge 10 -and $initialDirty) {
  throw 'A qualified I/O matrix requires a clean Git worktree before sampling starts.'
}
if ($Iterations -ge 10) {
  $repositoryRoot = [IO.Path]::GetFullPath((& git -C $projectRoot rev-parse --show-toplevel).Trim())
  if (-not $analyzeExisting) {
    foreach ($benchmarkRoot in @(
      [pscustomobject]@{ label = 'TargetRoot'; path = $TargetRoot },
      [pscustomobject]@{ label = 'CrossSourceRoot'; path = $CrossSourceRoot }
    )) {
      $resolvedBenchmarkRoot = [IO.Path]::GetFullPath($benchmarkRoot.path)
      $rootInsideRepository = (
        $resolvedBenchmarkRoot.Equals($repositoryRoot, [StringComparison]::OrdinalIgnoreCase) -or
        $resolvedBenchmarkRoot.StartsWith(
          $repositoryRoot.TrimEnd('\') + '\',
          [StringComparison]::OrdinalIgnoreCase
        )
      )
      if ($rootInsideRepository) {
        $probePath = Join-Path $resolvedBenchmarkRoot 'long-decompress-qualification-probe'
        & git -C $projectRoot check-ignore --quiet -- $probePath
        if ($LASTEXITCODE -ne 0) {
          throw "$($benchmarkRoot.label) would create untracked fixtures inside the Git repository."
        }
      }
    }
  }
  $outputInsideRepository = (
    $resolvedOutputDirectory.Equals($repositoryRoot, [StringComparison]::OrdinalIgnoreCase) -or
    $resolvedOutputDirectory.StartsWith(
      $repositoryRoot.TrimEnd('\') + '\',
      [StringComparison]::OrdinalIgnoreCase
    )
  )
  if ($outputInsideRepository) {
    foreach ($path in $pathsToCreate) {
      & git -C $projectRoot check-ignore --quiet -- $path
      if ($LASTEXITCODE -ne 0) {
        throw "A qualified matrix output inside the repository must be ignored by Git: $path"
      }
    }
  }
}

[IO.Directory]::CreateDirectory($resolvedOutputDirectory) | Out-Null

function Invoke-IoBaseline {
  param(
    [string]$Label,
    [string]$Source,
    [string]$Target,
    [string]$OutputPath
  )
  Write-Output "[io-matrix] scenario=$Label source=$Source target=$Target"
  & $baselineScript `
    -IoTopologyOnly `
    -Iterations $Iterations `
    -LargeFileMiB $LargeFileMiB `
    -SmallFileCount $SmallFileCount `
    -SourceRoot $Source `
    -TargetRoot $Target `
    -OutputPath $OutputPath
}

function Read-BaselineResult {
  param([string]$Path)
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "Expected baseline result was not created: $Path"
  }
  return Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json
}

function Assert-SameValue {
  param($Left, $Right, [string]$Description)
  if ($Left -ne $Right) {
    throw "I/O matrix results disagree on ${Description}: '$Left' versus '$Right'."
  }
}

function Get-MetricSummary {
  param(
    $SameResult,
    $CrossResult,
    [string]$Scenario,
    [string]$Metric
  )
  $sameMetric = $SameResult.aggregates.$Scenario.metrics.$Metric
  $crossMetric = $CrossResult.aggregates.$Scenario.metrics.$Metric
  $sameMedian = [double]$sameMetric.median
  $crossMedian = [double]$crossMetric.median
  if ($sameMedian -le 0 -or $crossMedian -le 0) {
    throw "I/O matrix metric must have positive medians: $Scenario/$Metric"
  }
  return [ordered]@{
    scenario = $Scenario
    metric = $Metric
    same_volume_median = $sameMedian
    cross_physical_disk_median = $crossMedian
    cross_change_percent = (($crossMedian - $sameMedian) / $sameMedian) * 100
    same_volume_range_percent = (
      ([double]$sameMetric.maximum - [double]$sameMetric.minimum) / $sameMedian
    ) * 100
    cross_physical_disk_range_percent = (
      ([double]$crossMetric.maximum - [double]$crossMetric.minimum) / $crossMedian
    ) * 100
    higher_is_better = $true
  }
}

if (-not $analyzeExisting) {
  Invoke-IoBaseline `
    -Label 'same_volume' `
    -Source $TargetRoot `
    -Target $TargetRoot `
    -OutputPath $sameVolumePath
  Invoke-IoBaseline `
    -Label 'cross_physical_disk' `
    -Source $CrossSourceRoot `
    -Target $TargetRoot `
    -OutputPath $crossPhysicalPath
} else {
  Write-Output "[io-matrix] analyzing existing qualified results without resampling"
}

$same = Read-BaselineResult -Path $sameVolumePath
$cross = Read-BaselineResult -Path $crossPhysicalPath

foreach ($result in @($same, $cross)) {
  if ([int]$result.schema_version -ne 1) {
    throw "Unsupported baseline schema version: $($result.schema_version)"
  }
  if ($result.configuration.workload_profile -ne 'zip_io_topology') {
    throw 'I/O matrix received a result from the wrong workload profile.'
  }
  Assert-SameValue $result.configuration.iterations $Iterations 'iteration count'
  Assert-SameValue $result.configuration.large_file_mib $LargeFileMiB 'large-file size'
  Assert-SameValue $result.configuration.small_file_count $SmallFileCount 'small-file count'
}

if ($same.storage.relation -ne 'same_volume') {
  throw "Expected same_volume topology, observed: $($same.storage.relation)"
}
if ($cross.storage.relation -ne 'cross_physical_disk') {
  throw "Expected cross_physical_disk topology, observed: $($cross.storage.relation)"
}
Assert-SameValue $same.machine.id $cross.machine.id 'machine identity'
Assert-SameValue $same.git.commit $cross.git.commit 'Git commit'
Assert-SameValue $same.app_version $cross.app_version 'application version'
Assert-SameValue $same.storage.target.volume_id $cross.storage.target.volume_id 'target volume'
Assert-SameValue $same.storage.target.disk_id $cross.storage.target.disk_id 'target physical disk'

if ($cross.storage.source.disk_id -eq $cross.storage.target.disk_id) {
  throw 'Cross-physical-disk scenario resolved to the same physical disk.'
}
if (
  $same.storage.source.medium -eq 'unknown' -or
  $same.storage.target.medium -eq 'unknown' -or
  $cross.storage.source.medium -eq 'unknown' -or
  $cross.storage.target.medium -eq 'unknown'
) {
  throw 'I/O matrix requires Windows to prove the media type of every endpoint.'
}

$metrics = @(
  Get-MetricSummary $same $cross 'zip_large_file' 'compression_mib_s'
  Get-MetricSummary $same $cross 'zip_large_file' 'extraction_mib_s'
  Get-MetricSummary $same $cross 'zip_many_small_files' 'compression_files_s'
  Get-MetricSummary $same $cross 'zip_many_small_files' 'extraction_files_s'
)
$qualified = (
  $Iterations -ge 10 -and
  $same.qualification.threshold_eligible -eq $true -and
  $cross.qualification.threshold_eligible -eq $true -and
  $same.git.dirty -eq $false -and
  $cross.git.dirty -eq $false
)
if ($Iterations -ge 10 -and -not $qualified) {
  throw 'The formal I/O matrix completed, but one or more results were not qualified.'
}

$matrix = [ordered]@{
  schema_version = 1
  generated_at = (Get-Date).ToUniversalTime().ToString('o')
  app_version = $same.app_version
  git = $same.git
  machine = $same.machine
  configuration = [ordered]@{
    iterations = $Iterations
    large_file_mib = $LargeFileMiB
    small_file_count = $SmallFileCount
    direction = 'source_to_shared_target'
  }
  qualification = [ordered]@{
    required_sample_count = 10
    threshold_eligible = $qualified
    note = if ($qualified) {
      'Qualified fixed-machine topology baseline; not evidence for changing scheduling defaults.'
    } else {
      'Observation only. Run at least 10 samples from a clean committed worktree.'
    }
  }
  coverage = [ordered]@{
    same_volume = $true
    cross_physical_disk = $true
    source_media = @($same.storage.source.medium, $cross.storage.source.medium)
    target_medium = $same.storage.target.medium
    hdd_covered = (
      @(
        $same.storage.source.medium,
        $same.storage.target.medium,
        $cross.storage.source.medium,
        $cross.storage.target.medium
      ) -contains 'hdd'
    )
  }
  results = [ordered]@{
    same_volume_path = $sameVolumePath
    cross_physical_disk_path = $crossPhysicalPath
    same_volume_storage = $same.storage
    cross_physical_disk_storage = $cross.storage
  }
  throughput_comparisons = $metrics
  scheduling_decision = [ordered]@{
    status = 'baseline_only'
    change_default_concurrency = $false
    reason = 'This matrix compares storage topology, not competing task scheduling policies.'
  }
}

[IO.File]::WriteAllText(
  $matrixPath,
  ($matrix | ConvertTo-Json -Depth 12),
  [Text.UTF8Encoding]::new($false)
)

Write-Output "I/O matrix result: $matrixPath"
Write-Output "Qualified: $qualified; HDD covered: $($matrix.coverage.hdd_covered)"
foreach ($metric in $metrics) {
  Write-Output (
    '{0}/{1}: same={2:N2}, cross={3:N2}, change={4:N2}%' -f (
      $metric.scenario,
      $metric.metric,
      $metric.same_volume_median,
      $metric.cross_physical_disk_median,
      $metric.cross_change_percent
    )
  )
}
