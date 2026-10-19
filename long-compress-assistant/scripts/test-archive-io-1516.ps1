param()
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or -not $env:RUNNER_TEMP) { throw 'Only an isolated Actions runner may mount this test disk.' }
$runnerRoot = [IO.Path]::GetFullPath($env:RUNNER_TEMP)
$testRoot = [IO.Path]::GetFullPath((Join-Path $runnerRoot ('long-io-1516-' + [guid]::NewGuid().ToString('N'))))
if (-not $testRoot.StartsWith($runnerRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Test directory escaped RUNNER_TEMP.' }
$mount = Join-Path $testRoot 'mount'
$disk = Join-Path $testRoot 'io.vhdx'
New-Item -ItemType Directory -Path $mount -Force | Out-Null
try {
  $create = Join-Path $testRoot 'create.txt'
  @(('create vdisk file="{0}" maximum=256 type=expandable' -f $disk), ('select vdisk file="{0}"' -f $disk), 'attach vdisk', 'create partition primary', 'format fs=ntfs quick label=LONGIO1516', ('assign mount="{0}"' -f $mount)) | Set-Content -LiteralPath $create -Encoding ascii
  & diskpart.exe /s $create
  $volume = Get-Volume -FilePath $mount -ErrorAction Stop
  if ($LASTEXITCODE -ne 0 -or $volume.FileSystemLabel -ne 'LONGIO1516' -or $volume.Size -ge 300MB) { throw 'Not the bounded owned test volume.' }
  New-Item -ItemType Directory -Path 'test-results/io-1516' -Force | Out-Null
  $env:IO1516_TEST_VOLUME = $mount
  $env:IO1516_RECEIPT = Join-Path (Get-Location) 'test-results/io-1516/real-disk.json'
  cargo test --manifest-path src-tauri/Cargo.toml --lib io1516_real_disk_full --locked -- --ignored --nocapture *> test-results/io-1516/real-disk.log
  $testExit = $LASTEXITCODE
  Get-Content -LiteralPath 'test-results/io-1516/real-disk.log'
  if ($testExit -ne 0) { throw "Real disk test failed ($testExit)" }
}
finally {
  if (Test-Path -LiteralPath $disk) {
    $detach = Join-Path $testRoot 'detach.txt'
    @(('select vdisk file="{0}"' -f $disk), 'detach vdisk') | Set-Content -LiteralPath $detach -Encoding ascii
    & diskpart.exe /s $detach | Out-Null
  }
  $checkedRoot = [IO.Path]::GetFullPath($testRoot)
  if (-not $checkedRoot.StartsWith($runnerRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Cleanup path escaped the runner directory.' }
  if (Test-Path -LiteralPath $checkedRoot) { Remove-Item -LiteralPath $checkedRoot -Recurse -Force }
}
