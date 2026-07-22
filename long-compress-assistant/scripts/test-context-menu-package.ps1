param(
  [switch]$PauseForVisualTest,
  [string]$VisualTestReadyFile,
  [string]$VisualTestContinueFile
)

$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
$resourceDirectory = Join-Path $projectRoot 'src-tauri\resources'
$quickExtractPackagePath = Join-Path $resourceDirectory 'long_compress_context_menu_extract.msix'
$quickPackPackagePath = Join-Path $resourceDirectory 'long_compress_context_menu_pack.msix'
$packagePaths = @($quickExtractPackagePath, $quickPackPackagePath)
$legacyPackagePath = Join-Path $resourceDirectory 'long_compress_context_menu.msix'
$packageArtifacts = @($packagePaths + $legacyPackagePath)
$registrationScript = Join-Path $resourceDirectory 'long_compress_context_menu_registration.ps1'
$shellManifest = Join-Path $projectRoot 'src-tauri\shell-extension\Cargo.toml'
$tauriConfig = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $projectRoot 'src-tauri\tauri.conf.json') | ConvertFrom-Json
$appExecutableName = "$($tauriConfig.package.productName).exe"
$appExecutable = Join-Path $projectRoot "src-tauri\target\release\$appExecutableName"
$tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$testRoot = Join-Path $tempRoot ("long-compress-context-menu-e2e-" + [guid]::NewGuid().ToString('N'))
$resolvedTestRoot = [System.IO.Path]::GetFullPath($testRoot)
if (-not $resolvedTestRoot.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
  throw "Refusing to use a test directory outside the system temp folder: $resolvedTestRoot"
}
if ($PauseForVisualTest) {
  foreach ($marker in @($VisualTestReadyFile, $VisualTestContinueFile)) {
    if (-not $marker) { throw 'Visual-test ready and continue marker paths are required.' }
    $resolvedMarker = [System.IO.Path]::GetFullPath($marker)
    if (-not $resolvedMarker.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
      throw "Refusing to use a visual-test marker outside the system temp folder: $resolvedMarker"
    }
  }
}

$currentIdentity = [Security.Principal.WindowsIdentity]::GetCurrent()
$currentPrincipal = [Security.Principal.WindowsPrincipal]::new($currentIdentity)
if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw 'This reversible AppX deployment test must be run from an elevated PowerShell window because Windows evaluates self-signed package trust from LocalMachine\TrustedPeople.'
}

$certificate = $null
$machineTrustedCertificate = $null
$packageBackups = @{}
$hadExistingPackage = $false
$smokeTaskName = $null
$smokeTaskFolder = $null
try {
  $hadExistingPackage = @(Get-AppxPackage -Name 'LongCompressAssistant.ContextMenu*' -ErrorAction SilentlyContinue).Count -gt 0
  if ($hadExistingPackage) {
    throw 'A LongCompressAssistant context-menu package is already registered; refusing to replace it during a development test.'
  }
  if (-not (Test-Path -LiteralPath $appExecutable)) {
    throw 'Build the release application before running the context-menu package test.'
  }
  New-Item -ItemType Directory -Path $resolvedTestRoot -Force | Out-Null
  $externalLocation = Join-Path $resolvedTestRoot 'external'
  $externalResources = Join-Path $externalLocation 'resources'
  New-Item -ItemType Directory -Path $externalResources -Force | Out-Null

  foreach ($packagePath in $packageArtifacts) {
    if (Test-Path -LiteralPath $packagePath) {
      $packageBackup = Join-Path $resolvedTestRoot ("existing-" + [IO.Path]::GetFileName($packagePath))
      Copy-Item -LiteralPath $packagePath -Destination $packageBackup
      $packageBackups[$packagePath] = $packageBackup
    }
  }

  $subject = 'CN=LongCompressAssistant Context Menu Development'
  $certificate = New-SelfSignedCertificate `
    -Type CodeSigningCert `
    -Subject $subject `
    -CertStoreLocation 'Cert:\CurrentUser\My' `
    -KeyAlgorithm RSA `
    -KeyLength 2048 `
    -HashAlgorithm SHA256 `
    -KeyExportPolicy Exportable `
    -NotAfter (Get-Date).AddDays(2)
  $passwordText = [guid]::NewGuid().ToString('N')
  $password = ConvertTo-SecureString $passwordText -AsPlainText -Force
  $pfxPath = Join-Path $resolvedTestRoot 'development-code-signing.pfx'
  $cerPath = Join-Path $resolvedTestRoot 'development-code-signing.cer'
  Export-PfxCertificate -Cert $certificate -FilePath $pfxPath -Password $password | Out-Null
  Export-Certificate -Cert $certificate -FilePath $cerPath | Out-Null
  $machineTrustedCertificate = Import-Certificate -FilePath $cerPath -CertStoreLocation 'Cert:\LocalMachine\TrustedPeople'

  $env:WINDOWS_CODE_SIGNING_PFX_BASE64 = [Convert]::ToBase64String([IO.File]::ReadAllBytes($pfxPath))
  $env:WINDOWS_CODE_SIGNING_PFX_PASSWORD = $passwordText
  $env:WINDOWS_CODE_SIGNING_PUBLISHER = $certificate.Subject
  $env:REQUIRE_WINDOWS_CONTEXT_MENU_PACKAGE = 'true'
  & (Join-Path $PSScriptRoot 'build-context-menu-package.ps1')
  if ($LASTEXITCODE -ne 0 -or @($packagePaths | Where-Object { -not (Test-Path -LiteralPath $_) }).Count -gt 0) {
    throw 'Both signed development context-menu packages were not produced.'
  }

  $shellDll = Get-ChildItem -LiteralPath $resourceDirectory -Filter 'long_compress_shell_extension_*.dll' -File | Select-Object -First 1
  if (-not $shellDll) { throw 'The staged shell extension DLL was not found.' }
  Copy-Item -LiteralPath $shellDll.FullName -Destination (Join-Path $externalResources $shellDll.Name)
  Copy-Item -LiteralPath $appExecutable -Destination (Join-Path $externalLocation $appExecutableName)

  $cargoManifest = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $projectRoot 'src-tauri\Cargo.toml')
  $versionMatch = [regex]::Match($cargoManifest, '(?m)^version\s*=\s*"(\d+)\.(\d+)\.(\d+)(?:[^\"]*)"')
  if (-not $versionMatch.Success) { throw 'Unable to determine the package version.' }
  $packageVersion = '{0}.{1}.{2}.0' -f $versionMatch.Groups[1].Value, $versionMatch.Groups[2].Value, $versionMatch.Groups[3].Value

  & $registrationScript -Action Install `
    -QuickExtractPackagePath $quickExtractPackagePath `
    -QuickPackPackagePath $quickPackPackagePath `
    -ExternalLocation $externalLocation `
    -PackageVersion $packageVersion
  cargo build --quiet --manifest-path $shellManifest --bin com_smoke
  if ($LASTEXITCODE -ne 0) { throw 'Failed to build the COM activation smoke-test executable.' }
  $cargoMetadata = cargo metadata --format-version 1 --no-deps --manifest-path $shellManifest | ConvertFrom-Json
  if ($LASTEXITCODE -ne 0) { throw 'Failed to locate the COM activation smoke-test executable.' }
  $smokeExecutable = Join-Path $cargoMetadata.target_directory 'debug\com_smoke.exe'
  if (-not (Test-Path -LiteralPath $smokeExecutable)) { throw "COM smoke-test executable not found: $smokeExecutable" }

  # An elevated UAC process does not load per-user COM registrations. Explorer
  # is the real medium-integrity client, so run the activation test through a
  # one-shot interactive task whose run level is explicitly LIMITED.
  $smokeOutput = Join-Path $resolvedTestRoot 'com-smoke-output.log'
  $smokeResult = Join-Path $resolvedTestRoot 'com-smoke-result.txt'
  $smokeRunner = Join-Path $resolvedTestRoot 'run-com-smoke.ps1'
  $standardUserCommand = @"
`$ErrorActionPreference = 'Stop'
try {
  `$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  `$principal = [Security.Principal.WindowsPrincipal]::new(`$identity)
  if (`$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'COM smoke test unexpectedly retained an elevated token.'
  }
  & '$smokeExecutable' --top-level-only *>&1 | Out-File -LiteralPath '$smokeOutput' -Encoding utf8
  if (`$LASTEXITCODE -ne 0) { throw "COM smoke test exited with code `$LASTEXITCODE." }
  Set-Content -LiteralPath '$smokeResult' -Value '0' -Encoding ascii
} catch {
  `$_ | Format-List * -Force | Out-File -LiteralPath '$smokeOutput' -Encoding utf8 -Append
  Set-Content -LiteralPath '$smokeResult' -Value '1' -Encoding ascii
}
"@
  Set-Content -LiteralPath $smokeRunner -Value $standardUserCommand -Encoding utf8
  $smokeTaskName = "LongCompressContextMenuSmoke-$([guid]::NewGuid().ToString('N'))"
  $taskService = New-Object -ComObject 'Schedule.Service'
  $taskService.Connect()
  $smokeTaskFolder = $taskService.GetFolder('\')
  $taskDefinition = $taskService.NewTask(0)
  $taskDefinition.RegistrationInfo.Description = 'Temporary Long Compress Assistant COM activation test'
  $taskDefinition.Principal.UserId = [Security.Principal.WindowsIdentity]::GetCurrent().Name
  $taskDefinition.Principal.LogonType = 3 # TASK_LOGON_INTERACTIVE_TOKEN
  $taskDefinition.Principal.RunLevel = 0 # TASK_RUNLEVEL_LUA
  $taskDefinition.Settings.Enabled = $true
  $taskDefinition.Settings.AllowDemandStart = $true
  $taskDefinition.Settings.ExecutionTimeLimit = 'PT2M'
  $taskAction = $taskDefinition.Actions.Create(0) # TASK_ACTION_EXEC
  $taskAction.Path = 'powershell.exe'
  $taskAction.Arguments = "-NoLogo -NoProfile -ExecutionPolicy Bypass -File `"$smokeRunner`""
  $taskAction.WorkingDirectory = $externalLocation
  $registeredTask = $smokeTaskFolder.RegisterTaskDefinition(
    $smokeTaskName,
    $taskDefinition,
    6, # TASK_CREATE_OR_UPDATE
    $null,
    $null,
    3, # TASK_LOGON_INTERACTIVE_TOKEN
    $null
  )
  [void]$registeredTask.Run($null)
  $smokeDeadline = [DateTime]::UtcNow.AddSeconds(60)
  while (-not (Test-Path -LiteralPath $smokeResult) -and [DateTime]::UtcNow -lt $smokeDeadline) {
    Start-Sleep -Milliseconds 200
  }
  if (-not (Test-Path -LiteralPath $smokeResult)) { throw 'Timed out waiting for the standard-user COM activation test.' }
  if (Test-Path -LiteralPath $smokeOutput) { Get-Content -LiteralPath $smokeOutput }
  if ((Get-Content -Raw -LiteralPath $smokeResult).Trim() -ne '0') {
    throw 'Packaged top-level COM activation smoke test failed.'
  }
  Write-Output 'Signed sparse-package registration and top-level COM activation succeeded.'
  if ($PauseForVisualTest) {
    Set-Content -LiteralPath $VisualTestReadyFile -Value 'ready' -Encoding ascii
    Write-Output 'Visual context-menu test is ready; waiting for the continue marker.'
    $visualTestDeadline = [DateTime]::UtcNow.AddMinutes(10)
    while (-not (Test-Path -LiteralPath $VisualTestContinueFile) -and [DateTime]::UtcNow -lt $visualTestDeadline) {
      Start-Sleep -Milliseconds 250
    }
    if (-not (Test-Path -LiteralPath $VisualTestContinueFile)) {
      throw 'Timed out waiting for visual context-menu test completion.'
    }
  }
} finally {
  Remove-Item Env:WINDOWS_CODE_SIGNING_PFX_BASE64 -ErrorAction SilentlyContinue
  Remove-Item Env:WINDOWS_CODE_SIGNING_PFX_PASSWORD -ErrorAction SilentlyContinue
  Remove-Item Env:WINDOWS_CODE_SIGNING_PUBLISHER -ErrorAction SilentlyContinue
  Remove-Item Env:REQUIRE_WINDOWS_CONTEXT_MENU_PACKAGE -ErrorAction SilentlyContinue
  if ($smokeTaskName -and $smokeTaskFolder) {
    try { $smokeTaskFolder.DeleteTask($smokeTaskName, 0) } catch { Write-Warning $_ }
  }
  if (-not $hadExistingPackage) {
    try { & $registrationScript -Action Uninstall } catch { Write-Warning $_ }
  }
  if ($machineTrustedCertificate) {
    Remove-Item -LiteralPath "Cert:\LocalMachine\TrustedPeople\$($machineTrustedCertificate.Thumbprint)" -Force -ErrorAction SilentlyContinue
  }
  if ($certificate) {
    Remove-Item -LiteralPath "Cert:\CurrentUser\My\$($certificate.Thumbprint)" -Force -ErrorAction SilentlyContinue
  }
  foreach ($packagePath in $packageArtifacts) {
    Remove-Item -LiteralPath $packagePath -Force -ErrorAction SilentlyContinue
    $packageBackup = $packageBackups[$packagePath]
    if ($packageBackup -and (Test-Path -LiteralPath $packageBackup)) {
      Copy-Item -LiteralPath $packageBackup -Destination $packagePath -Force
    }
  }
  if (Test-Path -LiteralPath $resolvedTestRoot) {
    Remove-Item -LiteralPath $resolvedTestRoot -Recurse -Force
  }
  if ($PauseForVisualTest) {
    Remove-Item -LiteralPath $VisualTestReadyFile -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $VisualTestContinueFile -Force -ErrorAction SilentlyContinue
  }
}
