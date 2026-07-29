param(
  [Parameter(Mandatory = $true)]
  [string]$PreviousVersion,
  [Parameter(Mandatory = $true)]
  [string]$TargetVersion
)

$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
$resultsRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'test-results\public-update-validation'))
$uiArtifactDirectory = Join-Path $projectRoot 'test-results\public-updater-ui'
$driverPidPath = Join-Path $uiArtifactDirectory 'tauri-driver.pid'
$activeWebviewProfilePath = Join-Path $uiArtifactDirectory 'active-webview-profile.txt'
$runId = Get-Date -Format 'yyyyMMdd-HHmmss'
$evidenceDirectory = Join-Path $resultsRoot $runId
$backupRoot = Join-Path ([IO.Path]::GetTempPath()) "long-decompress-public-update-$([guid]::NewGuid().ToString('N'))"
$productName = "Long$([char]0x89E3)$([char]0x538B)"
$applicationName = "$productName.exe"
$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$productName"
$appDataPaths = @(
  [IO.Path]::GetFullPath((Join-Path $env:APPDATA 'LongDecompress')),
  [IO.Path]::GetFullPath((Join-Path $env:APPDATA 'com.longcompress.assistant'))
)
$evidence = [ordered]@{
  schemaVersion = 1
  startedAt = (Get-Date).ToUniversalTime().ToString('o')
  previousVersion = $PreviousVersion
  targetVersion = $TargetVersion
  checks = @()
}
$validationSucceeded = $false

function Add-Check {
  param([string]$Name, [bool]$Passed, [string]$Detail)
  $script:evidence.checks += [ordered]@{
    name = $Name
    passed = $Passed
    detail = $Detail
  }
  if (-not $Passed) {
    throw "$Name failed: $Detail"
  }
}

function Get-DirectoryFingerprint {
  param([string]$Path)
  if (-not (Test-Path -LiteralPath $Path)) {
    return 'missing'
  }
  $root = [IO.Path]::GetFullPath($Path).TrimEnd('\')
  $records = Get-ChildItem -LiteralPath $root -Recurse -Force -File |
    Sort-Object FullName |
    ForEach-Object {
      $relative = $_.FullName.Substring($root.Length).TrimStart('\')
      "$relative|$($_.Length)|$((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash)"
    }
  $payload = [Text.Encoding]::UTF8.GetBytes(($records -join "`n"))
  $hasher = [Security.Cryptography.SHA256]::Create()
  try {
    return ([BitConverter]::ToString($hasher.ComputeHash($payload)) -replace '-', '')
  } finally {
    $hasher.Dispose()
  }
}

function Get-DataFingerprints {
  $result = [ordered]@{}
  foreach ($path in $appDataPaths) {
    $result[$path] = Get-DirectoryFingerprint $path
  }
  return $result
}

function Get-InstalledState {
  if (-not (Test-Path -LiteralPath $uninstallKey)) {
    return $null
  }
  $properties = Get-ItemProperty -LiteralPath $uninstallKey
  $installLocation = [IO.Path]::GetFullPath(([string]$properties.InstallLocation).Trim('"'))
  return [ordered]@{
    version = [string]$properties.DisplayVersion
    installLocation = $installLocation
    executable = Join-Path $installLocation $applicationName
    uninstaller = Join-Path $installLocation 'uninstall.exe'
  }
}

function Get-ContextMenuMode {
  $legacy = Test-Path -LiteralPath 'HKCU:\Software\Classes\*\shell\LongDecompress'
  $native = Test-Path -LiteralPath 'HKCU:\Software\Classes\*\shell\LongDecompressNative'
  if ($legacy -and $native) { return 'mixed' }
  if ($legacy) { return 'legacy' }
  if ($native) { return 'native' }
  return 'none'
}

function Compare-Fingerprints {
  param($Expected, $Actual)
  foreach ($path in $appDataPaths) {
    Add-Check "application update preserves $path" ($Expected[$path] -eq $Actual[$path]) (
      "expected=$($Expected[$path]); actual=$($Actual[$path])"
    )
  }
}

function Save-Evidence {
  New-Item -ItemType Directory -Path $evidenceDirectory -Force | Out-Null
  $evidence.finishedAt = (Get-Date).ToUniversalTime().ToString('o')
  $evidence.succeeded = $validationSucceeded
  $evidence.failedChecks = @($evidence.checks | Where-Object { -not $_.passed }).Count
  $json = $evidence | ConvertTo-Json -Depth 8
  [IO.File]::WriteAllText(
    (Join-Path $evidenceDirectory 'result.json'),
    $json,
    [Text.UTF8Encoding]::new($false)
  )
}

try {
  $manifest = Invoke-RestMethod (
    "https://github.com/Longyuyeee/long_Decompress/releases/latest/download/latest.json"
  )
  Add-Check 'latest manifest version matches target' ($manifest.version -eq $TargetVersion) (
    "expected=$TargetVersion; actual=$($manifest.version)"
  )
  $platform = $manifest.platforms.'windows-x86_64'
  Add-Check 'latest manifest has a signed Windows asset' (
    [bool]$platform.signature -and [bool]$platform.url
  ) ([string]$platform.url)

  $initialState = Get-InstalledState
  Add-Check 'previous release is installed' ($null -ne $initialState) $uninstallKey
  Add-Check 'installed baseline version matches' ($initialState.version -eq $PreviousVersion) (
    "expected=$PreviousVersion; actual=$($initialState.version)"
  )
  Add-Check 'installed application exists' (Test-Path -LiteralPath $initialState.executable) (
    $initialState.executable
  )
  Add-Check 'installed uninstaller exists' (Test-Path -LiteralPath $initialState.uninstaller) (
    $initialState.uninstaller
  )
  Add-Check 'no installed application process is running' (
    @(Get-CimInstance Win32_Process -Filter "Name = '$applicationName'" -ErrorAction SilentlyContinue).Count -eq 0
  ) $applicationName

  New-Item -ItemType Directory -Path $backupRoot -Force | Out-Null
  for ($index = 0; $index -lt $appDataPaths.Count; $index += 1) {
    if (Test-Path -LiteralPath $appDataPaths[$index]) {
      Copy-Item -LiteralPath $appDataPaths[$index] `
        -Destination (Join-Path $backupRoot "data-$index") -Recurse
    }
  }
  $baselineFingerprints = Get-DataFingerprints
  $evidence.baseline = [ordered]@{
    installedState = $initialState
    dataFingerprints = $baselineFingerprints
    contextMenuMode = Get-ContextMenuMode
  }

  $edgeDriver = if ($env:EDGE_DRIVER_PATH) {
    $env:EDGE_DRIVER_PATH
  } else {
    Join-Path $env:TEMP 'long-compress-edge-driver\msedgedriver.exe'
  }
  $tauriDriver = if ($env:TAURI_DRIVER_PATH) {
    $env:TAURI_DRIVER_PATH
  } else {
    Join-Path $env:USERPROFILE '.cargo\bin\tauri-driver.exe'
  }
  Add-Check 'Microsoft EdgeDriver exists' (Test-Path -LiteralPath $edgeDriver) $edgeDriver
  Add-Check 'tauri-driver exists' (Test-Path -LiteralPath $tauriDriver) $tauriDriver

  $env:PUBLIC_UPDATER_APP = $initialState.executable
  $env:PUBLIC_UPDATER_EXPECTED_VERSION = $TargetVersion
  $env:EDGE_DRIVER_PATH = $edgeDriver
  $env:TAURI_DRIVER_PATH = $tauriDriver
  Remove-Item -LiteralPath $driverPidPath,$activeWebviewProfilePath `
    -Force -ErrorAction SilentlyContinue
  & node.exe (Join-Path $PSScriptRoot 'run-public-updater-ui.mjs')
  Add-Check 'desktop updater UI completed its hand-off' ($LASTEXITCODE -eq 0) "exitCode=$LASTEXITCODE"

  $updateDeadline = (Get-Date).AddMinutes(4)
  do {
    Start-Sleep -Milliseconds 500
    $updatedState = Get-InstalledState
  } while (
    ($null -eq $updatedState -or $updatedState.version -ne $TargetVersion) -and
    (Get-Date) -lt $updateDeadline
  )
  Add-Check 'application update installs target version' (
    $null -ne $updatedState -and $updatedState.version -eq $TargetVersion
  ) "expected=$TargetVersion; actual=$($updatedState.version)"
  Add-Check 'application update preserves install location' (
    $updatedState.installLocation.Equals(
      $initialState.installLocation,
      [StringComparison]::OrdinalIgnoreCase
    )
  ) "expected=$($initialState.installLocation); actual=$($updatedState.installLocation)"
  Add-Check 'updated executable exists' (Test-Path -LiteralPath $updatedState.executable) (
    $updatedState.executable
  )
  Add-Check 'updated executable product version matches' (
    (Get-Item -LiteralPath $updatedState.executable).VersionInfo.ProductVersion.StartsWith($TargetVersion)
  ) (Get-Item -LiteralPath $updatedState.executable).VersionInfo.ProductVersion

  $restartDeadline = (Get-Date).AddSeconds(30)
  do {
    $running = @(
      Get-CimInstance Win32_Process -Filter "Name = '$applicationName'" -ErrorAction SilentlyContinue |
        Where-Object {
          $_.ExecutablePath -and
          [IO.Path]::GetFullPath($_.ExecutablePath).Equals(
            $updatedState.executable,
            [StringComparison]::OrdinalIgnoreCase
          )
        }
    )
    if ($running.Count -gt 0) { break }
    Start-Sleep -Milliseconds 500
  } while ((Get-Date) -lt $restartDeadline)
  Add-Check 'application restarts after update' ($running.Count -gt 0) (
    "processCount=$($running.Count)"
  )

  Compare-Fingerprints $baselineFingerprints (Get-DataFingerprints)
  $contextMenuDeadline = (Get-Date).AddSeconds(20)
  do {
    $contextMenuMode = Get-ContextMenuMode
    if ($contextMenuMode -eq 'legacy') { break }
    Start-Sleep -Milliseconds 250
  } while ((Get-Date) -lt $contextMenuDeadline)
  Add-Check 'updated release registers the legacy context menu' (
    $contextMenuMode -eq 'legacy'
  ) "actual=$contextMenuMode"
  $shellDlls = @(
    Get-ChildItem -LiteralPath $updatedState.installLocation `
      -Filter 'long_compress_shell_extension_*.dll' -File -ErrorAction SilentlyContinue
  )
  Add-Check 'only the target shell extension remains' (
    $shellDlls.Count -eq 1 -and
    $shellDlls[0].Name -eq "long_compress_shell_extension_$($TargetVersion.Replace('.', '_')).dll"
  ) (($shellDlls | ForEach-Object Name) -join ',')
  Add-Check 'unsigned release contains no context-menu identity package' (
    @(Get-ChildItem -LiteralPath $updatedState.installLocation `
      -Filter 'long_compress_context_menu*.msix' -File -ErrorAction SilentlyContinue).Count -eq 0
  ) $updatedState.installLocation

  $evidence.updated = [ordered]@{
    installedState = $updatedState
    dataFingerprints = Get-DataFingerprints
    contextMenuMode = Get-ContextMenuMode
    shellDlls = @($shellDlls | ForEach-Object Name)
  }
  $validationSucceeded = $true
} finally {
  Remove-Item Env:PUBLIC_UPDATER_APP -ErrorAction SilentlyContinue
  Remove-Item Env:PUBLIC_UPDATER_EXPECTED_VERSION -ErrorAction SilentlyContinue
  if (Test-Path -LiteralPath $driverPidPath) {
    $driverPid = [int](Get-Content -Raw -LiteralPath $driverPidPath)
    Stop-Process -Id $driverPid -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
  }
  if ($validationSucceeded -and (Test-Path -LiteralPath $activeWebviewProfilePath)) {
    $webviewProfile = (Get-Content -Raw -LiteralPath $activeWebviewProfilePath).Trim()
    if (
      $webviewProfile -and
      [IO.Path]::GetFullPath($webviewProfile).StartsWith(
        [IO.Path]::GetFullPath([IO.Path]::GetTempPath()),
        [StringComparison]::OrdinalIgnoreCase
      ) -and
      [IO.Path]::GetFileName($webviewProfile).StartsWith(
        'long-decompress-public-updater-webview-',
        [StringComparison]::Ordinal
      )
    ) {
      Remove-Item -LiteralPath $webviewProfile -Recurse -Force -ErrorAction SilentlyContinue
    }
  }
  Save-Evidence
  if ($validationSucceeded) {
    Remove-Item -LiteralPath $backupRoot -Recurse -Force
  } else {
    Write-Warning "User-data backup retained after failure: $backupRoot"
  }
}

Write-Host "Public application update validation passed. Evidence: $evidenceDirectory"
