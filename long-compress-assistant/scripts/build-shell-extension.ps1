$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $projectRoot 'src-tauri\shell-extension\Cargo.toml'
$sourceDll = Join-Path $projectRoot 'src-tauri\shell-extension\target\release\long_compress_shell_extension.dll'
$resourceDirectory = Join-Path $projectRoot 'src-tauri\resources'
$manifest = Get-Content -Raw -LiteralPath $manifestPath
$versionMatch = [regex]::Match($manifest, '(?m)^version\s*=\s*"([^"]+)"')
if (-not $versionMatch.Success) {
  throw 'Unable to determine the shell extension version.'
}
$versionSuffix = $versionMatch.Groups[1].Value -replace '[^0-9A-Za-z]', '_'
$destinationDll = Join-Path $resourceDirectory "long_compress_shell_extension_$versionSuffix.dll"

cargo build --release --manifest-path $manifestPath
if ($LASTEXITCODE -ne 0) {
  throw 'Failed to build the Windows Explorer shell extension.'
}

New-Item -ItemType Directory -Path $resourceDirectory -Force | Out-Null
Get-ChildItem -LiteralPath $resourceDirectory -Filter 'long_compress_shell_extension*.dll' -File |
  Remove-Item -Force
Copy-Item -LiteralPath $sourceDll -Destination $destinationDll -Force
Write-Output "Shell extension staged at $destinationDll"
