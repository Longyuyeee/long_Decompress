param(
    [string]$Destination = (Join-Path $PSScriptRoot "..\test-results\wix3-tool")
)

$ErrorActionPreference = "Stop"

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$destinationPath = [System.IO.Path]::GetFullPath($Destination)
$allowedPrefix = $repositoryRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
    [System.IO.Path]::DirectorySeparatorChar
if (-not $destinationPath.StartsWith($allowedPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "WiX test-tool destination must stay inside the repository: $destinationPath"
}

$archivePath = Join-Path $destinationPath "wix314-binaries.zip"
$extractPath = Join-Path $destinationPath "root"
$downloadUrl = "https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip"
$expectedSha256 = "6ac824e1642d6f7277d0ed7ea09411a508f6116ba6fae0aa5f2c7daa2ff43d31"

New-Item -ItemType Directory -Path $destinationPath -Force | Out-Null
$validArchive = (Test-Path -LiteralPath $archivePath) -and
    ((Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant() -eq $expectedSha256)
if (-not $validArchive) {
    Remove-Item -LiteralPath $archivePath -Force -ErrorAction SilentlyContinue
    & curl.exe -L --fail --retry 3 --output $archivePath $downloadUrl
    if ($LASTEXITCODE -ne 0) {
        throw "Could not download the pinned WiX Toolset 3.14.1 binaries."
    }
}

$actualHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualHash -ne $expectedSha256) {
    throw "WiX Toolset archive SHA-256 mismatch."
}

$requiredTools = @("candle.exe", "light.exe", "torch.exe", "pyro.exe")
$missingTools = @(
    $requiredTools | Where-Object {
        -not (Test-Path -LiteralPath (Join-Path $extractPath $_))
    }
)
if ($missingTools.Count -gt 0) {
    if (Test-Path -LiteralPath $extractPath) {
        Remove-Item -LiteralPath $extractPath -Recurse -Force
    }
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractPath -Force
}

foreach ($tool in $requiredTools) {
    $toolPath = Join-Path $extractPath $tool
    if (-not (Test-Path -LiteralPath $toolPath)) {
        throw "Expected WiX test tool was not extracted: $toolPath"
    }
}

Write-Output $extractPath
