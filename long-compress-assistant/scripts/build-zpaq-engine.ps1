param([switch]$TestIO)
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
$compilerRoot = & "${env:ProgramFiles(x86)}/Microsoft Visual Studio/Installer/vswhere.exe" -latest -property installationPath
if (!$compilerRoot) { throw 'Visual Studio C++ tools are required' }
$buildDirectory = Join-Path $taskRoot 'test-results/zpaq-build'
New-Item -ItemType Directory -Force $buildDirectory | Out-Null
$sourceDirectory = Join-Path $taskRoot 'vendor/zpaq'
$outputDirectory = Join-Path $taskRoot 'src-tauri/resources/zpaq-engine'
if ($TestIO) { $outputDirectory = Join-Path $taskRoot 'test-results/zpaq-io-test' }
New-Item -ItemType Directory -Force $outputDirectory | Out-Null
Push-Location $buildDirectory
try {
  $faultDefine = if ($TestIO) { '/DLONG_ZPAQ_TEST_IO' } else { '' }
  $buildCommand = 'call "{0}\VC\Auxiliary\Build\vcvars64.bat" >nul && cl /nologo /O2 /MT /EHsc /D_CRT_SECURE_NO_WARNINGS {3} /Fe:"{1}\long-zpaq.exe" "{2}\zpaq.cpp" "{2}\libzpaq.cpp" advapi32.lib /link /Brepro' -f $compilerRoot,$outputDirectory,$sourceDirectory,$faultDefine
  & cmd.exe /d /s /c $buildCommand
  if ($LASTEXITCODE -ne 0) { throw 'ZPAQ adapter compilation failed' }
} finally { Pop-Location }
