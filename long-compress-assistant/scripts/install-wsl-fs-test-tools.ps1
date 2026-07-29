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
        Spec = "libntfs-3g89t64=1:2022.10.3-1.2ubuntu3.2"
        File = "libntfs-3g89t64_1%3a2022.10.3-1.2ubuntu3.2_amd64.deb"
        Sha256 = "4ae668265884cbccb44d01a389cee7909343916627d21b04a7f202719d916071"
    },
    @{
        Spec = "ntfs-3g=1:2022.10.3-1.2ubuntu3.2"
        File = "ntfs-3g_1%3a2022.10.3-1.2ubuntu3.2_amd64.deb"
        Sha256 = "3a34223b68b55596eb91768b9c879b8a740b16467f56673ea2b1c0e8093d678e"
    }
)

New-Item -ItemType Directory -Path $destinationPath -Force | Out-Null
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
