param(
  [Parameter(Mandatory = $true)]
  [ValidateSet('MissingMediaFeaturePack', 'MediaFeaturePackInstalled')]
  [string]$Phase,
  [Parameter(Mandatory = $true)]
  [string]$InstalledExecutable,
  [string]$EvidenceDirectory
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$manifest = Get-Content -Raw -Encoding utf8 -LiteralPath (
  Join-Path $projectRoot 'config\media-dependencies.json'
) | ConvertFrom-Json
$videoEvidence = $manifest.candidateBaselines.video
$expectedVersion = [string]$videoEvidence.installedLifecycle.candidateVersion
$expectedExecutableBytes = [long]$videoEvidence.installedExecutableBytes
$expectedExecutableHash = ([string]$videoEvidence.installedExecutableSha256).ToLowerInvariant()
$productName = "Long$([char]0x89E3)$([char]0x538B)"
$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$productName"
$applicationName = "$productName.exe"
$evidenceRoot = if ([string]::IsNullOrWhiteSpace($EvidenceDirectory)) {
  Join-Path $projectRoot 'test-results\windows-n-video-runtime'
} else {
  [IO.Path]::GetFullPath($EvidenceDirectory)
}
$phaseFileName = if ($Phase -eq 'MissingMediaFeaturePack') {
  'before-media-feature-pack.json'
} else {
  'after-media-feature-pack.json'
}
$phaseReportPath = Join-Path $evidenceRoot $phaseFileName
$preflightPath = Join-Path $evidenceRoot "$Phase-preflight.json"
$beforeReportPath = Join-Path $evidenceRoot 'before-media-feature-pack.json'
$checks = [Collections.Generic.List[object]]::new()
$failure = $null

function Get-FileSha256 {
  param([string]$Path)
  $stream = [IO.File]::OpenRead($Path)
  $hasher = [Security.Cryptography.SHA256]::Create()
  try {
    return ([BitConverter]::ToString($hasher.ComputeHash($stream)) -replace '-', '').ToLowerInvariant()
  } finally {
    $hasher.Dispose()
    $stream.Dispose()
  }
}

function Get-StringSha256 {
  param([string]$Value)
  $hasher = [Security.Cryptography.SHA256]::Create()
  try {
    $bytes = [Text.Encoding]::UTF8.GetBytes("long-decompress-windows-n-evidence-v1|$Value")
    return ([BitConverter]::ToString($hasher.ComputeHash($bytes)) -replace '-', '').ToLowerInvariant()
  } finally {
    $hasher.Dispose()
  }
}

function Add-Check {
  param([string]$Name, [bool]$Passed, [string]$Detail)
  $checks.Add([ordered]@{ name = $Name; passed = $Passed; detail = $Detail })
  if (-not $Passed) {
    throw "$Name failed: $Detail"
  }
}

function Get-MachineFacts {
  $os = Get-CimInstance Win32_OperatingSystem
  $currentVersion = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
  $machineGuid = [string](Get-ItemPropertyValue -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Cryptography' -Name MachineGuid)
  $editionId = [string]$currentVersion.EditionID
  return [ordered]@{
    identitySha256 = Get-StringSha256 $machineGuid
    caption = [string]$os.Caption
    editionId = $editionId
    isWindowsNEdition = $editionId -match '(?i)N$'
    version = [string]$os.Version
    buildNumber = [string]$os.BuildNumber
    displayVersion = [string]$currentVersion.DisplayVersion
    architecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
  }
}

function Get-MediaFoundationModules {
  $system32 = [Environment]::SystemDirectory
  $modules = foreach ($name in @('mfplat.dll', 'mf.dll', 'mfreadwrite.dll')) {
    $path = Join-Path $system32 $name
    if (Test-Path -LiteralPath $path -PathType Leaf) {
      $item = Get-Item -LiteralPath $path
      [ordered]@{
        name = $name
        present = $true
        bytes = $item.Length
        sha256 = Get-FileSha256 $path
        fileVersion = $item.VersionInfo.FileVersion
      }
    } else {
      [ordered]@{ name = $name; present = $false }
    }
  }
  return @($modules)
}

function Write-PhaseReport {
  param($Machine, $ExecutableIdentity, $Preflight, $MediaFoundationModules, [bool]$Passed)
  $report = [ordered]@{
    schemaVersion = 2
    measuredAt = (Get-Date).ToUniversalTime().ToString('o')
    phase = $Phase
    producerScriptSha256 = Get-FileSha256 $PSCommandPath
    machine = $Machine
    expected = [ordered]@{
      windowsNEdition = $true
      installedVersion = $expectedVersion
      executableBytes = $expectedExecutableBytes
      executableSha256 = $expectedExecutableHash
      missingPhaseClassification = 'VIDEO_ENGINE_MEDIA_FOUNDATION_UNAVAILABLE'
      installedPhaseProductionPreflight = $true
      installedPhaseRealSoftwareTranscode = $true
    }
    actual = [ordered]@{
      executable = $ExecutableIdentity
      mediaFoundationModules = $MediaFoundationModules
      productionPreflight = $Preflight
      beforeReportSha256 = $beforeReportSha256
    }
    checks = $checks
    failure = $failure
    passed = $Passed
  }
  New-Item -ItemType Directory -Path $evidenceRoot -Force | Out-Null
  $report | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $phaseReportPath -Encoding utf8
}

$machine = $null
$executableIdentity = $null
$preflight = $null
$mediaFoundationModules = @()
$beforeReportSha256 = $null

try {
  New-Item -ItemType Directory -Path $evidenceRoot -Force | Out-Null
  $machine = Get-MachineFacts
  if (-not $machine.isWindowsNEdition) {
    throw "WINDOWS_N_MACHINE_REQUIRED: editionId=$($machine.editionId); caption=$($machine.caption)"
  }
  Add-Check 'host is a real Windows N edition' $machine.isWindowsNEdition (
    "editionId=$($machine.editionId); caption=$($machine.caption)"
  )

  Add-Check 'formal uninstall registry key exists' (Test-Path -LiteralPath $uninstallKey) $uninstallKey
  $installedState = Get-ItemProperty -LiteralPath $uninstallKey
  $installLocation = ([string]$installedState.InstallLocation).Trim('"')
  $expectedExecutable = [IO.Path]::GetFullPath((Join-Path $installLocation $applicationName))
  $executablePath = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $InstalledExecutable).Path)
  Add-Check 'installed version matches candidate' ([string]$installedState.DisplayVersion -eq $expectedVersion) (
    "expected=$expectedVersion; actual=$($installedState.DisplayVersion)"
  )
  Add-Check 'executable is the formally installed application' (
    $executablePath.Equals($expectedExecutable, [StringComparison]::OrdinalIgnoreCase)
  ) "expected=$expectedExecutable; actual=$executablePath"
  $executableIdentity = [ordered]@{
    path = $executablePath
    bytes = (Get-Item -LiteralPath $executablePath).Length
    sha256 = Get-FileSha256 $executablePath
  }
  Add-Check 'installed candidate executable identity matches' (
    $executableIdentity.bytes -eq $expectedExecutableBytes -and
    $executableIdentity.sha256 -eq $expectedExecutableHash
  ) "expectedBytes=$expectedExecutableBytes; actualBytes=$($executableIdentity.bytes); expectedSha256=$expectedExecutableHash; actualSha256=$($executableIdentity.sha256)"

  $mediaFoundationModules = Get-MediaFoundationModules
  Remove-Item -LiteralPath $preflightPath -Force -ErrorAction SilentlyContinue
  & $executablePath '--internal-video-engine-preflight-report' $preflightPath
  $preflightExitCode = $LASTEXITCODE
  Add-Check 'production preflight writes its machine report' (
    Test-Path -LiteralPath $preflightPath -PathType Leaf
  ) $preflightPath
  $preflight = Get-Content -Raw -Encoding utf8 -LiteralPath $preflightPath | ConvertFrom-Json

  if ($Phase -eq 'MissingMediaFeaturePack') {
    Add-Check 'production preflight refuses missing Media Foundation' ($preflightExitCode -eq 2) (
      "expectedExitCode=2; actual=$preflightExitCode"
    )
    Add-Check 'missing Media Foundation classification is stable' (
      $preflight.passed -eq $false -and
      [string]$preflight.error -match '^VIDEO_ENGINE_MEDIA_FOUNDATION_UNAVAILABLE: (mfplat|mf|mfreadwrite)\.dll: win32=\d+$'
    ) "error=$($preflight.error)"
  } else {
    Add-Check 'pre-feature-pack report exists' (
      Test-Path -LiteralPath $beforeReportPath -PathType Leaf
    ) $beforeReportPath
    $beforeReportSha256 = Get-FileSha256 $beforeReportPath
    $before = Get-Content -Raw -Encoding utf8 -LiteralPath $beforeReportPath | ConvertFrom-Json
    Add-Check 'pre-feature-pack report schema matches' ($before.schemaVersion -eq 2) (
      "expected=2; actual=$($before.schemaVersion)"
    )
    Add-Check 'pre-feature-pack report phase matches' (
      [string]$before.phase -eq 'MissingMediaFeaturePack'
    ) "phase=$($before.phase)"
    Add-Check 'pre-feature-pack phase passed' ($before.passed -eq $true) "passed=$($before.passed)"
    Add-Check 'both phases use the same evidence producer' (
      [string]$before.producerScriptSha256 -eq (Get-FileSha256 $PSCommandPath)
    ) "before=$($before.producerScriptSha256); after=$(Get-FileSha256 $PSCommandPath)"
    Add-Check 'pre-feature-pack report is a Windows N result' (
      $before.machine.isWindowsNEdition -eq $true -and
      [string]$before.machine.editionId -match '(?i)N$'
    ) "editionId=$($before.machine.editionId)"
    Add-Check 'both phases use the locked candidate identity' (
      [long]$before.actual.executable.bytes -eq $expectedExecutableBytes -and
      [string]$before.actual.executable.sha256 -eq $expectedExecutableHash
    ) "beforeBytes=$($before.actual.executable.bytes); beforeSha256=$($before.actual.executable.sha256)"
    Add-Check 'both phases ran on the same machine' (
      [string]$before.machine.identitySha256 -eq [string]$machine.identitySha256
    ) "before=$($before.machine.identitySha256); after=$($machine.identitySha256)"
    Add-Check 'production preflight passes after Media Feature Pack installation' (
      $preflightExitCode -eq 0 -and $preflight.passed -eq $true -and
      $preflight.status.mediaFoundationAvailable -eq $true
    ) "exitCode=$preflightExitCode; passed=$($preflight.passed)"

    $previousAppBinary = $env:TAURI_APP_BINARY
    $previousEvidenceDirectory = $env:VIDEO_RUNTIME_EVIDENCE_DIRECTORY
    try {
      $env:TAURI_APP_BINARY = $executablePath
      $env:VIDEO_RUNTIME_EVIDENCE_DIRECTORY = Join-Path $evidenceRoot 'after-feature-pack-runtime'
      & node (Join-Path $projectRoot 'scripts\test-installed-video-runtime.mjs')
      Add-Check 'real installed software transcode passes after Media Feature Pack installation' (
        $LASTEXITCODE -eq 0
      ) "exitCode=$LASTEXITCODE"
    } finally {
      if ($null -eq $previousAppBinary) {
        Remove-Item Env:TAURI_APP_BINARY -ErrorAction SilentlyContinue
      } else {
        $env:TAURI_APP_BINARY = $previousAppBinary
      }
      if ($null -eq $previousEvidenceDirectory) {
        Remove-Item Env:VIDEO_RUNTIME_EVIDENCE_DIRECTORY -ErrorAction SilentlyContinue
      } else {
        $env:VIDEO_RUNTIME_EVIDENCE_DIRECTORY = $previousEvidenceDirectory
      }
    }
  }
} catch {
  $failure = $_.Exception.Message
} finally {
  Write-PhaseReport $machine $executableIdentity $preflight $mediaFoundationModules ($null -eq $failure)
}

if ($null -ne $failure) {
  [Console]::Error.WriteLine("$failure Evidence: $phaseReportPath")
  exit 1
}

Write-Output "Windows N video runtime phase passed: $Phase. Evidence: $phaseReportPath"
