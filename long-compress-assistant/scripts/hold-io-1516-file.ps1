param([string]$Path, [string]$Ready, [string]$Stop)
$ErrorActionPreference = 'Stop'
$stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::None)
try {
  [IO.File]::WriteAllText($Ready, 'locked')
  while (-not (Test-Path -LiteralPath $Stop)) { Start-Sleep -Milliseconds 100 }
} finally { $stream.Dispose() }
