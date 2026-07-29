param(
    [string]$Destination = (Join-Path $PSScriptRoot "..\test-results\qemu-img-tool")
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$version = "20260422"
$expectedSha512 = "64a43c0d39acddc9d30d290935a312a2b5c4fa62cffe6c27090f2a45ca6c8de0f0e8673e1e5117fb116a8742f86df92163531afc23f34758aadfc6d82c1f41a5"
$downloadUrl = "https://qemu.weilnetz.de/w64/2026/qemu-w64-setup-$version.exe"
$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$destinationPath = [System.IO.Path]::GetFullPath($Destination)
$allowedPrefix = $repositoryRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
    [System.IO.Path]::DirectorySeparatorChar
if (-not $destinationPath.StartsWith($allowedPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "QEMU test-tool destination must stay inside the repository: $destinationPath"
}
$installerPath = Join-Path $destinationPath "qemu-w64-setup-$version.exe"
$extractPath = Join-Path $destinationPath "root"
$toolPath = Join-Path $extractPath "qemu-img.exe"
$sevenZip = Join-Path $PSScriptRoot "..\src-tauri\resources\archive-engine\7z.exe"

New-Item -ItemType Directory -Path $destinationPath -Force | Out-Null

function Test-InstallerHash {
    if (-not (Test-Path -LiteralPath $installerPath)) {
        return $false
    }
    return (Get-FileHash -LiteralPath $installerPath -Algorithm SHA512).Hash.ToLowerInvariant() -eq $expectedSha512
}

if (-not (Test-InstallerHash)) {
    & curl.exe `
        --fail `
        --location `
        --retry 8 `
        --retry-all-errors `
        --continue-at - `
        --silent `
        --show-error `
        --output $installerPath `
        $downloadUrl
    if ($LASTEXITCODE -ne 0) {
        throw "QEMU test-tool download failed with exit code $LASTEXITCODE."
    }
}

if (-not (Test-InstallerHash)) {
    throw "QEMU installer SHA-512 does not match the pinned official checksum."
}
if (-not (Test-Path -LiteralPath $sevenZip)) {
    throw "Bundled 7-Zip was not found: $sevenZip"
}

if (Test-Path -LiteralPath $extractPath) {
    Remove-Item -LiteralPath $extractPath -Recurse -Force
}
New-Item -ItemType Directory -Path $extractPath -Force | Out-Null
& $sevenZip x $installerPath "-o$extractPath" "qemu-img.exe" "*.dll" -y | Out-Null
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $toolPath)) {
    throw "Could not extract qemu-img.exe and its runtime libraries."
}

$versionOutput = & $toolPath --version
if ($LASTEXITCODE -ne 0 -or $versionOutput[0] -notmatch "qemu-img version 11\.0\.0") {
    throw "qemu-img validation failed: $($versionOutput -join ' ')"
}

Write-Output $toolPath
