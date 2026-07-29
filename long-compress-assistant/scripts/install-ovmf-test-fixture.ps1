param(
    [string]$Destination = (Join-Path $PSScriptRoot "..\test-results\ovmf-fixture")
)

$ErrorActionPreference = "Stop"

function Convert-ToWslPath {
    param([Parameter(Mandatory = $true)][string]$WindowsPath)
    $fullPath = [System.IO.Path]::GetFullPath($WindowsPath)
    if ($fullPath -notmatch "^([A-Za-z]):\\(.*)$") {
        throw "OVMF fixture destination must be on a Windows drive: $fullPath"
    }
    $drive = $Matches[1].ToLowerInvariant()
    $remainder = $Matches[2].Replace("\", "/")
    return "/mnt/$drive/$remainder"
}

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$destinationPath = [System.IO.Path]::GetFullPath($Destination)
$allowedPrefix = $repositoryRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
    [System.IO.Path]::DirectorySeparatorChar
if (-not $destinationPath.StartsWith($allowedPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "OVMF fixture destination must stay inside the repository: $destinationPath"
}

$packageSpec = "ovmf=2024.02-2"
$packageFile = "ovmf_2024.02-2_all.deb"
$packageSha256 = "f266d23604c4ca119f3dde83f2d7a65dad6cc2b93f0a8e07ee7b9eea1f64b217"
$firmwareSha256 = "b44425a582c7ca4f92662942c63d4ed94cbb365d04f62b7428e048cc88cfc22d"
$extractPath = Join-Path $destinationPath "root"
$firmwarePath = Join-Path $extractPath "usr\share\OVMF\OVMF_CODE_4M.fd"
$wslDestination = Convert-ToWslPath $destinationPath
$wslExtractPath = "$wslDestination/root"

New-Item -ItemType Directory -Path $destinationPath -Force | Out-Null
$packagePath = Join-Path $destinationPath $packageFile
$validPackage = (Test-Path -LiteralPath $packagePath) -and
    ((Get-FileHash -LiteralPath $packagePath -Algorithm SHA256).Hash.ToLowerInvariant() -eq $packageSha256)
if (-not $validPackage) {
    Remove-Item -LiteralPath $packagePath -Force -ErrorAction SilentlyContinue
    & wsl.exe -d Ubuntu --cd $wslDestination -- /usr/bin/apt-get download $packageSpec
    if ($LASTEXITCODE -ne 0) {
        throw "Could not download the pinned Ubuntu OVMF package."
    }
}
if ((Get-FileHash -LiteralPath $packagePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $packageSha256) {
    throw "Ubuntu OVMF package SHA-256 mismatch."
}
& wsl.exe -d Ubuntu -- /usr/bin/dpkg-deb -x "$wslDestination/$packageFile" $wslExtractPath
if ($LASTEXITCODE -ne 0) {
    throw "Could not extract the pinned Ubuntu OVMF package."
}
if (-not (Test-Path -LiteralPath $firmwarePath)) {
    throw "Expected OVMF firmware fixture was not extracted: $firmwarePath"
}
if ((Get-FileHash -LiteralPath $firmwarePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $firmwareSha256) {
    throw "Extracted OVMF firmware SHA-256 mismatch."
}

Write-Output $firmwarePath
