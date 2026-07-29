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

$packageSpec = "ovmf=2024.02-2ubuntu0.9"
$packageFile = "ovmf_2024.02-2ubuntu0.9_all.deb"
$packageSha256 = "a094c13d06f2740691ff57d108dff32aa087179363ddb0de42d463b4f7f9bc13"
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

Write-Output $firmwarePath
