param(
  [Parameter(Mandatory = $true)]
  [ValidatePattern('^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$')]
  [string]$Version,

  [Parameter(Mandatory = $true)]
  [ValidatePattern('^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$')]
  [string]$PreviousVersion,

  [Parameter(Mandatory = $true)]
  [long]$ReleaseRunId,

  [Parameter(Mandatory = $true)]
  [int]$ValidationIssue,

  [Parameter(Mandatory = $true)]
  [string]$EvidencePath,

  [Parameter(Mandatory = $true)]
  [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$repository = 'Longyuyeee/long_Decompress'
$tag = "v$Version"
$resolvedEvidencePath = [IO.Path]::GetFullPath($EvidencePath)
$resolvedOutputPath = [IO.Path]::GetFullPath($OutputPath)

if (-not (Test-Path -LiteralPath $resolvedEvidencePath -PathType Leaf)) {
  throw "Release validation evidence does not exist: $resolvedEvidencePath"
}
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
  throw 'GitHub CLI (gh) is required to generate a release validation report.'
}

$release = gh release view $tag --repo $repository --json tagName,targetCommitish,publishedAt,url,assets |
  ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "Unable to read GitHub Release $tag." }
$run = gh run view $ReleaseRunId --repo $repository --json conclusion,headSha,url,createdAt,updatedAt |
  ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "Unable to read GitHub Actions run $ReleaseRunId." }
$issue = gh issue view $ValidationIssue --repo $repository --json state,title,url |
  ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "Unable to read release validation issue #$ValidationIssue." }
$evidence = Get-Content -Raw -Encoding utf8 -LiteralPath $resolvedEvidencePath | ConvertFrom-Json
$manifest = Invoke-RestMethod "https://github.com/$repository/releases/download/$tag/latest.json"

$expectedAssets = @(
  "Long-Decompress_${Version}_x64-setup.exe",
  "Long-Decompress_${Version}_x64-setup.nsis.zip",
  "Long-Decompress_${Version}_x64-setup.nsis.zip.sig",
  'latest.json'
)
$assetsByName = @{}
foreach ($asset in $release.assets) { $assetsByName[[string]$asset.name] = $asset }
$missingAssets = @($expectedAssets | Where-Object { -not $assetsByName.ContainsKey($_) })
$installer = $assetsByName["Long-Decompress_${Version}_x64-setup.exe"]

$checks = [Collections.Generic.List[object]]::new()
function Add-ValidationCheck {
  param([string]$Name, [bool]$Passed, [string]$Detail)
  [void]$checks.Add([pscustomobject]@{
    name = $Name
    passed = $Passed
    detail = $Detail
  })
}

Add-ValidationCheck 'Release tag matches' ($release.tagName -eq $tag) (
  "expected=$tag; actual=$($release.tagName)"
)
Add-ValidationCheck 'Release workflow succeeded' ($run.conclusion -eq 'success') (
  "conclusion=$($run.conclusion); head=$($run.headSha)"
)
$assetCheckDetail = if ($missingAssets.Count -eq 0) {
  $expectedAssets -join ', '
} else {
  "missing=$($missingAssets -join ', ')"
}
Add-ValidationCheck 'Required assets exist' ($missingAssets.Count -eq 0) $assetCheckDetail
Add-ValidationCheck 'Updater manifest version matches' ($manifest.version -eq $Version) (
  "expected=$Version; actual=$($manifest.version)"
)
$windowsUpdater = $manifest.platforms.'windows-x86_64'
Add-ValidationCheck 'Updater manifest is signed' (
  [bool]$windowsUpdater.signature -and [bool]$windowsUpdater.url
) ([string]$windowsUpdater.url)
Add-ValidationCheck 'Public update evidence targets this release' (
  $evidence.previousVersion -eq $PreviousVersion -and $evidence.targetVersion -eq $Version
) "expected=$PreviousVersion->$Version; actual=$($evidence.previousVersion)->$($evidence.targetVersion)"
Add-ValidationCheck 'Public update automation succeeded' (
  $evidence.succeeded -eq $true -and [int]$evidence.failedChecks -eq 0
) "succeeded=$($evidence.succeeded); failedChecks=$($evidence.failedChecks)"

$cascade = $evidence.updated.contextMenuCascade
$strictContextMenuPassed = (
  $null -ne $cascade -and
  $cascade.valid -eq $true -and
  [int]$cascade.commandCount -eq 17
)
$contextMenuDetail = if ($null -eq $cascade) {
  'missing contextMenuCascade evidence; legacy root-only checks are insufficient'
} else {
  "valid=$($cascade.valid); commandCount=$($cascade.commandCount); detail=$($cascade.detail)"
}
Add-ValidationCheck 'Traditional Explorer submenus were strictly validated' `
  $strictContextMenuPassed $contextMenuDetail

$failedChecks = @($checks | Where-Object { -not $_.passed })
$overall = if ($failedChecks.Count -eq 0) { 'PASS' } else { 'INCOMPLETE' }
$installerDigest = if ($installer -and $installer.digest) { [string]$installer.digest } else { 'unavailable' }
$evidenceRelative = $resolvedEvidencePath
try {
  $repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
  if ($resolvedEvidencePath.StartsWith($repoRoot, [StringComparison]::OrdinalIgnoreCase)) {
    $evidenceRelative = $resolvedEvidencePath.Substring($repoRoot.Length).TrimStart('\', '/')
  }
} catch {
  $evidenceRelative = $resolvedEvidencePath
}

$checkRows = $checks | ForEach-Object {
  $result = if ($_.passed) { 'PASS' } else { 'INCOMPLETE' }
  $detail = ([string]$_.detail).Replace('|', '\|').Replace("`r", ' ').Replace("`n", ' ')
  "| $($_.name) | $result | $detail |"
}
$assetRows = $expectedAssets | ForEach-Object {
  $asset = $assetsByName[$_]
  if ($asset) {
    "| $($asset.name) | $($asset.size) | $($asset.digest) |"
  } else {
    "| $_ | - | MISSING |"
  }
}

$markdown = @"
# Long Decompress $Version Release Validation

> Generated: $((Get-Date).ToUniversalTime().ToString('yyyy-MM-dd HH:mm:ss')) UTC  
> Overall result: **$overall**

## Traceability

| Item | Value |
| --- | --- |
| Release | [$tag]($($release.url)) |
| Release commit | ``$($release.targetCommitish)`` |
| Release Actions | [run $ReleaseRunId]($($run.url)) |
| Validation issue | [#$ValidationIssue]($($issue.url)) ($($issue.state)) |
| Previous release | ``v$PreviousVersion`` |
| Public-update evidence | ``$evidenceRelative`` |
| Installer SHA-256 | ``$installerDigest`` |

## Automated checks

| Check | Result | Evidence |
| --- | --- | --- |
$($checkRows -join "`n")

## Release assets

| Asset | Bytes | GitHub digest |
| --- | ---: | --- |
$($assetRows -join "`n")

## Decision rule

The report returns ``PASS`` only when the Release, Actions run, four required assets, signed updater, public upgrade, and all 17 commands below the four classic Explorer menu roots have direct evidence. Older evidence that checks only the ``LongDecompress`` root or one CommandStore verb is reported as ``INCOMPLETE``.

Generated by ``scripts/generate-release-validation.ps1``. The linked GitHub validation issue is authoritative for discussion, defects, and fixes.
"@

$outputDirectory = Split-Path -Parent $resolvedOutputPath
if ($outputDirectory) { [IO.Directory]::CreateDirectory($outputDirectory) | Out-Null }
[IO.File]::WriteAllText($resolvedOutputPath, $markdown, [Text.UTF8Encoding]::new($false))
Write-Output "Release validation report: $resolvedOutputPath"
Write-Output "Overall result: $overall"
if ($failedChecks.Count -gt 0) {
  Write-Output "Incomplete checks: $($failedChecks.name -join ', ')"
}
