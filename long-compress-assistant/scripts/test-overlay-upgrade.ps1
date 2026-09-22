param(
  [Parameter(Mandatory=$true)][string]$Installer,
  [Parameter(Mandatory=$true)][string]$CandidateExecutable,
  [Parameter(Mandatory=$true)][string]$Version,
  [switch]$AllowExistingInstall
)
$ErrorActionPreference = 'Stop'
if (-not $AllowExistingInstall) { throw 'Explicit approval to close and upgrade the installed application is required.' }
$installerPath = (Resolve-Path -LiteralPath $Installer).Path
$candidatePath = (Resolve-Path -LiteralPath $CandidateExecutable).Path
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Long解压'
$baseline = Get-ItemProperty -LiteralPath $key
$installPath = [IO.Path]::GetFullPath($baseline.InstallLocation.Trim('"'))
if ((Split-Path $installPath -Leaf) -ne 'Long解压') { throw 'Unexpected installation directory.' }
$exePath = Join-Path $installPath 'Long解压.exe'
$dataPaths = @('LongDecompress', 'com.longcompress.assistant') | ForEach-Object {
  [IO.Path]::GetFullPath((Join-Path $env:APPDATA $_))
}
$evidenceRoot = Join-Path (Split-Path $PSScriptRoot -Parent) ('test-results\overlay-upgrade-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $evidenceRoot | Out-Null
function Fingerprint([string]$Root) {
  if (-not (Test-Path -LiteralPath $Root)) { return @() }
  return @(Get-ChildItem -LiteralPath $Root -File -Recurse | Sort-Object FullName | ForEach-Object {
    ($_.FullName.Substring($Root.Length)) + ':' + (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
  })
}
$report = [ordered]@{ baselineRegistryVersion=$baseline.DisplayVersion; baselineExecutableVersion=(Get-Item -LiteralPath $exePath).VersionInfo.ProductVersion; candidateVersion=$Version; backupDirectory=$evidenceRoot; succeeded=$false }
try {
  Get-Process | Where-Object { $_.Path -eq $exePath } | ForEach-Object {
    Stop-Process -Id $_.Id -Force
    Wait-Process -Id $_.Id -Timeout 10 -ErrorAction SilentlyContinue
  }
  Copy-Item -LiteralPath $installPath -Destination (Join-Path $evidenceRoot 'installation') -Recurse
  if (Compare-Object @(Fingerprint $installPath) @(Fingerprint (Join-Path $evidenceRoot 'installation'))) { throw 'Installation backup mismatch.' }
  & reg.exe export 'HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Long解压' (Join-Path $evidenceRoot 'uninstall-registration.reg') /y | Out-Null
  if ($LASTEXITCODE -ne 0) { throw 'Registry backup failed.' }
  $before = @{}
  foreach ($path in $dataPaths) {
    $before[$path] = @(Fingerprint $path)
    if (Test-Path -LiteralPath $path) {
      $backup = Join-Path $evidenceRoot (Split-Path $path -Leaf)
      Copy-Item -LiteralPath $path -Destination $backup -Recurse
      if (Compare-Object $before[$path] @(Fingerprint $backup)) { throw 'User-data backup mismatch.' }
    }
  }
  $process = Start-Process -FilePath $installerPath -ArgumentList '/P','/NS','/NR' -WindowStyle Hidden -PassThru
  if (-not $process.WaitForExit(90000)) { throw 'Installer timed out; backup retained. Inspect before retrying.' }
  if ($process.ExitCode -ne 0) { throw "Installer exit: $($process.ExitCode)" }
  $after = Get-ItemProperty -LiteralPath $key
  if ($after.DisplayVersion -ne $Version -or $after.InstallLocation.Trim('"') -ne $installPath) { throw 'Installed registry identity mismatch.' }
  if ((Get-Item -LiteralPath $exePath).VersionInfo.ProductVersion -ne $Version) { throw 'Executable version mismatch.' }
  if ((Get-FileHash $exePath).Hash -ne (Get-FileHash $candidatePath).Hash) { throw 'Installed executable differs from candidate.' }
  $dlls = @(Get-ChildItem -LiteralPath (Join-Path $installPath 'resources') -Filter 'long_compress_shell_extension_*.dll')
  if ($dlls.Count -ne 1 -or $dlls[0].Name -ne ('long_compress_shell_extension_' + $Version.Replace('.','_') + '.dll')) { throw 'Shell extension identity mismatch.' }
  foreach ($path in $dataPaths) {
    if (Compare-Object $before[$path] @(Fingerprint $path)) { throw "User data changed during overlay: $(Split-Path $path -Leaf)" }
  }
  $report.installerSha256 = (Get-FileHash $installerPath).Hash
  $report.executableSha256 = (Get-FileHash $exePath).Hash
  $report.userDataUnchanged = $true
  $report.succeeded = $true
} finally {
  $report | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $evidenceRoot 'result.json') -Encoding utf8
  Write-Output "Evidence and retained backup: $evidenceRoot"
}
