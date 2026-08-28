param(
    [string]$DestinationPath
)

$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($DestinationPath)) {
    $DestinationPath = Join-Path $projectRoot 'test-results\hfsx-fixture'
}

$destination = [System.IO.Path]::GetFullPath($DestinationPath)
$testResultsRoot = [System.IO.Path]::GetFullPath((Join-Path $projectRoot 'test-results'))
if (-not $destination.StartsWith($testResultsRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "HFSX fixture output must stay inside $testResultsRoot"
}

$toolCommit = 'ec239599c1f234a4e01ae3fe51214d0c77e5baa3'
$emptyImageSha256 = 'E12EA3FDDF982870C9B8617F9D057D8F6E18CEBCD2F3A754E766A168385DDD4F'
$toolRoot = Join-Path $destination 'libdmg-hfsplus'
$stageRoot = Join-Path $destination 'stage'
$hfsPlusImage = Join-Path $destination 'payload.hfs'
$hfsxImage = Join-Path $destination 'payload.hfsx'
$verificationRoot = Join-Path $destination 'verification'
$payloadPath = Join-Path $stageRoot 'known-payload.txt'
$payloadText = "Long Decompress HFSX real payload`n"

function Convert-ToWslPath([string]$WindowsPath) {
    $fullPath = [System.IO.Path]::GetFullPath($WindowsPath)
    if ($fullPath -notmatch '^([A-Za-z]):\\(.*)$') {
        throw "Cannot convert path to WSL form: $fullPath"
    }
    $drive = $Matches[1].ToLowerInvariant()
    $tail = $Matches[2].Replace('\', '/')
    return "/mnt/$drive/$tail"
}

function Get-Sha256([string]$Path) {
    $stream = [System.IO.File]::OpenRead([System.IO.Path]::GetFullPath($Path))
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([System.BitConverter]::ToString($sha256.ComputeHash($stream))).Replace('-', '')
    }
    finally {
        $sha256.Dispose()
        $stream.Dispose()
    }
}

New-Item -ItemType Directory -Path $destination -Force | Out-Null
if (-not (Test-Path -LiteralPath (Join-Path $toolRoot '.git'))) {
    git clone --filter=blob:none --no-checkout https://github.com/mozilla/libdmg-hfsplus.git $toolRoot
    if ($LASTEXITCODE -ne 0) {
        throw 'Could not clone the pinned HFS+ fixture tool.'
    }
}

git -C $toolRoot fetch --depth 1 origin $toolCommit
if ($LASTEXITCODE -ne 0) {
    throw "Could not fetch HFS+ fixture tool commit $toolCommit."
}
git -C $toolRoot checkout --detach $toolCommit
if ($LASTEXITCODE -ne 0) {
    throw "Could not check out HFS+ fixture tool commit $toolCommit."
}
$actualCommit = (git -C $toolRoot rev-parse HEAD).Trim()
if ($actualCommit -ne $toolCommit) {
    throw "Unexpected HFS+ fixture tool commit: $actualCommit"
}

$emptyImage = Join-Path $toolRoot 'test\empty.hfs'
$actualEmptyImageSha256 = Get-Sha256 $emptyImage
if ($actualEmptyImageSha256 -ne $emptyImageSha256) {
    throw "Pinned empty HFS+ image hash mismatch: $actualEmptyImageSha256"
}

$toolExecutable = Join-Path $toolRoot 'hfsplus'
$wslToolRoot = Convert-ToWslPath $toolRoot
$compileArguments = @(
    '-d', 'Ubuntu', '--', 'gcc', '-O2', '-fno-strict-aliasing', '-Iincludes',
    'common/abstractfile.c', 'common/base64.c',
    'hfs/btree.c', 'hfs/catalog.c', 'hfs/extents.c', 'hfs/xattr.c',
    'hfs/fastunicodecompare.c', 'hfs/flatfile.c', 'hfs/hfslib.c',
    'hfs/rawfile.c', 'hfs/utility.c', 'hfs/volume.c', 'hfs/hfscompress.c',
    'hfs/hfs.c', '-lz', '-o', 'hfsplus'
)
Push-Location $toolRoot
try {
    & wsl.exe @compileArguments
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $toolExecutable)) {
        throw 'Could not compile the pinned HFS+ fixture tool in Ubuntu WSL.'
    }
}
finally {
    Pop-Location
}

if (Test-Path -LiteralPath $stageRoot) {
    Remove-Item -LiteralPath $stageRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $stageRoot -Force | Out-Null
[System.IO.File]::WriteAllText(
    $payloadPath,
    $payloadText,
    [System.Text.UTF8Encoding]::new($false)
)
Copy-Item -LiteralPath $emptyImage -Destination $hfsPlusImage -Force

$wslToolExecutable = "$wslToolRoot/hfsplus"
$wslHfsPlusImage = Convert-ToWslPath $hfsPlusImage
$wslStageRoot = "$(Convert-ToWslPath $stageRoot)/"
& wsl.exe -d Ubuntu -- $wslToolExecutable $wslHfsPlusImage addall $wslStageRoot
if ($LASTEXITCODE -ne 0) {
    throw 'Could not add the known payload to the HFS+ fixture.'
}

$bytes = [System.IO.File]::ReadAllBytes($hfsPlusImage)
function Read-UInt32BigEndian([byte[]]$Buffer, [int]$Offset) {
    return (
        ([uint32]$Buffer[$Offset] -shl 24) -bor
        ([uint32]$Buffer[$Offset + 1] -shl 16) -bor
        ([uint32]$Buffer[$Offset + 2] -shl 8) -bor
        [uint32]$Buffer[$Offset + 3]
    )
}

$primaryHeader = 1024
$alternateHeader = $bytes.Length - 1024
foreach ($headerOffset in @($primaryHeader, $alternateHeader)) {
    $bytes[$headerOffset] = 0x48
    $bytes[$headerOffset + 1] = 0x58
    $bytes[$headerOffset + 2] = 0x00
    $bytes[$headerOffset + 3] = 0x05
}

$blockSize = Read-UInt32BigEndian $bytes ($primaryHeader + 40)
$catalogStartBlock = Read-UInt32BigEndian $bytes ($primaryHeader + 272 + 16)
$catalogHeaderOffset = ($catalogStartBlock * $blockSize) + 14
$bytes[$catalogHeaderOffset + 37] = 0xBC
[System.IO.File]::WriteAllBytes($hfsxImage, $bytes)

$sevenZip = Join-Path $projectRoot 'src-tauri\resources\archive-engine\7z.exe'
$listing = (& $sevenZip l $hfsxImage 2>&1) -join "`n"
if ($LASTEXITCODE -ne 0 -or $listing -notmatch 'Method = HFSX') {
    throw "Generated image was not recognized as HFSX.`n$listing"
}

if (Test-Path -LiteralPath $verificationRoot) {
    Remove-Item -LiteralPath $verificationRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $verificationRoot -Force | Out-Null
& $sevenZip x $hfsxImage "-o$verificationRoot" -y | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw 'Bundled 7-Zip could not extract the generated HFSX fixture.'
}

$extractedPayload = Join-Path $verificationRoot 'Firefox\known-payload.txt'
$expectedHash = Get-Sha256 $payloadPath
$actualHash = Get-Sha256 $extractedPayload
if ($actualHash -ne $expectedHash) {
    throw "HFSX payload hash mismatch: expected=$expectedHash; actual=$actualHash"
}

Write-Output "Pinned HFSX fixture ready: $hfsxImage"
Write-Output "Payload SHA-256: $expectedHash"
