param(
    [Parameter(Mandatory = $true)]
    [string]$Destination
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$edgeCandidates = @(
    "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    "C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    (Join-Path $env:LOCALAPPDATA "Microsoft\Edge\Application\msedge.exe")
)
$edgePath = $edgeCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $edgePath) {
    throw "Microsoft Edge was not found; a matching EdgeDriver cannot be installed."
}

$edgeVersion = (Get-Item -LiteralPath $edgePath).VersionInfo.FileVersion
if (-not $edgeVersion) {
    throw "Unable to determine the installed Microsoft Edge version."
}

$destinationPath = [System.IO.Path]::GetFullPath($Destination)
New-Item -ItemType Directory -Path $destinationPath -Force | Out-Null
$archivePath = Join-Path $destinationPath "edgedriver_win64.zip"
$driverPath = Join-Path $destinationPath "msedgedriver.exe"
$downloadUrl = "https://msedgedriver.microsoft.com/$edgeVersion/edgedriver_win64.zip"

try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing
    Expand-Archive -LiteralPath $archivePath -DestinationPath $destinationPath -Force
} finally {
    Remove-Item -LiteralPath $archivePath -Force -ErrorAction SilentlyContinue
}

if (-not (Test-Path -LiteralPath $driverPath)) {
    throw "The EdgeDriver archive did not contain msedgedriver.exe."
}

$driverVersion = & $driverPath --version
if ($LASTEXITCODE -ne 0 -or $driverVersion -notmatch [regex]::Escape($edgeVersion)) {
    throw "EdgeDriver version validation failed. Edge=$edgeVersion Driver=$driverVersion"
}

Write-Output $driverPath
