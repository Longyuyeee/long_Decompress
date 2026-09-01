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
$runId = Get-Date -Format 'yyyyMMdd-HHmmss'
$evidenceDirectory = Join-Path $resultsRoot $runId
$backupRoot = Join-Path ([IO.Path]::GetTempPath()) "long-decompress-public-update-$([guid]::NewGuid().ToString('N'))"
$productName = "Long$([char]0x89E3)$([char]0x538B)"
$applicationName = "$productName.exe"
$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$productName"
$autoStartKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$autoStartValueName = $productName
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

function Get-FileSha256 {
  param([string]$Path)
  $stream = [IO.File]::Open(
    $Path,
    [IO.FileMode]::Open,
    [IO.FileAccess]::Read,
    [IO.FileShare]::ReadWrite
  )
  $hasher = [Security.Cryptography.SHA256]::Create()
  try {
    return ([BitConverter]::ToString($hasher.ComputeHash($stream)) -replace '-', '')
  } finally {
    $hasher.Dispose()
    $stream.Dispose()
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
      "$relative|$($_.Length)|$(Get-FileSha256 $_.FullName)"
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

function Get-PersistedAutoStartPreference {
  $settingsPath = Join-Path $env:APPDATA 'com.longcompress.assistant\app_settings.json'
  if (-not (Test-Path -LiteralPath $settingsPath)) {
    return $false
  }
  $settings = Get-Content -Raw -LiteralPath $settingsPath | ConvertFrom-Json
  return [bool]$settings.autoStart
}

function Get-AutoStartRegistration {
  $properties = Get-ItemProperty -LiteralPath $autoStartKey -ErrorAction SilentlyContinue
  if ($null -eq $properties -or
      -not ($properties.PSObject.Properties.Name -contains $autoStartValueName)) {
    return $null
  }
  return [string]$properties.$autoStartValueName
}

function Stop-InstalledApplication {
  param([string]$Executable)
  $normalizedExecutable = [IO.Path]::GetFullPath($Executable)
  $processes = @(
    Get-CimInstance Win32_Process -Filter "Name = '$applicationName'" -ErrorAction SilentlyContinue |
      Where-Object {
        $_.ExecutablePath -and
        [IO.Path]::GetFullPath($_.ExecutablePath).Equals(
          $normalizedExecutable,
          [StringComparison]::OrdinalIgnoreCase
        )
      }
  )
  foreach ($process in $processes) {
    Stop-Process -Id $process.ProcessId -Force -ErrorAction Stop
  }

  $deadline = (Get-Date).AddSeconds(10)
  do {
    $remaining = @(
      Get-CimInstance Win32_Process -Filter "Name = '$applicationName'" -ErrorAction SilentlyContinue |
        Where-Object {
          $_.ExecutablePath -and
          [IO.Path]::GetFullPath($_.ExecutablePath).Equals(
            $normalizedExecutable,
            [StringComparison]::OrdinalIgnoreCase
          )
        }
    )
    if ($remaining.Count -eq 0) { return $true }
    Start-Sleep -Milliseconds 250
  } while ((Get-Date) -lt $deadline)
  return $false
}

function Get-ContextMenuMode {
  $legacy = Test-Path -LiteralPath 'HKCU:\Software\Classes\*\shell\LongDecompress'
  $native = Test-Path -LiteralPath 'HKCU:\Software\Classes\*\shell\LongDecompressNative'
  if ($legacy -and $native) { return 'mixed' }
  if ($legacy) { return 'legacy' }
  if ($native) { return 'native' }
  return 'none'
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
    if (-not ($rootProperties.PSObject.Properties.Name -contains 'SubCommands') -or
        [string]$rootProperties.SubCommands -ne '') {
      [void]$errors.Add("missing empty SubCommands value: $root")
    }
    if ([string]$rootProperties.Position -ne 'Top') {
      [void]$errors.Add("missing top group position: $root")
    }
    $submenu = Join-Path $root 'shell'
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
  $quickDefinitions = @(
    @{ path = 'HKCU:\Software\Classes\*\shell\LongDecompressQuickPack'; cli = '--quick-pack "%1"' },
    @{ path = 'HKCU:\Software\Classes\directory\shell\LongDecompressQuickPack'; cli = '--quick-pack "%1"' },
    @{ path = 'HKCU:\Software\Classes\directory\Background\shell\LongDecompressQuickPack'; cli = '--quick-pack "%V"' },
    @{ path = 'HKCU:\Software\Classes\SystemFileAssociations\.zip\shell\LongDecompressQuickExtract'; cli = '--quick-extract "%1"' }
  )
  $quickActionCount = 0
  foreach ($definition in $quickDefinitions) {
    $quickProperties = Get-ItemProperty -LiteralPath $definition.path -ErrorAction SilentlyContinue
    if ($null -eq $quickProperties -or [string]$quickProperties.Position -ne 'Top') {
      [void]$errors.Add("missing quick-action top group position: $($definition.path)")
    }
    $commandKey = Join-Path $definition.path 'command'
    if (-not (Test-Path -LiteralPath $commandKey)) {
      [void]$errors.Add("missing quick action: $($definition.path)")
      continue
    }
    $command = [string](Get-Item -LiteralPath $commandKey).GetValue('')
    $expected = "`"$ExpectedExecutable`" $($definition.cli)"
    if (-not $command.Equals($expected, [StringComparison]::OrdinalIgnoreCase)) {
      [void]$errors.Add("wrong quick action target: $command")
      continue
    }
    $quickActionCount += 1
  }
  return [pscustomobject]@{
    valid = $errors.Count -eq 0
    commandCount = $commandCount
    quickActionCount = $quickActionCount
    detail = if ($errors.Count -eq 0) { "roots=4; commands=$commandCount; quickActions=$quickActionCount" } else { $errors -join '; ' }
  }
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
  $autoStartPreference = Get-PersistedAutoStartPreference
  $autoStartRegistration = Get-AutoStartRegistration
  $expectedAutoStartRegistration = "`"$($initialState.executable)`" --autostart"
  Add-Check 'baseline auto-start preference matches Windows registration' (
    ($autoStartPreference -and
      $autoStartRegistration -eq $expectedAutoStartRegistration) -or
    (-not $autoStartPreference -and $null -eq $autoStartRegistration)
  ) (
    "preference=$autoStartPreference; expected=$expectedAutoStartRegistration; actual=$autoStartRegistration"
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

  $env:PUBLIC_UPDATER_APP = $initialState.executable
  $env:PUBLIC_UPDATER_EXPECTED_VERSION = $TargetVersion
  $env:PUBLIC_UPDATER_ARTIFACT_DIR = $uiArtifactDirectory
  $applicationArtifact = Join-Path $uiArtifactDirectory 'independent-application.json'
  Remove-Item -LiteralPath $applicationArtifact -Force -ErrorAction SilentlyContinue
  & node.exe (Join-Path $PSScriptRoot 'run-public-updater-ui.mjs')
  Add-Check 'desktop updater UI completed its hand-off' ($LASTEXITCODE -eq 0) "exitCode=$LASTEXITCODE"

  $launchedApplication = Get-Content -Raw -LiteralPath $applicationArtifact | ConvertFrom-Json
  $originalProcessId = [int]$launchedApplication.processId
  $exitDeadline = (Get-Date).AddSeconds(60)
  do {
    $originalProcess = Get-Process -Id $originalProcessId -ErrorAction SilentlyContinue
    if ($null -eq $originalProcess) { break }
    Start-Sleep -Milliseconds 250
  } while ((Get-Date) -lt $exitDeadline)
  Add-Check 'pre-update application process exits' ($null -eq $originalProcess) (
    "processId=$originalProcessId"
  )

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
          $_.ProcessId -ne $originalProcessId -and
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

  # Startup refreshes existing Explorer integration to the current registry
  # schema. Wait for that setup hook before stopping the restarted process.
  $contextMenuDeadline = (Get-Date).AddSeconds(20)
  do {
    $contextMenuMode = Get-ContextMenuMode
    $contextMenuCascade = Get-ClassicContextMenuCascadeStatus $updatedState.executable
    if ($contextMenuMode -eq 'legacy' -and $contextMenuCascade.valid) { break }
    Start-Sleep -Milliseconds 250
  } while ((Get-Date) -lt $contextMenuDeadline)
  Add-Check 'updated release registers the legacy context menu' (
    $contextMenuMode -eq 'legacy'
  ) "actual=$contextMenuMode"
  Add-Check 'updated release registers complete legacy submenus' (
    $contextMenuCascade.valid
  ) $contextMenuCascade.detail

  # The previous uninstaller may still be finishing after the new process
  # first reports a complete menu. The product performs delayed refreshes;
  # observe beyond both retries before stopping the new process.
  Start-Sleep -Seconds 8
  $stableContextMenuMode = Get-ContextMenuMode
  $stableContextMenuCascade = Get-ClassicContextMenuCascadeStatus $updatedState.executable
  Add-Check 'updated context menu remains complete after updater cleanup' (
    $stableContextMenuMode -eq 'legacy' -and $stableContextMenuCascade.valid
  ) "mode=$stableContextMenuMode; $($stableContextMenuCascade.detail)"

  # The restarted application keeps its SQLite database open. Prove the restart
  # first, then quiesce this exact installation before hashing persistent data.
  Add-Check 'updated application quiesces for data validation' (
    Stop-InstalledApplication $updatedState.executable
  ) $updatedState.executable

  # The updater and old uninstaller can finish cleanup after the restarted app
  # first registers its menu. Require the registration to survive process exit
  # so a transient startup state cannot be reported as a successful migration.
  $finalContextMenuMode = Get-ContextMenuMode
  $finalContextMenuCascade = Get-ClassicContextMenuCascadeStatus $updatedState.executable
  Add-Check 'updated context menu remains registered after application exit' (
    $finalContextMenuMode -eq 'legacy' -and $finalContextMenuCascade.valid
  ) "mode=$finalContextMenuMode; $($finalContextMenuCascade.detail)"
  $updatedAutoStartRegistration = Get-AutoStartRegistration
  $expectedUpdatedAutoStartRegistration = "`"$($updatedState.executable)`" --autostart"
  Add-Check 'application update preserves coherent auto-start state' (
    ($autoStartPreference -and
      $updatedAutoStartRegistration -eq $expectedUpdatedAutoStartRegistration) -or
    (-not $autoStartPreference -and $null -eq $updatedAutoStartRegistration)
  ) (
    "preference=$autoStartPreference; expected=$expectedUpdatedAutoStartRegistration; actual=$updatedAutoStartRegistration"
  )

  Compare-Fingerprints $baselineFingerprints (Get-DataFingerprints)
  $resourceDirectory = Join-Path $updatedState.installLocation 'resources'
  $shellDlls = @(
    Get-ChildItem -LiteralPath $resourceDirectory `
      -Filter 'long_compress_shell_extension_*.dll' -File -ErrorAction SilentlyContinue
  )
  Add-Check 'only the target shell extension remains' (
    $shellDlls.Count -eq 1 -and
    $shellDlls[0].Name -eq "long_compress_shell_extension_$($TargetVersion.Replace('.', '_')).dll"
  ) "resourceDirectory=$resourceDirectory; found=$(($shellDlls | ForEach-Object Name) -join ',')"
  Add-Check 'unsigned release contains no context-menu identity package' (
    @(Get-ChildItem -LiteralPath $resourceDirectory `
      -Filter 'long_compress_context_menu*.msix' -File -ErrorAction SilentlyContinue).Count -eq 0
  ) $resourceDirectory

  $evidence.updated = [ordered]@{
    installedState = $updatedState
    dataFingerprints = Get-DataFingerprints
    contextMenuMode = $finalContextMenuMode
    contextMenuCascade = $finalContextMenuCascade
    shellDlls = @($shellDlls | ForEach-Object Name)
  }
  $validationSucceeded = $true
} finally {
  Remove-Item Env:PUBLIC_UPDATER_APP -ErrorAction SilentlyContinue
  Remove-Item Env:PUBLIC_UPDATER_EXPECTED_VERSION -ErrorAction SilentlyContinue
  Remove-Item Env:PUBLIC_UPDATER_ARTIFACT_DIR -ErrorAction SilentlyContinue
  Save-Evidence
  if ($validationSucceeded) {
    Remove-Item -LiteralPath $backupRoot -Recurse -Force
  } else {
    Write-Warning "User-data backup retained after failure: $backupRoot"
  }
}

Write-Host "Public application update validation passed. Evidence: $evidenceDirectory"
