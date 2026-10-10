$ErrorActionPreference = 'Stop'
$project = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$binary = Join-Path $project 'src-tauri\target\release\long-compress-assistant.exe'
$node = (Get-Command node).Source
$evidence = Join-Path $project 'test-results\optimization-resource-audit'
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
$samples = @()
foreach ($round in 1..3) {
  foreach ($parallel in 0..1) {
    $env:LONG_OPTIMIZATION_PARALLEL = "$parallel"
    $outLog = Join-Path $evidence "round-$round-parallel-$parallel.log"
    $errLog = Join-Path $evidence "round-$round-parallel-$parallel.stderr.log"
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $job = Start-Process -FilePath $node -ArgumentList @('scripts/test-tauri-desktop.mjs', '--optimization-only') -WorkingDirectory $project -WindowStyle Hidden -PassThru -RedirectStandardOutput $outLog -RedirectStandardError $errLog
    $null = $job.Handle
    $peakWorking = 0L
    $peakPrivate = 0L
    $observedCpuSeconds = 0.0
    $sampleCount = 0
    while (-not $job.HasExited) {
      foreach ($candidate in @(Get-Process -Name 'long-compress-assistant' -ErrorAction SilentlyContinue)) {
        if ($candidate.Path -eq $binary) {
          try {
            $peakWorking = [Math]::Max($peakWorking, $candidate.PeakWorkingSet64)
            $peakPrivate = [Math]::Max($peakPrivate, $candidate.PrivateMemorySize64)
            $observedCpuSeconds = [Math]::Max($observedCpuSeconds, $candidate.TotalProcessorTime.TotalSeconds)
            $sampleCount++
          } catch { }
        }
      }
      Start-Sleep -Milliseconds 75
      $job.Refresh()
    }
    $job.WaitForExit()
    $watch.Stop()
    if ($job.ExitCode -ne 0) { throw "Native workflow failed. See $outLog and $errLog" }
    if ($sampleCount -eq 0) { throw 'No native process resource samples were captured.' }
    $samples += [ordered]@{ round=$round; parallel=($parallel -eq 1); processPeakWorkingSetBytes=$peakWorking; sampledPeakPrivateBytes=$peakPrivate; observedCpuSeconds=$observedCpuSeconds; samples=$sampleCount; workflowWallMs=$watch.ElapsedMilliseconds }
    Write-Output ($samples[-1] | ConvertTo-Json -Compress)
  }
}
[ordered]@{
  verifiedAt=(Get-Date).ToUniversalTime().ToString('o'); binarySha256=(Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash;
  scope='Native process for the entire real UI retry/ZIP/conflict/smart-directory workflow; excludes WebView2 child processes. 75 ms polling; CPU is last observed accumulated CPU seconds, not percent. Includes startup, fixture processing and UI waits; not extraction speed or physical disk I/O.';
  samples=$samples
} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $evidence 'result.json') -Encoding utf8
