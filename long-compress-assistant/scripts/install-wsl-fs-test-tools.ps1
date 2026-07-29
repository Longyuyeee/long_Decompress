param(
    [string]$Destination = (Join-Path $PSScriptRoot "..\test-results\wsl-fs-tools")
)

$ErrorActionPreference = "Stop"

function Convert-ToWslPath {
    param([Parameter(Mandatory = $true)][string]$WindowsPath)
    $fullPath = [System.IO.Path]::GetFullPath($WindowsPath)
    if ($fullPath -notmatch "^([A-Za-z]):\\(.*)$") {
        throw "WSL test-tool destination must be on a Windows drive: $fullPath"
    }
    $drive = $Matches[1].ToLowerInvariant()
    $remainder = $Matches[2].Replace("\", "/")
    return "/mnt/$drive/$remainder"
}

$destinationPath = [System.IO.Path]::GetFullPath($Destination)
$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$allowedPrefix = $repositoryRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
    [System.IO.Path]::DirectorySeparatorChar
if (-not $destinationPath.StartsWith($allowedPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "WSL filesystem test-tool destination must stay inside the repository: $destinationPath"
}
$wslDestination = Convert-ToWslPath $destinationPath
$extractPath = Join-Path $destinationPath "root"
$wslExtractPath = "$wslDestination/root"
$packages = @(
    @{
        Spec = "dosfstools=4.2-1.1build1"
        File = "dosfstools_4.2-1.1build1_amd64.deb"
        Sha256 = "ace1ecc42b4d842f8e7a931933824a626dcdb7d4916470833f330ccad82afd63"
    },
    @{
        Spec = "mtools=4.0.43-1build1"
        File = "mtools_4.0.43-1build1_amd64.deb"
        Sha256 = "deb50411c17b001c2400dd8a0146f39d12070a2fe5e92d734b1c6d0e73119262"
    },
    @{
        Spec = "libntfs-3g89t64=1:2022.10.3-1.2ubuntu3"
        File = "libntfs-3g89t64_1%3a2022.10.3-1.2ubuntu3_amd64.deb"
        Sha256 = "37dcdf2bfad2f88ebc0ed62de0e26b5afdc6e8f31790da138adcd23c749a5956"
    },
    @{
        Spec = "ntfs-3g=1:2022.10.3-1.2ubuntu3"
        File = "ntfs-3g_1%3a2022.10.3-1.2ubuntu3_amd64.deb"
        Sha256 = "440c02fce744c0ff808772aa4b68b852c82d3a090cd0e9f86a5fe53aaa8867ce"
    }
)

New-Item -ItemType Directory -Path $destinationPath -Force | Out-Null
if (Test-Path -LiteralPath $extractPath) {
    Remove-Item -LiteralPath $extractPath -Recurse -Force
}
New-Item -ItemType Directory -Path $extractPath -Force | Out-Null

foreach ($package in $packages) {
    $packagePath = Join-Path $destinationPath $package.File
    $validPackage = (Test-Path -LiteralPath $packagePath) -and
        ((Get-FileHash -LiteralPath $packagePath -Algorithm SHA256).Hash.ToLowerInvariant() -eq $package.Sha256)
    if (-not $validPackage) {
        Remove-Item -LiteralPath $packagePath -Force -ErrorAction SilentlyContinue
        & wsl.exe -d Ubuntu --cd $wslDestination -- /usr/bin/apt-get download $package.Spec
        if ($LASTEXITCODE -ne 0) {
            throw "Could not download pinned Ubuntu package $($package.Spec)."
        }
    }
    $actualHash = (Get-FileHash -LiteralPath $packagePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne $package.Sha256) {
        throw "Ubuntu package SHA-256 mismatch: $($package.File)"
    }
    & wsl.exe -d Ubuntu -- /usr/bin/dpkg-deb -x "$wslDestination/$($package.File)" $wslExtractPath
    if ($LASTEXITCODE -ne 0) {
        throw "Could not extract Ubuntu package $($package.File)."
    }
}

$requiredTools = @(
    (Join-Path $extractPath "usr\sbin\mkfs.fat"),
    (Join-Path $extractPath "usr\bin\mcopy"),
    (Join-Path $extractPath "sbin\mkntfs"),
    (Join-Path $extractPath "sbin\ntfscp")
)
foreach ($tool in $requiredTools) {
    if (-not (Test-Path -LiteralPath $tool)) {
        throw "Expected filesystem test tool was not extracted: $tool"
    }
}

Write-Output $extractPath
