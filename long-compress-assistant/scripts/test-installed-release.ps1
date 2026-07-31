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
$contextMenuRegistryBackups = @()
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

function Get-ClassicContextMenuCommand {
  $commandKeys = @(
    'HKCU:\Software\Classes\SystemFileAssociations\.zip\shell\LongDecompress\ExtendedSubCommandsKey\shell\01.LongDecompress.open\command',
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.open\command'
  )
  foreach ($commandKey in $commandKeys) {
    if (Test-Path -LiteralPath $commandKey) {
      return [string](Get-Item -LiteralPath $commandKey).GetValue('')
    }
  }
  return ''
}

function Test-ClassicContextMenuTarget {
  param([string]$ExpectedExecutable)
  $command = Get-ClassicContextMenuCommand
  return $command -like "`"$ExpectedExecutable`"*"
}

function Get-ClassicContextMenuCascadeStatus {
  param([string]$ExpectedExecutable)
  $definitions = @(
    @{ path = 'HKCU:\Software\Classes\*\shell\LongDecompress'; count = 3 },
    @{ path = 'HKCU:\Software\Classes\SystemFileAssociations\.zip\shell\LongDecompress'; count = 8 },
    @{ path = 'HKCU:\Software\Classes\directory\shell\LongDecompress'; count = 3 },
    @{ path = 'HKCU:\Software\Classes\directory\Background\shell\LongDecompress'; count = 3 }
  )
  $errors = [Collections.Generic.List[string]]::new()
  $commandCount = 0
  foreach ($definition in $definitions) {
    $root = $definition.path
    if (-not (Test-Path -LiteralPath $root)) {
      [void]$errors.Add("missing root: $root")
      continue
    }
    $rootProperties = Get-ItemProperty -LiteralPath $root
    if ($rootProperties.PSObject.Properties.Name -contains 'SubCommands') {
      [void]$errors.Add("obsolete SubCommands value: $root")
    }
    $submenu = Join-Path $root 'ExtendedSubCommandsKey\shell'
    $children = if (Test-Path -LiteralPath $submenu) {
      @(Get-ChildItem -LiteralPath $submenu | Sort-Object PSChildName)
    } else {
      @()
    }
    if ($children.Count -ne $definition.count) {
      [void]$errors.Add("submenu count $($children.Count)/$($definition.count): $root")
    }
    foreach ($child in $children) {
      $commandKey = Join-Path $child.PSPath 'command'
      if (-not (Test-Path -LiteralPath $commandKey)) {
        [void]$errors.Add("missing command: $($child.PSChildName)")
        continue
      }
      $command = [string](Get-Item -LiteralPath $commandKey).GetValue('')
      $commandCount += 1
      if (-not $command.StartsWith("`"$ExpectedExecutable`" ", [StringComparison]::OrdinalIgnoreCase)) {
        [void]$errors.Add("wrong target: $command")
      }
    }
  }
  return [pscustomobject]@{
    valid = $errors.Count -eq 0
    commandCount = $commandCount
    detail = if ($errors.Count -eq 0) { "roots=4; commands=$commandCount" } else { $errors -join '; ' }
  }
}

function Get-OwnedContextMenuRegistryPaths {
  $template = Get-Content -Raw -Encoding utf8 -LiteralPath $installerTemplate
  $paths = [regex]::Matches(
    $template,
    'DeleteRegKey HKCU "([^"]*LongDecompress[^"]*)"'
  ) | ForEach-Object { $_.Groups[1].Value }
  $paths += @(
    'Software\Classes\CLSID\{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4A}',
    'Software\Classes\CLSID\{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4B}',
    'Software\Classes\CLSID\{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4C}'
  )
  return @($paths | Sort-Object -Unique)
}

function Backup-ContextMenuRegistry {
  $registryBackupDirectory = Join-Path $backupRoot 'context-menu-registry'
  New-Item -ItemType Directory -Path $registryBackupDirectory -Force | Out-Null
  $index = 0
  foreach ($path in Get-OwnedContextMenuRegistryPaths) {
    $providerPath = "HKCU:\$path"
    if (-not (Test-Path -LiteralPath $providerPath)) {
      continue
    }
    $backup = Join-Path $registryBackupDirectory "$index.reg"
    & reg.exe export "HKCU\$path" $backup /y | Out-Null
    if ($LASTEXITCODE -ne 0) {
      throw "Unable to back up context-menu registry path: HKCU\$path"
    }
    $script:contextMenuRegistryBackups += $backup
    $index += 1
  }
}

function Restore-ContextMenuRegistry {
  foreach ($path in Get-OwnedContextMenuRegistryPaths) {
    $providerPath = "HKCU:\$path"
    if (Test-Path -LiteralPath $providerPath) {
      Remove-Item -LiteralPath $providerPath -Recurse -Force
    }
  }
  foreach ($backup in $contextMenuRegistryBackups) {
    & reg.exe import $backup | Out-Null
    if ($LASTEXITCODE -ne 0) {
      throw "Unable to restore context-menu registry backup: $backup"
    }
  }
}

function Restore-ContextMenuMode {
  param($InstalledState, [string]$ExpectedMode)
  Restore-ContextMenuRegistry
  if ($ExpectedMode -eq 'native') {
    $process = Start-Process -FilePath $InstalledState.executable -PassThru -WindowStyle Hidden
    try {
      $deadline = (Get-Date).AddSeconds(20)
      do {
        if ((Get-ContextMenuMode) -eq 'native') { break }
        Start-Sleep -Milliseconds 250
      } while ((Get-Date) -lt $deadline)
    } finally {
      Stop-InstalledApplication $InstalledState.installLocation
      if (-not $process.HasExited) {
        $process.WaitForExit(5000) | Out-Null
      }
    }
  }
  Add-Check 'baseline context-menu mode is restored' (
    (Get-ContextMenuMode) -eq $ExpectedMode
  ) "expected=$ExpectedMode; actual=$(Get-ContextMenuMode)"
  if ($ExpectedMode -eq 'legacy') {
    Add-Check 'baseline context-menu target is restored' (
      Test-ClassicContextMenuTarget $InstalledState.executable
    ) "expected=$($InstalledState.executable); actual=$(Get-ClassicContextMenuCommand)"
  }
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
  $expectedVersionSuffix = $ExpectedVersion -replace '[^0-9A-Za-z]', '_'
  $expectedDll = "long_compress_shell_extension_$expectedVersionSuffix.dll"
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
  $nativeRoot = 'HKCU:\Software\Classes\*\shell\LongDecompressNative'
  Add-Check 'classic context-menu root is registered' (Test-Path -LiteralPath $legacyRoot) $legacyRoot
  Add-Check 'unsigned native context-menu root is absent' (-not (Test-Path -LiteralPath $nativeRoot)) $nativeRoot
  $cascade = Get-ClassicContextMenuCascadeStatus $ExpectedExecutable
  Add-Check 'classic context-menu submenus are complete and target current executable' (
    $cascade.valid
  ) $cascade.detail
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
  Backup-ContextMenuRegistry
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
      if (
        (Test-Path -LiteralPath 'HKCU:\Software\Classes\*\shell\LongDecompress') -and
        (Test-ClassicContextMenuTarget $candidateState.executable)
      ) {
        break
      }
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
      Write-Warning $evidence.recovery
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
