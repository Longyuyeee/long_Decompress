param()

$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') {
  throw 'D-03.3.1 VHD test is restricted to an isolated GitHub Actions Windows runner.'
}
if (-not $env:RUNNER_TEMP) {
  throw 'RUNNER_TEMP is required for the isolated VHD boundary.'
}

$runnerTemp = [System.IO.Path]::GetFullPath($env:RUNNER_TEMP)
$testRoot = Join-Path $runnerTemp "long-pdf-low-capacity-$([guid]::NewGuid().ToString('N'))"
$testRoot = [System.IO.Path]::GetFullPath($testRoot)
if (-not $testRoot.StartsWith($runnerTemp, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "Resolved test root escaped RUNNER_TEMP: $testRoot"
}
$vhdPath = Join-Path $testRoot 'low-capacity.vhdx'
$mountPath = Join-Path $testRoot 'mount'
$createScript = Join-Path $testRoot 'create-vhd.txt'
$cleanupScript = Join-Path $testRoot 'cleanup-vhd.txt'

New-Item -ItemType Directory -Path $mountPath -Force | Out-Null
try {
  @(
    "create vdisk file=`"$vhdPath`" maximum=96 type=expandable"
    "select vdisk file=`"$vhdPath`""
    'attach vdisk'
    'create partition primary'
    'format fs=ntfs quick label=LONGPDFLOW'
    "assign mount=`"$mountPath`""
  ) | Set-Content -LiteralPath $createScript -Encoding ascii
  $createOutput = & diskpart.exe /s $createScript 2>&1
  $mountedVolume = Get-Volume -FilePath $mountPath -ErrorAction SilentlyContinue
  if (
    $LASTEXITCODE -ne 0 -or
    -not $mountedVolume -or
    $mountedVolume.FileSystemLabel -ne 'LONGPDFLOW' -or
    $mountedVolume.Size -ge 128MB
  ) {
    throw "Unable to create isolated low-capacity VHD: $($createOutput -join [Environment]::NewLine)"
  }

  node scripts/run-d03-pdf-low-capacity.mjs "--volume=$mountPath"
  if ($LASTEXITCODE -ne 0) {
    throw "D-03.3.1 runner failed with exit code $LASTEXITCODE"
  }
}
finally {
  if (Test-Path -LiteralPath $vhdPath) {
    @(
      "select vdisk file=`"$vhdPath`""
      'detach vdisk'
    ) | Set-Content -LiteralPath $cleanupScript -Encoding ascii
    & diskpart.exe /s $cleanupScript | Out-Null
  }
  if (Test-Path -LiteralPath $testRoot) {
    Remove-Item -LiteralPath $testRoot -Recurse -Force
  }
}
