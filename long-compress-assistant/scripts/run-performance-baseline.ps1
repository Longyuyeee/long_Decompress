param(
  [ValidateRange(1, 20)]
  [int]$Iterations = 3,

  [ValidateRange(16, 2048)]
  [int]$LargeFileMiB = 100,

  [ValidateRange(1000, 50000)]
  [int]$SmallFileCount = 10000,

  [switch]$SkipAes,

  [string]$BaselinePath,

  [ValidateRange(1, 100)]
  [double]$RegressionThresholdPercent = 25,

  [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$manifestPath = Join-Path $projectRoot 'src-tauri\Cargo.toml'
if (-not $OutputPath) {
  $runId = Get-Date -Format 'yyyyMMdd-HHmmss'
  $OutputPath = Join-Path $projectRoot "test-results\performance-baseline\$runId\result.json"
}
$resolvedOutputPath = [IO.Path]::GetFullPath($OutputPath)
$resolvedBaselinePath = if ($BaselinePath) { [IO.Path]::GetFullPath($BaselinePath) } else { $null }

function Get-Median {
  param([double[]]$Values)
  if ($Values.Count -eq 0) { return $null }
  $sorted = @($Values | Sort-Object)
  $middle = [int][Math]::Floor($sorted.Count / 2)
  if ($sorted.Count % 2 -eq 1) { return [double]$sorted[$middle] }
  return ([double]$sorted[$middle - 1] + [double]$sorted[$middle]) / 2
}

function Invoke-PerformanceScenario {
  param(
    [string]$TestTarget,
    [string]$TestName,
    [int]$Iteration
  )
  Write-Host "[performance] iteration=$Iteration target=$TestTarget test=$TestName"
  $arguments = @(
    'test', '--release', '--manifest-path', $manifestPath,
    '--test', $TestTarget, $TestName,
    '--', '--ignored', '--exact', '--nocapture'
  )
  # Cargo writes normal compilation progress to stderr. Windows PowerShell 5.1
  # turns redirected native stderr into error records when the global policy is
  # Stop, so judge the command by its process exit code instead.
  $previousErrorActionPreference = $ErrorActionPreference
  try {
    $ErrorActionPreference = 'Continue'
    $output = @(& cargo @arguments 2>&1)
    $exitCode = $LASTEXITCODE
  } finally {
    $ErrorActionPreference = $previousErrorActionPreference
  }
  $output | ForEach-Object { Write-Host ([string]$_) }
  if ($exitCode -ne 0) {
    throw "Performance scenario failed: $TestTarget/$TestName (exit $exitCode)"
  }
  $matches = @(
    $output |
      ForEach-Object { [regex]::Match([string]$_, 'PERF_JSON\s+(\{.*\})') } |
      Where-Object Success
  )
  if ($matches.Count -ne 1) {
    throw "Expected one PERF_JSON record from $TestTarget/$TestName, found $($matches.Count)."
  }
  $record = $matches[0].Groups[1].Value | ConvertFrom-Json
  $record | Add-Member -NotePropertyName iteration -NotePropertyValue $Iteration
  $record | Add-Member -NotePropertyName captured_at -NotePropertyValue (
    (Get-Date).ToUniversalTime().ToString('o')
  )
  return $record
}

function Get-MachineIdentity {
  $operatingSystem = Get-CimInstance Win32_OperatingSystem
  $processor = @(Get-CimInstance Win32_Processor | Select-Object -First 1)[0]
  $logicalProcessors = [Environment]::ProcessorCount
  $memoryBytes = [int64]$operatingSystem.TotalVisibleMemorySize * 1024
  $powerPlan = (& powercfg.exe /getactivescheme 2>$null | Out-String).Trim()
  $identity = "{0}|{1}|{2}|{3}|{4}" -f (
    $processor.Name.Trim(),
    $logicalProcessors,
    $memoryBytes,
    $operatingSystem.BuildNumber,
    [Runtime.InteropServices.RuntimeInformation]::OSArchitecture
  )
  return [ordered]@{
    id = $identity
    os_caption = [string]$operatingSystem.Caption
    os_version = [string]$operatingSystem.Version
    os_build = [string]$operatingSystem.BuildNumber
    architecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    cpu = $processor.Name.Trim()
    logical_processors = $logicalProcessors
    memory_bytes = $memoryBytes
    power_plan = $powerPlan
  }
}

function Get-Aggregates {
  param([object[]]$Samples)
  $aggregate = [ordered]@{}
  foreach ($scenario in @($Samples.scenario | Sort-Object -Unique)) {
    $scenarioSamples = @($Samples | Where-Object scenario -eq $scenario)
    $metricNames = switch ($scenario) {
      'zip_large_file' {
        @('compression_mib_s', 'extraction_mib_s', 'compression_ms', 'extraction_ms', 'peak_working_set_delta_mib')
      }
      'zip_many_small_files' {
        @('compression_files_s', 'extraction_files_s', 'compression_ms', 'extraction_ms', 'peak_working_set_delta_mib')
      }
      'aes_v2_large_file' {
        @('encryption_mib_s', 'decryption_mib_s', 'encryption_ms', 'decryption_ms', 'peak_working_set_delta_mib')
      }
      default { @() }
    }
    $metrics = [ordered]@{}
    foreach ($metric in $metricNames) {
      $values = [double[]]@($scenarioSamples | ForEach-Object { [double]$_.$metric })
      $metrics[$metric] = [ordered]@{
        median = Get-Median $values
        minimum = [double]($values | Measure-Object -Minimum).Minimum
        maximum = [double]($values | Measure-Object -Maximum).Maximum
      }
    }
    $aggregate[$scenario] = [ordered]@{
      sample_count = $scenarioSamples.Count
      metrics = $metrics
    }
  }
  return $aggregate
}

function Compare-Baseline {
  param($Current, $Baseline, [double]$ThresholdPercent)
  $comparisons = [Collections.Generic.List[object]]::new()
  $regressions = [Collections.Generic.List[string]]::new()
  $throughputSuffixes = @('_mib_s', '_files_s')
  foreach ($scenario in @($Current.Keys)) {
    $currentScenario = $Current[$scenario]
    $baselineScenarioProperty = $Baseline.PSObject.Properties[$scenario]
    $baselineScenario = if ($null -ne $baselineScenarioProperty) {
      $baselineScenarioProperty.Value
    } else {
      $null
    }
    if ($null -eq $baselineScenario) {
      throw "Qualified baseline is missing scenario: $scenario"
    }
    foreach ($metric in @($currentScenario.metrics.Keys)) {
      $baselineMetricProperty = $baselineScenario.metrics.PSObject.Properties[$metric]
      $baselineMetric = if ($null -ne $baselineMetricProperty) {
        $baselineMetricProperty.Value
      } else {
        $null
      }
      if ($null -eq $baselineMetric) {
        throw "Qualified baseline is missing metric: $scenario/$metric"
      }
      if ([double]$baselineMetric.median -eq 0) {
        throw "Qualified baseline metric has a zero median: $scenario/$metric"
      }
      $currentMedian = [double]$currentScenario.metrics[$metric].median
      $baselineMedian = [double]$baselineMetric.median
      $changePercent = (($currentMedian - $baselineMedian) / $baselineMedian) * 100
      $higherIsBetter = @($throughputSuffixes | Where-Object { $metric.EndsWith($_) }).Count -gt 0
      $regressed = if ($higherIsBetter) {
        $changePercent -lt -$ThresholdPercent
      } else {
        $changePercent -gt $ThresholdPercent
      }
      [void]$comparisons.Add([pscustomobject]@{
        scenario = $scenario
        metric = $metric
        baseline_median = $baselineMedian
        current_median = $currentMedian
        change_percent = $changePercent
        higher_is_better = $higherIsBetter
        regressed = $regressed
      })
      if ($regressed) { [void]$regressions.Add("$scenario/$metric") }
    }
  }
  return [pscustomobject]@{
    comparisons = @($comparisons)
    regressions = @($regressions)
  }
}

$previousLargeSize = [Environment]::GetEnvironmentVariable('LONG_DECOMPRESS_PERF_SIZE_MIB')
$previousFileCount = [Environment]::GetEnvironmentVariable('LONG_DECOMPRESS_PERF_FILE_COUNT')
$previousAesSize = [Environment]::GetEnvironmentVariable('LONG_DECOMPRESS_PERF_AES_SIZE_MIB')
$machine = Get-MachineIdentity
$baseline = $null
if ($resolvedBaselinePath) {
  if (-not (Test-Path -LiteralPath $resolvedBaselinePath -PathType Leaf)) {
    throw "Baseline file does not exist: $resolvedBaselinePath"
  }
  $baseline = Get-Content -Raw -Encoding utf8 -LiteralPath $resolvedBaselinePath | ConvertFrom-Json
  if ([int]$baseline.schema_version -ne 1) {
    throw "Unsupported baseline schema version: $($baseline.schema_version)"
  }
  if ($baseline.machine.id -ne $machine.id) {
    throw 'Baseline machine identity does not match this machine.'
  }
  if (
    [int]$baseline.configuration.large_file_mib -ne $LargeFileMiB -or
    [int]$baseline.configuration.small_file_count -ne $SmallFileCount -or
    [bool]$baseline.configuration.aes_included -ne (-not $SkipAes)
  ) {
    throw 'Baseline workload configuration does not match this run.'
  }
}
$samples = [Collections.Generic.List[object]]::new()
try {
  $env:LONG_DECOMPRESS_PERF_SIZE_MIB = [string]$LargeFileMiB
  $env:LONG_DECOMPRESS_PERF_FILE_COUNT = [string]$SmallFileCount
  $env:LONG_DECOMPRESS_PERF_AES_SIZE_MIB = [string]$LargeFileMiB
  for ($iteration = 1; $iteration -le $Iterations; $iteration += 1) {
    [void]$samples.Add((Invoke-PerformanceScenario `
      'archive_performance_regression' 'real_zip_compress_extract_baseline' $iteration))
    [void]$samples.Add((Invoke-PerformanceScenario `
      'archive_performance_regression' 'real_zip_many_small_files_baseline' $iteration))
    if (-not $SkipAes) {
      [void]$samples.Add((Invoke-PerformanceScenario `
        'aes_stream_performance' 'real_aes_stream_100_mib_baseline' $iteration))
    }
  }
} finally {
  [Environment]::SetEnvironmentVariable('LONG_DECOMPRESS_PERF_SIZE_MIB', $previousLargeSize)
  [Environment]::SetEnvironmentVariable('LONG_DECOMPRESS_PERF_FILE_COUNT', $previousFileCount)
  [Environment]::SetEnvironmentVariable('LONG_DECOMPRESS_PERF_AES_SIZE_MIB', $previousAesSize)
}

$aggregates = Get-Aggregates -Samples @($samples)
$gitCommit = (& git -C $projectRoot rev-parse HEAD).Trim()
$gitDirty = @(& git -C $projectRoot status --porcelain).Count -gt 0
$baselineComparison = $null
$thresholdApplied = $false
$regressions = @()
$comparisonMetrics = [object[]]@()
if ($resolvedBaselinePath) {
  $baselineEligible = (
    $baseline.qualification.threshold_eligible -eq $true -and
    [int]$baseline.qualification.sample_count -ge 10
  )
  $thresholdApplied = $baselineEligible -and $Iterations -ge 10
  if ($thresholdApplied) {
    $baselineComparison = Compare-Baseline $aggregates $baseline.aggregates $RegressionThresholdPercent
    $comparisonMetrics = [object[]]@($baselineComparison.comparisons)
    $regressions = @($baselineComparison.regressions)
  }
}

$result = [ordered]@{
  schema_version = 1
  generated_at = (Get-Date).ToUniversalTime().ToString('o')
  app_version = (Get-Content -Raw -Encoding utf8 (Join-Path $projectRoot 'package.json') | ConvertFrom-Json).version
  git = [ordered]@{
    commit = $gitCommit
    dirty = $gitDirty
  }
  machine = $machine
  toolchain = [ordered]@{
    rustc = (& rustc --version).Trim()
    cargo = (& cargo --version).Trim()
  }
  configuration = [ordered]@{
    iterations = $Iterations
    large_file_mib = $LargeFileMiB
    small_file_count = $SmallFileCount
    aes_included = -not $SkipAes
    regression_threshold_percent = $RegressionThresholdPercent
  }
  qualification = [ordered]@{
    sample_count = $Iterations
    required_sample_count = 10
    threshold_eligible = $Iterations -ge 10
    note = if ($Iterations -ge 10) {
      'Eligible as a fixed-machine warning baseline.'
    } else {
      'Observation only. Accumulate at least 10 fixed-machine samples before enabling warnings.'
    }
  }
  samples = @($samples)
  aggregates = $aggregates
  comparison = [ordered]@{
    baseline_path = $resolvedBaselinePath
    threshold_applied = $thresholdApplied
    metrics = $comparisonMetrics
    regressions = $regressions
  }
}

$outputDirectory = Split-Path -Parent $resolvedOutputPath
if ($outputDirectory) { [IO.Directory]::CreateDirectory($outputDirectory) | Out-Null }
[IO.File]::WriteAllText(
  $resolvedOutputPath,
  ($result | ConvertTo-Json -Depth 12),
  [Text.UTF8Encoding]::new($false)
)
Write-Output "Performance baseline result: $resolvedOutputPath"
Write-Output "Samples per scenario: $Iterations; threshold eligible: $($result.qualification.threshold_eligible)"
if ($regressions.Count -gt 0) {
  throw "Performance regressions exceeded $RegressionThresholdPercent%: $($regressions -join ', ')"
}
