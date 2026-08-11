# Test Files Generator for Long Decompress
# Generates various compressed formats for testing

$ErrorActionPreference = "Stop"
$TestDir = "test-files"
$SourceDir = "$TestDir\source"
$OutputDir = "$TestDir\archives"

Write-Host "=== Long Decompress Test Files Generator ===" -ForegroundColor Cyan

# Clean and create directories
if (Test-Path $TestDir) {
    Remove-Item $TestDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $SourceDir | Out-Null
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

Write-Host "`n[1/4] Creating source files..." -ForegroundColor Yellow

# Create test files with different content
$files = @(
    @{Name="readme.txt"; Content="This is a test file for Long Decompress.`nSupports 37+ formats!`n测试中文字符。"}
    @{Name="data.json"; Content='{"name":"Long Decompress","version":"1.0","formats":["ZIP","7Z","RAR","TAR"]}'}
    @{Name="script.js"; Content="console.log(''Hello from Long Decompress'');`nconst formats = [''ZIP'', ''7Z'', ''RAR''];"}
    @{Name="config.xml"; Content='<?xml version="1.0"?><config><name>Test</name><enabled>true</enabled></config>'}
    @{Name="notes.md"; Content="# Test Document`n`n## Features`n- Multi-format support`n- Password protection`n- Batch operations"}
)

foreach ($file in $files) {
    $path = Join-Path $SourceDir $file.Name
    [System.IO.File]::WriteAllText($path, $file.Content, [System.Text.UTF8Encoding]::new($false))
    Write-Host "  Created: $($file.Name)" -ForegroundColor Green
}

# Create a subfolder with more files
$SubDir = "$SourceDir\documents"
New-Item -ItemType Directory -Force -Path $SubDir | Out-Null
@("doc1.txt", "doc2.txt", "doc3.txt") | ForEach-Object {
    "Sample document content - $_" | Out-File -FilePath "$SubDir\$_" -Encoding utf8
}

Write-Host "`n[2/4] Checking for 7z command..." -ForegroundColor Yellow

# Check if 7z is available
$7zPath = $null
$possiblePaths = @(
    "C:\Program Files\7-Zip\7z.exe",
    "C:\Program Files (x86)\7-Zip\7z.exe",
    "$env:ProgramFiles\7-Zip\7z.exe"
)

foreach ($path in $possiblePaths) {
    if (Test-Path $path) {
        $7zPath = $path
        break
    }
}

if (-not $7zPath) {
    Write-Host "  7-Zip not found. Trying 'where 7z'..." -ForegroundColor Yellow
    $whereResult = where.exe 7z 2>$null
    if ($LASTEXITCODE -eq 0 -and $whereResult) {
        $7zPath = $whereResult[0]
    }
}

if (-not $7zPath) {
    Write-Host "  ERROR: 7z.exe not found. Please install 7-Zip from https://www.7-zip.org/" -ForegroundColor Red
    Write-Host "  Only basic PowerShell compression will be available." -ForegroundColor Yellow
    $use7z = $false
} else {
    Write-Host "  Found: $7zPath" -ForegroundColor Green
    $use7z = $true
}

Write-Host "`n[3/4] Creating compressed archives..." -ForegroundColor Yellow

$testPassword = "test123"

# Helper function to run 7z
function Invoke-7z {
    param($Arguments)
    & $7zPath $Arguments 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "7z command failed with exit code $LASTEXITCODE"
    }
}

# 1. ZIP (PowerShell native)
Write-Host "  Creating ZIP..." -ForegroundColor Gray
Compress-Archive -Path "$SourceDir\*" -DestinationPath "$OutputDir\test.zip" -Force

# 2. ZIP with password (7z)
if ($use7z) {
    Write-Host "  Creating ZIP with password..." -ForegroundColor Gray
    Invoke-7z @("a", "-tzip", "-p$testPassword", "$OutputDir\test-password.zip", "$SourceDir\*")
}

# 3. 7Z format
if ($use7z) {
    Write-Host "  Creating 7Z..." -ForegroundColor Gray
    Invoke-7z @("a", "-t7z", "$OutputDir\test.7z", "$SourceDir\*")

    Write-Host "  Creating 7Z with password..." -ForegroundColor Gray
    Invoke-7z @("a", "-t7z", "-p$testPassword", "-mhe=on", "$OutputDir\test-password.7z", "$SourceDir\*")
}

# 4. TAR
if ($use7z) {
    Write-Host "  Creating TAR..." -ForegroundColor Gray
    Invoke-7z @("a", "-ttar", "$OutputDir\test.tar", "$SourceDir\*")
}

# 5. TAR.GZ
if ($use7z) {
    Write-Host "  Creating TAR.GZ..." -ForegroundColor Gray
    Invoke-7z @("a", "-tgzip", "$OutputDir\test.tar.gz", "$SourceDir\*")
}

# 6. TAR.BZ2
if ($use7z) {
    Write-Host "  Creating TAR.BZ2..." -ForegroundColor Gray
    Invoke-7z @("a", "-tbzip2", "$OutputDir\test.tar.bz2", "$SourceDir\*")
}

# 7. TAR.XZ
if ($use7z) {
    Write-Host "  Creating TAR.XZ..." -ForegroundColor Gray
    Invoke-7z @("a", "-txz", "$OutputDir\test.tar.xz", "$SourceDir\*")
}

# 8. GZ (single file)
if ($use7z) {
    Write-Host "  Creating GZ..." -ForegroundColor Gray
    Invoke-7z @("a", "-tgzip", "$OutputDir\readme.txt.gz", "$SourceDir\readme.txt")
}

# 9. BZ2 (single file)
if ($use7z) {
    Write-Host "  Creating BZ2..." -ForegroundColor Gray
    Invoke-7z @("a", "-tbzip2", "$OutputDir\readme.txt.bz2", "$SourceDir\readme.txt")
}

# 10. XZ (single file)
if ($use7z) {
    Write-Host "  Creating XZ..." -ForegroundColor Gray
    Invoke-7z @("a", "-txz", "$OutputDir\readme.txt.xz", "$SourceDir\readme.txt")
}

# 11. Zstd (if supported)
if ($use7z) {
    Write-Host "  Creating Zstd..." -ForegroundColor Gray
    try {
        Invoke-7z @("a", "-tzstd", "$OutputDir\test.tar.zst", "$SourceDir\*")
    } catch {
        Write-Host "    Zstd not supported by this 7z version" -ForegroundColor DarkYellow
    }
}

# 12. Split ZIP (multi-volume)
if ($use7z) {
    Write-Host "  Creating split ZIP (3 volumes)..." -ForegroundColor Gray
    Invoke-7z @("a", "-tzip", "-v100k", "$OutputDir\test-split.zip", "$SourceDir\*")
}

Write-Host "`n[4/4] Creating test summary..." -ForegroundColor Yellow

# Generate summary file
$summary = @"
# Long Decompress Test Files

Generated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
Test Password: $testPassword

## Source Files Location
$SourceDir

## Archives Created
"@

Get-ChildItem $OutputDir -File | ForEach-Object {
    $size = [math]::Round($_.Length / 1KB, 2)
    $summary += "`n- $($_.Name) ($size KB)"
}

$summary += @"


## Testing Instructions

### Basic Decompression Test
1. Open Long Decompress application
2. Drag any archive from '$OutputDir' to the decompress zone
3. Verify extraction succeeds

### Password-Protected Test
1. Drag 'test-password.zip' or 'test-password.7z'
2. When prompted, enter password: $testPassword
3. Verify extraction succeeds

### Batch Operation Test
1. Select multiple archives (Ctrl+Click)
2. Add to decompress queue
3. Start batch decompression

### Compression Test
1. Go to Compress tab
2. Drag files from '$SourceDir'
3. Select different formats (ZIP, 7Z, TAR.GZ, etc.)
4. Test with and without password
5. Verify compressed files can be extracted

### Split Archive Test
1. Decompress 'test-split.zip.001'
2. Ensure all volumes (.001, .002, .003) are in same folder
3. Verify extraction succeeds

## Format Coverage

✓ ZIP (standard)
✓ ZIP (password)
✓ 7Z (standard)
✓ 7Z (password + header encryption)
✓ TAR
✓ TAR.GZ
✓ TAR.BZ2
✓ TAR.XZ
✓ GZ (single file)
✓ BZ2 (single file)
✓ XZ (single file)
✓ Zstd (if 7z supports)
✓ Split archives

"@

$summary | Out-File "$TestDir\README.md" -Encoding utf8

Write-Host "`n=== Generation Complete ===" -ForegroundColor Green
Write-Host "`nTest files location: $TestDir" -ForegroundColor Cyan
Write-Host "Archives location: $OutputDir" -ForegroundColor Cyan
Write-Host "Test password: $testPassword" -ForegroundColor Yellow
Write-Host "`nRead '$TestDir\README.md' for testing instructions." -ForegroundColor White

# Open the output folder
Start-Process explorer.exe $OutputDir
