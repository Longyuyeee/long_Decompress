param(
  [Parameter(Mandatory = $true)]
  [string]$PreviousInstaller,
  [Parameter(Mandatory = $true)]
  [string]$CandidateInstaller,
  [Parameter(Mandatory = $true)]
  [string]$PreviousVersion,
  [Parameter(Mandatory = $true)]
  [string]$CandidateVersion,
  [switch]$AllowExistingInstall
)

$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
$testResultsRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'test-results'))
$runId = Get-Date -Format 'yyyyMMdd-HHmmss'
$evidenceDirectory = [IO.Path]::GetFullPath(
  (Join-Path $testResultsRoot "installed-release-validation\$runId")
)
$backupRoot = [IO.Path]::GetFullPath(
  (Join-Path ([IO.Path]::GetTempPath()) "long-decompress-release-backup-$([guid]::NewGuid().ToString('N'))")
)
$productName = "Long$([char]0x89E3)$([char]0x538B)"
$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$productName"
$appProductKey = "HKCU:\Software\Longyuyeee\$productName"
$appDataPaths = @(
  [IO.Path]::GetFullPath((Join-Path $env:APPDATA 'LongDecompress')),
  [IO.Path]::GetFullPath((Join-Path $env:APPDATA 'com.longcompress.assistant'))
)
$installerTemplate = Join-Path $projectRoot 'src-tauri\installer.nsi'
$applicationName = "$productName.exe"
$restoreRequired = $false
$validationSucceeded = $false
$baselineContextMenuMode = 'none'
$evidence = [ordered]@{
  schemaVersion = 1
  startedAt = (Get-Date).ToUniversalTime().ToString('o')
  machine = [ordered]@{
    windows = [Environment]::OSVersion.VersionString
    architecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
  }
  previousVersion = $PreviousVersion
  candidateVersion = $CandidateVersion
  checks = @()
}

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

function Resolve-Installer {
  param([string]$Path, [string]$Label)
  $resolved = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $Path).Path)
  if ([IO.Path]::GetExtension($resolved) -ne '.exe') {
    throw "$Label must be an executable installer: $resolved"
  }
  return $resolved
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

function Compare-Fingerprints {
  param($Expected, $Actual, [string]$Stage)
  foreach ($path in $appDataPaths) {
    Add-Check "$Stage preserves $path" ($Expected[$path] -eq $Actual[$path]) (
      "expected=$($Expected[$path]); actual=$($Actual[$path])"
    )
  }
}

function Backup-UserData {
  New-Item -ItemType Directory -Path $backupRoot -Force | Out-Null
  for ($index = 0; $index -lt $appDataPaths.Count; $index += 1) {
    $path = $appDataPaths[$index]
    if (Test-Path -LiteralPath $path) {
      Copy-Item -LiteralPath $path -Destination (Join-Path $backupRoot "data-$index") -Recurse
    }
  }
}

function Restore-UserData {
  for ($index = 0; $index -lt $appDataPaths.Count; $index += 1) {
    $path = $appDataPaths[$index]
    $backup = Join-Path $backupRoot "data-$index"
    if (Test-Path -LiteralPath $backup) {
      if (Test-Path -LiteralPath $path) {
        Remove-Item -LiteralPath $path -Recurse -Force
      }
      $parent = Split-Path -Parent $path
      New-Item -ItemType Directory -Path $parent -Force | Out-Null
      Copy-Item -LiteralPath $backup -Destination $path -Recurse
    } elseif (Test-Path -LiteralPath $path) {
      Remove-Item -LiteralPath $path -Recurse -Force
    }
  }
}

function Stop-InstalledApplication {
  param([string]$InstallLocation)
  $expectedExecutable = [IO.Path]::GetFullPath((Join-Path $InstallLocation $applicationName))
  Get-CimInstance Win32_Process -Filter "Name = '$applicationName'" -ErrorAction SilentlyContinue |
    Where-Object {
      $_.ExecutablePath -and
      [IO.Path]::GetFullPath($_.ExecutablePath).Equals(
        $expectedExecutable,
        [StringComparison]::OrdinalIgnoreCase
      )
    } |
    ForEach-Object {
      Stop-Process -Id $_.ProcessId -Force -ErrorAction Stop
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

function Restore-ContextMenuMode {
  param($InstalledState, [string]$ExpectedMode)
  if ($ExpectedMode -eq 'none') {
    Add-Check 'baseline context-menu absence is restored' (
      (Get-ContextMenuMode) -eq 'none'
    ) "actual=$(Get-ContextMenuMode)"
    return
  }

  $process = Start-Process -FilePath $InstalledState.executable -PassThru -WindowStyle Hidden
  try {
    $deadline = (Get-Date).AddSeconds(20)
    do {
      $actualMode = Get-ContextMenuMode
      if ($actualMode -eq $ExpectedMode) { break }
      Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
  } finally {
    Stop-InstalledApplication $InstalledState.installLocation
    if (-not $process.HasExited) {
      $process.WaitForExit(5000) | Out-Null
    }
  }
  Add-Check 'baseline context-menu mode is restored' (
    (Get-ContextMenuMode) -eq $ExpectedMode
  ) "expected=$ExpectedMode; actual=$(Get-ContextMenuMode)"
}

function Invoke-Installer {
  param([string]$Path)
  $process = Start-Process -FilePath $Path -ArgumentList @('/P', '/NS', '/NR') -PassThru -WindowStyle Hidden
  if (-not $process.WaitForExit(90000)) {
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    throw "Installer timed out after 90 seconds: $Path"
  }
  Add-Check "installer exits successfully: $([IO.Path]::GetFileName($Path))" ($process.ExitCode -eq 0) (
    "exitCode=$($process.ExitCode)"
  )
}

function Get-InstalledState {
  if (-not (Test-Path -LiteralPath $uninstallKey)) {
    return $null
  }
  $properties = Get-ItemProperty -LiteralPath $uninstallKey
  $installLocation = [string]$properties.InstallLocation
  $installLocation = [IO.Path]::GetFullPath($installLocation.Trim('"'))
  return [ordered]@{
    version = [string]$properties.DisplayVersion
    installLocation = $installLocation
    executable = Join-Path $installLocation $applicationName
    uninstaller = Join-Path $installLocation 'uninstall.exe'
  }
}

function Assert-Installed {
  param([string]$ExpectedVersion, [string]$ExpectedLocation)
  $state = Get-InstalledState
  Add-Check 'uninstall registry key exists' ($null -ne $state) $uninstallKey
  Add-Check 'installed version matches' ($state.version -eq $ExpectedVersion) (
    "expected=$ExpectedVersion; actual=$($state.version)"
  )
  Add-Check 'install location is preserved' (
    $state.installLocation.Equals($ExpectedLocation, [StringComparison]::OrdinalIgnoreCase)
  ) "expected=$ExpectedLocation; actual=$($state.installLocation)"
  Add-Check 'installed executable exists' (Test-Path -LiteralPath $state.executable) $state.executable
  Add-Check 'installed uninstaller exists' (Test-Path -LiteralPath $state.uninstaller) $state.uninstaller

  $productVersion = (Get-Item -LiteralPath $state.executable).VersionInfo.ProductVersion
  Add-Check 'executable product version matches' ($productVersion.StartsWith($ExpectedVersion)) (
    "expected=$ExpectedVersion; actual=$productVersion"
  )
  $expectedDll = "long_compress_shell_extension_$($ExpectedVersion.Replace('.', '_')).dll"
  $shellDlls = @(
    Get-ChildItem -LiteralPath (Join-Path $state.installLocation 'resources') `
      -Filter 'long_compress_shell_extension_*.dll' -File -ErrorAction SilentlyContinue
  )
  Add-Check 'one shell extension is installed' ($shellDlls.Count -eq 1) (
    "found=$($shellDlls.Name -join ',')"
  )
  Add-Check 'shell extension version matches' ($shellDlls[0].Name -eq $expectedDll) (
    "expected=$expectedDll; actual=$($shellDlls[0].Name)"
  )
  return $state
}

function Assert-ClassicContextMenu {
  param([string]$ExpectedExecutable)
  $legacyRoot = 'HKCU:\Software\Classes\*\shell\LongDecompress'
  $legacyCommand = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.open\command'
  $nativeRoot = 'HKCU:\Software\Classes\*\shell\LongDecompressNative'
  Add-Check 'classic context-menu root is registered' (Test-Path -LiteralPath $legacyRoot) $legacyRoot
  Add-Check 'classic context-menu command is registered' (Test-Path -LiteralPath $legacyCommand) $legacyCommand
  Add-Check 'unsigned native context-menu root is absent' (-not (Test-Path -LiteralPath $nativeRoot)) $nativeRoot
  $command = (Get-ItemProperty -LiteralPath $legacyCommand).'(default)'
  if (-not $command) {
    $command = (Get-Item -LiteralPath $legacyCommand).GetValue('')
  }
  Add-Check 'classic context-menu targets current executable' (
    [string]$command -like "`"$ExpectedExecutable`"*"
  ) "command=$command"
}

function Assert-Uninstalled {
  param([string]$InstallLocation)
  Add-Check 'uninstall registry key is removed' (-not (Test-Path -LiteralPath $uninstallKey)) $uninstallKey
  Add-Check 'product registry key is removed or empty' (
    -not (Test-Path -LiteralPath $appProductKey) -or
    @((Get-Item -LiteralPath $appProductKey).GetValueNames()).Count -eq 0
  ) $appProductKey
  Add-Check 'installed executable is removed' (
    -not (Test-Path -LiteralPath (Join-Path $InstallLocation $applicationName))
  ) $InstallLocation
  Add-Check 'installed uninstaller is removed' (
    -not (Test-Path -LiteralPath (Join-Path $InstallLocation 'uninstall.exe'))
  ) $InstallLocation

  $template = Get-Content -Raw -Encoding utf8 -LiteralPath $installerTemplate
  $cleanupKeys = [regex]::Matches($template, 'DeleteRegKey HKCU "([^"]+)"') |
    ForEach-Object { "HKCU:\$($_.Groups[1].Value)" } |
    Sort-Object -Unique
  $remainingKeys = @($cleanupKeys | Where-Object { Test-Path -LiteralPath $_ })
  Add-Check 'all installer-owned context-menu keys are removed' ($remainingKeys.Count -eq 0) (
    "remaining=$($remainingKeys -join ',')"
  )
}

$previousInstallerPath = Resolve-Installer $PreviousInstaller 'Previous installer'
$candidateInstallerPath = Resolve-Installer $CandidateInstaller 'Candidate installer'

try {
  if (-not $AllowExistingInstall) {
    throw 'This test changes the current-user installation. Re-run with -AllowExistingInstall after reviewing the backup and restore behavior.'
  }
  $initialState = Get-InstalledState
  Add-Check 'previous release is installed' ($null -ne $initialState) $uninstallKey
  Add-Check 'installed baseline version matches' ($initialState.version -eq $PreviousVersion) (
    "expected=$PreviousVersion; actual=$($initialState.version)"
  )
  Add-Check 'no installed application process is running' (
    @(Get-CimInstance Win32_Process -Filter "Name = '$applicationName'" -ErrorAction SilentlyContinue).Count -eq 0
  ) "Close $productName before running installed-release validation."
  $baselineContextMenuMode = Get-ContextMenuMode
  Add-Check 'baseline context-menu mode is valid' (
    $baselineContextMenuMode -in @('none', 'legacy', 'native')
  ) "actual=$baselineContextMenuMode"
  $evidence.baselineContextMenuMode = $baselineContextMenuMode

  New-Item -ItemType Directory -Path $evidenceDirectory -Force | Out-Null
  Backup-UserData
  $baselineFingerprints = Get-DataFingerprints
  $evidence.baselineDataFingerprints = $baselineFingerprints
  $restoreRequired = $true

  Invoke-Installer $candidateInstallerPath
  $candidateState = Assert-Installed $CandidateVersion $initialState.installLocation
  Compare-Fingerprints $baselineFingerprints (Get-DataFingerprints) 'overlay install'

  $candidateProcess = Start-Process -FilePath $candidateState.executable -PassThru -WindowStyle Hidden
  try {
    $deadline = (Get-Date).AddSeconds(20)
    do {
      if (Test-Path -LiteralPath 'HKCU:\Software\Classes\*\shell\LongDecompress') { break }
      Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
    Assert-ClassicContextMenu $candidateState.executable
  } finally {
    Stop-InstalledApplication $candidateState.installLocation
    if (-not $candidateProcess.HasExited) {
      $candidateProcess.WaitForExit(5000) | Out-Null
    }
  }

  $candidateUninstaller = $candidateState.uninstaller
  # Use NSIS silent uninstall here. `/P` is the passive installer switch; when
  # invoked directly on uninstall.exe it may leave the confirmation UI waiting
  # without a visible window.
  $uninstallProcess = Start-Process -FilePath $candidateUninstaller `
    -ArgumentList @('/S') `
    -PassThru -WindowStyle Hidden
  if (-not $uninstallProcess.WaitForExit(90000)) {
    Stop-Process -Id $uninstallProcess.Id -Force -ErrorAction SilentlyContinue
    throw "Candidate uninstaller timed out after 90 seconds: $candidateUninstaller"
  }
  Add-Check 'candidate uninstaller exits successfully' ($uninstallProcess.ExitCode -eq 0) (
    "exitCode=$($uninstallProcess.ExitCode)"
  )
  $uninstallDeadline = (Get-Date).AddSeconds(30)
  while (
    ((Test-Path -LiteralPath $uninstallKey) -or
      (Test-Path -LiteralPath (Join-Path $candidateState.installLocation $applicationName))) -and
    (Get-Date) -lt $uninstallDeadline
  ) {
    Start-Sleep -Milliseconds 250
  }
  Assert-Uninstalled $candidateState.installLocation
  Compare-Fingerprints $baselineFingerprints (Get-DataFingerprints) 'uninstall'

  Invoke-Installer $previousInstallerPath
  $restoredState = Assert-Installed $PreviousVersion $initialState.installLocation
  Compare-Fingerprints $baselineFingerprints (Get-DataFingerprints) 'baseline restore'
  Restore-ContextMenuMode $restoredState $baselineContextMenuMode
  Compare-Fingerprints $baselineFingerprints (Get-DataFingerprints) 'baseline menu restore'
  $restoreRequired = $false
  $validationSucceeded = $true
  $evidence.restoredInstallLocation = $restoredState.installLocation
} finally {
  if ($restoreRequired) {
    try {
      $currentState = Get-InstalledState
      if ($currentState) {
        Stop-InstalledApplication $currentState.installLocation
      }
      Invoke-Installer $previousInstallerPath
      Restore-UserData
      $recoveredState = Get-InstalledState
      if ($recoveredState) {
        Restore-ContextMenuMode $recoveredState $baselineContextMenuMode
      }
      $evidence.recovery = 'Previous installer and user-data backup restored after failure.'
    } catch {
      $evidence.recovery = "Automatic recovery failed; backup retained at $backupRoot. Error: $_"
      Write-Error $evidence.recovery
    }
  }

  $evidence.completedAt = (Get-Date).ToUniversalTime().ToString('o')
  $evidence.succeeded = $validationSucceeded
  if (Test-Path -LiteralPath $evidenceDirectory) {
    $evidencePath = Join-Path $evidenceDirectory 'result.json'
    [IO.File]::WriteAllText(
      $evidencePath,
      ($evidence | ConvertTo-Json -Depth 8),
      [Text.UTF8Encoding]::new($false)
    )
    Write-Output "Installed-release validation evidence: $evidencePath"
  }
  if ($validationSucceeded -and (Test-Path -LiteralPath $backupRoot)) {
    Remove-Item -LiteralPath $backupRoot -Recurse -Force
  } elseif (Test-Path -LiteralPath $backupRoot) {
    Write-Warning "Recovery backup retained at $backupRoot"
  }
}

if (-not $validationSucceeded) {
  throw 'Installed-release validation did not complete successfully.'
}
