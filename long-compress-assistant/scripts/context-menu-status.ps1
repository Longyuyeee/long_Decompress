# Read-only compatibility helpers for the installed-release/update checks.
# Classic COM menus use the historical Native verb key; inspect the provider,
# rather than confusing a key name with the Windows 11 sparse-menu location.
function Test-DynamicClassicContextMenu {
  $root = 'HKCU:\Software\Classes\CLSID\{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4A}'
  return (Get-ItemProperty -LiteralPath $root -Name MenuProvider -ErrorAction SilentlyContinue).MenuProvider -eq 'classic'
}

function Get-DynamicClassicContextMenuStatus {
  param([string]$ExpectedExecutable)
  if (-not (Test-DynamicClassicContextMenu)) { return $null }
  $errors = [Collections.Generic.List[string]]::new()
  $version = [regex]::Match([Diagnostics.FileVersionInfo]::GetVersionInfo($ExpectedExecutable).ProductVersion, '\d+\.\d+\.\d+').Value
  $expectedDllName = 'long_compress_shell_extension_' + $version.Replace('.', '_') + '.dll'
  $clsid = '{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4D}'
  foreach ($class in @('*', 'Directory')) {
    $path = "HKCU:\Software\Classes\$class\shell\LongDecompressNative"
    $entry = Get-ItemProperty -LiteralPath $path -ErrorAction SilentlyContinue
    if ($entry.ExplorerCommandHandler -ne $clsid -or $entry.MultiSelectModel -ne 'Player') {
      [void]$errors.Add("incomplete dynamic root: $class")
    }
    if (Test-Path -LiteralPath "HKCU:\Software\Classes\$class\shell\LongDecompress") {
      [void]$errors.Add("obsolete duplicate root: $class")
    }
  }
  foreach ($id in @('{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4A}', $clsid)) {
    $path = "HKCU:\Software\Classes\CLSID\$id"
    $entry = Get-ItemProperty -LiteralPath $path -ErrorAction SilentlyContinue
    if ([string]$entry.ApplicationPath -ine $ExpectedExecutable) { [void]$errors.Add("wrong application target: $id") }
    $server = Get-Item -LiteralPath "$path\InprocServer32" -ErrorAction SilentlyContinue
    $dll = if ($server) { [string]$server.GetValue('') } else { '' }
    $resourceDirectory = Join-Path (Split-Path -Parent $ExpectedExecutable) 'resources'
    if (-not $dll -or -not (Test-Path -LiteralPath $dll) -or
        (Split-Path -Parent $dll) -ine $resourceDirectory -or
        (Split-Path -Leaf $dll) -ine $expectedDllName) { [void]$errors.Add("missing or wrong installed DLL: $id") }
  }
  return [pscustomobject]@{ valid = $errors.Count -eq 0; detail = if ($errors.Count) { $errors -join '; ' } else { 'single classic COM provider and current installed targets verified; commands are covered by the Shell catalog test' }; commandCount = 0; quickActionCount = 0 }
}
