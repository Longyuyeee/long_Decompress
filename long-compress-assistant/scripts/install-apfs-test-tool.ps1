param(
    [string]$Destination = (Join-Path $PSScriptRoot "..\test-results\apfs-tool")
)

$ErrorActionPreference = "Stop"

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$destinationPath = [System.IO.Path]::GetFullPath($Destination)
$allowedPrefix = $repositoryRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
    [System.IO.Path]::DirectorySeparatorChar
if (-not $destinationPath.StartsWith($allowedPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "APFS test-tool destination must stay inside the repository: $destinationPath"
}

$goArchiveName = "go1.26.5.windows-amd64.zip"
$goArchive = Join-Path $destinationPath $goArchiveName
$goSdk = Join-Path $destinationPath "go-sdk"
$goExecutable = Join-Path $goSdk "go\bin\go.exe"
$goUrl = "https://dl.google.com/go/$goArchiveName"
$goSha256 = "97e6b2a833b6d89f9ff17d25419ac0a7e3b482a044e9ab18cdef834bd834fd38"
$apfsSource = Join-Path $destinationPath "source"
$apfsCommit = "001fcf26671c6d457a2291c5abc2535f54f06ea4"
$generator = Join-Path $repositoryRoot "tests\fixtures\apfs-generator"

New-Item -ItemType Directory -Path $destinationPath -Force | Out-Null
$validGoArchive = (Test-Path -LiteralPath $goArchive) -and
    ((Get-FileHash -LiteralPath $goArchive -Algorithm SHA256).Hash.ToLowerInvariant() -eq $goSha256)
if (-not $validGoArchive) {
    Remove-Item -LiteralPath $goArchive -Force -ErrorAction SilentlyContinue
    & curl.exe --ssl-no-revoke -L --fail --retry 5 --retry-all-errors --output $goArchive $goUrl
    if ($LASTEXITCODE -ne 0) {
        throw "Could not download the pinned Go 1.26.5 SDK."
    }
}
if ((Get-FileHash -LiteralPath $goArchive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $goSha256) {
    throw "Go SDK archive SHA-256 mismatch."
}
if (
    -not (Test-Path -LiteralPath $goExecutable) -or
    -not (Test-Path -LiteralPath (Join-Path $goSdk "go\src\fmt\print.go"))
) {
    New-Item -ItemType Directory -Path $goSdk -Force | Out-Null
    & tar.exe -xf $goArchive -C $goSdk
    if ($LASTEXITCODE -ne 0) {
        throw "Could not extract the pinned Go SDK."
    }
}

if (-not (Test-Path -LiteralPath (Join-Path $apfsSource ".git"))) {
    & git clone https://github.com/go-filesystems/apfs.git $apfsSource
    if ($LASTEXITCODE -ne 0) {
        throw "Could not clone the APFS fixture generator dependency."
    }
}
& git -C $apfsSource fetch origin $apfsCommit
if ($LASTEXITCODE -ne 0) {
    throw "Could not fetch the pinned APFS implementation commit."
}
& git -C $apfsSource checkout --detach $apfsCommit
if ($LASTEXITCODE -ne 0) {
    throw "Could not check out the pinned APFS implementation commit."
}
$actualCommit = (& git -C $apfsSource rev-parse HEAD).Trim()
if ($actualCommit -ne $apfsCommit) {
    throw "APFS implementation commit mismatch."
}

$previousGoToolchain = $env:GOTOOLCHAIN
$previousGoModCache = $env:GOMODCACHE
$previousGoCache = $env:GOCACHE
try {
    $env:GOTOOLCHAIN = "local"
    $env:GOMODCACHE = Join-Path $destinationPath "mod-cache"
    $env:GOCACHE = Join-Path $destinationPath "build-cache"
    Push-Location $generator
    try {
        & $goExecutable mod download
        if ($LASTEXITCODE -ne 0) {
            throw "Could not download the pinned APFS generator dependencies."
        }
    }
    finally {
        Pop-Location
    }
}
finally {
    $env:GOTOOLCHAIN = $previousGoToolchain
    $env:GOMODCACHE = $previousGoModCache
    $env:GOCACHE = $previousGoCache
}

Write-Output $goExecutable
