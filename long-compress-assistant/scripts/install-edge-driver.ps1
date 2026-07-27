param(
    [Parameter(Mandatory = $true)]
    [string]$Destination
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$webViewRoots = @(
    "C:\Program Files (x86)\Microsoft\EdgeWebView\Application",
    "C:\Program Files\Microsoft\EdgeWebView\Application",
    (Join-Path $env:LOCALAPPDATA "Microsoft\EdgeWebView\Application")
)
$webViewCandidates = $webViewRoots |
    Where-Object { Test-Path -LiteralPath $_ } |
    ForEach-Object {
        Get-ChildItem -LiteralPath $_ -Directory |
            Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
            ForEach-Object {
                $executable = Join-Path $_.FullName "msedgewebview2.exe"
                if (Test-Path -LiteralPath $executable) {
                    [pscustomobject]@{
                        Version = [version]$_.Name
                        Path = $executable
                    }
                }
            }
    } |
    Sort-Object Version -Descending
$webView = $webViewCandidates | Select-Object -First 1
if (-not $webView) {
    throw "Microsoft Edge WebView2 Runtime was not found; a matching EdgeDriver cannot be installed."
}

$webViewVersion = $webView.Version.ToString()

$destinationPath = [System.IO.Path]::GetFullPath($Destination)
New-Item -ItemType Directory -Path $destinationPath -Force | Out-Null
$archivePath = Join-Path $destinationPath "edgedriver_win64.zip"
$driverPath = Join-Path $destinationPath "msedgedriver.exe"
$downloadUrl = "https://msedgedriver.microsoft.com/$webViewVersion/edgedriver_win64.zip"

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
if ($LASTEXITCODE -ne 0 -or $driverVersion -notmatch [regex]::Escape($webViewVersion)) {
    throw "EdgeDriver version validation failed. WebView2=$webViewVersion Driver=$driverVersion"
}

Write-Output $driverPath
