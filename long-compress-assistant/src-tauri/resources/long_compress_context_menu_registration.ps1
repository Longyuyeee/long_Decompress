param(
  [Parameter(Mandatory = $true)]
  [ValidateSet('Install', 'Uninstall')]
  [string]$Action,
  [string]$QuickExtractPackagePath,
  [string]$QuickPackPackagePath,
  [string]$ExternalLocation,
  [string]$PackageVersion
)

$ErrorActionPreference = 'Stop'
$packageNames = @(
  'LongCompressAssistant.ContextMenu.QuickExtract',
  'LongCompressAssistant.ContextMenu.QuickPack'
)
$legacyPackageNames = @('LongCompressAssistant.ContextMenu')

$installedPackages = @(
  @($packageNames + $legacyPackageNames) |
    ForEach-Object { Get-AppxPackage -Name $_ -ErrorAction SilentlyContinue }
)

if ($Action -eq 'Install') {
  $packagePaths = @($QuickExtractPackagePath, $QuickPackPackagePath)
  foreach ($packagePath in $packagePaths) {
    if (-not $packagePath -or -not (Test-Path -LiteralPath $packagePath)) {
      throw "Context-menu identity package not found: $packagePath"
    }
  }
  if (-not $ExternalLocation -or -not (Test-Path -LiteralPath $ExternalLocation)) {
    throw "Application install directory not found: $ExternalLocation"
  }
  if (-not $PackageVersion) {
    throw 'PackageVersion is required when installing the context-menu identity package.'
  }
  $currentPackageNames = @($installedPackages | Where-Object { $_.Version.ToString() -eq $PackageVersion } | ForEach-Object Name)
  if (@($packageNames | Where-Object { $_ -notin $currentPackageNames }).Count -eq 0) {
    exit 0
  }
  $installedPackages | Remove-AppxPackage -ErrorAction Stop
  foreach ($packagePath in $packagePaths) {
    Add-AppxPackage -Path $packagePath -ExternalLocation $ExternalLocation
  }
} else {
  $installedPackages | Remove-AppxPackage -ErrorAction Stop
}
