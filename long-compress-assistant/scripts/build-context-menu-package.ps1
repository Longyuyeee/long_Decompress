$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
$resourceDirectory = Join-Path $projectRoot 'src-tauri\resources'
$templatePath = Join-Path $projectRoot 'src-tauri\windows-context-menu\AppxManifest.xml.template'
$tauriConfig = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $projectRoot 'src-tauri\tauri.conf.json') | ConvertFrom-Json
$appExecutableName = "$($tauriConfig.package.productName).exe"
$packageDefinitions = @(
  @{
    FileName = 'long_compress_context_menu_extract.msix'
    PackageName = 'LongCompressAssistant.ContextMenu.QuickExtract'
    ApplicationId = 'LongCompressQuickExtract'
    DisplayNameBase64 = '5LiA6ZSu6Kej5Y6L5Yiw5ZCM5ZCN5paH5Lu25aS5'
    DescriptionBase64 = '5bCG5Y6L57yp5YyF6Kej5Y6L5Yiw5ZCM5ZCN5paH5Lu25aS5'
    SurrogateDisplayNameBase64 = '6IOn6Kej5Y6L5b+r5o236Kej5Y6L5ZG95Luk'
    Clsid = 'D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4B'
    ContextMenuItems = @'
            <desktop5:ItemType Type="*">
              <desktop5:Verb Id="LongDecompressQuickExtract" Clsid="D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4B" />
            </desktop5:ItemType>
'@
  },
  @{
    FileName = 'long_compress_context_menu_pack.msix'
    PackageName = 'LongCompressAssistant.ContextMenu.QuickPack'
    ApplicationId = 'LongCompressQuickPack'
    DisplayNameBase64 = '5LiA6ZSu5omT5YyF5Li6IFpJUA=='
    DescriptionBase64 = '5bCG5omA6YCJ5paH5Lu25omT5YyF5Li6IFpJUA=='
    SurrogateDisplayNameBase64 = '6IOn6Kej5Y6L5b+r5o235omT5YyF5ZG95Luk'
    Clsid = 'D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4C'
    ContextMenuItems = @'
            <desktop5:ItemType Type="*">
              <desktop5:Verb Id="LongDecompressQuickPack" Clsid="D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4C" />
            </desktop5:ItemType>
            <desktop5:ItemType Type="Directory">
              <desktop5:Verb Id="LongDecompressQuickPackDirectory" Clsid="D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4C" />
            </desktop5:ItemType>
'@
  }
)
$outputPackages = $packageDefinitions | ForEach-Object { Join-Path $resourceDirectory $_.FileName }
$legacyOutputPackage = Join-Path $resourceDirectory 'long_compress_context_menu.msix'

@($outputPackages + $legacyOutputPackage) | ForEach-Object {
  Remove-Item -LiteralPath $_ -Force -ErrorAction SilentlyContinue
}

$pfxBase64 = $env:WINDOWS_CODE_SIGNING_PFX_BASE64
$pfxPassword = $env:WINDOWS_CODE_SIGNING_PFX_PASSWORD
$publisher = $env:WINDOWS_CODE_SIGNING_PUBLISHER
if (-not $pfxBase64 -or -not $pfxPassword -or -not $publisher) {
  if ($env:REQUIRE_WINDOWS_CONTEXT_MENU_PACKAGE -eq 'true') {
    throw 'Windows 11 context-menu package is required, but one or more code-signing variables are missing.'
  }
  Write-Warning 'Skipping Windows 11 context-menu identity package: code-signing environment variables are not configured.'
  exit 0
}

$sdkBin = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Directory |
  Sort-Object Name -Descending |
  ForEach-Object { Join-Path $_.FullName 'x64' } |
  Where-Object { (Test-Path (Join-Path $_ 'makeappx.exe')) -and (Test-Path (Join-Path $_ 'signtool.exe')) } |
  Select-Object -First 1
if (-not $sdkBin) {
  throw 'Windows SDK MakeAppx.exe and SignTool.exe are required to build the context-menu identity package.'
}

$cargoManifest = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $projectRoot 'src-tauri\Cargo.toml')
$versionMatch = [regex]::Match($cargoManifest, '(?m)^version\s*=\s*"(\d+)\.(\d+)\.(\d+)(?:[^\"]*)"')
if (-not $versionMatch.Success) {
  throw 'Unable to determine the application version.'
}
$packageVersion = '{0}.{1}.{2}.0' -f $versionMatch.Groups[1].Value, $versionMatch.Groups[2].Value, $versionMatch.Groups[3].Value
$shellDll = Get-ChildItem -LiteralPath $resourceDirectory -Filter 'long_compress_shell_extension_*.dll' -File | Select-Object -First 1
if (-not $shellDll) {
  throw 'Build the shell extension before building the context-menu identity package.'
}

$stagingDirectory = Join-Path ([System.IO.Path]::GetTempPath()) ("long-compress-context-menu-" + [guid]::NewGuid().ToString('N'))
$pfxPath = "$stagingDirectory.pfx"
$tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$resolvedStagingDirectory = [System.IO.Path]::GetFullPath($stagingDirectory)
if (-not $resolvedStagingDirectory.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
  throw "Refusing to use a staging directory outside the system temp folder: $resolvedStagingDirectory"
}
try {
  New-Item -ItemType Directory -Path $stagingDirectory -Force | Out-Null
  $stagedResourceDirectory = Join-Path $stagingDirectory 'resources'
  New-Item -ItemType Directory -Path $stagedResourceDirectory -Force | Out-Null
  Copy-Item -LiteralPath $shellDll.FullName -Destination (Join-Path $stagedResourceDirectory $shellDll.Name)
  [System.IO.File]::WriteAllBytes($pfxPath, [Convert]::FromBase64String($pfxBase64))
  $certificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new(
    $pfxPath,
    $pfxPassword,
    [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::EphemeralKeySet
  )
  if ($certificate.Subject -ne $publisher) {
    throw "WINDOWS_CODE_SIGNING_PUBLISHER does not match the PFX subject. Expected '$($certificate.Subject)'."
  }
  if (-not $certificate.HasPrivateKey) {
    throw 'The Windows code-signing PFX does not contain a private key.'
  }

  foreach ($definition in $packageDefinitions) {
    $outputPackage = Join-Path $resourceDirectory $definition.FileName
    $displayName = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($definition.DisplayNameBase64))
    $description = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($definition.DescriptionBase64))
    $surrogateDisplayName = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($definition.SurrogateDisplayNameBase64))
    $manifest = Get-Content -Raw -Encoding utf8 -LiteralPath $templatePath
    $manifest = $manifest.Replace('__PUBLISHER__', [System.Security.SecurityElement]::Escape($publisher))
    $manifest = $manifest.Replace('__VERSION__', $packageVersion)
    $manifest = $manifest.Replace('__PACKAGE_NAME__', $definition.PackageName)
    $manifest = $manifest.Replace('__APPLICATION_ID__', $definition.ApplicationId)
    $manifest = $manifest.Replace('__DISPLAY_NAME__', [System.Security.SecurityElement]::Escape($displayName))
    $manifest = $manifest.Replace('__DESCRIPTION__', [System.Security.SecurityElement]::Escape($description))
    $manifest = $manifest.Replace('__SURROGATE_DISPLAY_NAME__', [System.Security.SecurityElement]::Escape($surrogateDisplayName))
    $manifest = $manifest.Replace('__COMMAND_CLSID__', $definition.Clsid)
    $manifest = $manifest.Replace('__CONTEXT_MENU_ITEMS__', $definition.ContextMenuItems.TrimEnd())
    $manifest = $manifest.Replace('__APP_EXECUTABLE__', [System.Security.SecurityElement]::Escape($appExecutableName))
    $manifest = $manifest.Replace('__SHELL_EXTENSION_DLL__', $shellDll.Name)
    Set-Content -LiteralPath (Join-Path $stagingDirectory 'AppxManifest.xml') -Value $manifest -Encoding utf8

    & (Join-Path $sdkBin 'makeappx.exe') pack /o /d $stagingDirectory /nv /p $outputPackage
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx failed to build $($definition.FileName)." }
    & (Join-Path $sdkBin 'signtool.exe') sign /fd SHA256 /f $pfxPath /p $pfxPassword $outputPackage
    if ($LASTEXITCODE -ne 0) { throw "SignTool failed to sign $($definition.FileName)." }
    & (Join-Path $sdkBin 'signtool.exe') verify /pa /all $outputPackage
    if ($LASTEXITCODE -ne 0) { throw "Signature verification failed for $($definition.FileName)." }
    Write-Output "Signed Windows 11 context-menu package staged at $outputPackage"
  }
} finally {
  if (Test-Path -LiteralPath $resolvedStagingDirectory) {
    Remove-Item -LiteralPath $resolvedStagingDirectory -Recurse -Force
  }
  Remove-Item -LiteralPath $pfxPath -Force -ErrorAction SilentlyContinue
}
