$ErrorActionPreference = "Stop"

# Create directories
New-Item -ItemType Directory -Force -Path "test-files\source" | Out-Null
New-Item -ItemType Directory -Force -Path "test-files\archives" | Out-Null

# Create test files
"Test content for Long Decompress" | Out-File "test-files\source\test1.txt" -Encoding utf8
"Another test file" | Out-File "test-files\source\test2.txt" -Encoding utf8
'{"test":"data"}' | Out-File "test-files\source\data.json" -Encoding utf8

New-Item -ItemType Directory -Force -Path "test-files\source\subfolder" | Out-Null
"Subfolder content" | Out-File "test-files\source\subfolder\nested.txt" -Encoding utf8

Write-Host "Creating ZIP..." -ForegroundColor Green
Compress-Archive -Path "test-files\source\*" -DestinationPath "test-files\archives\test.zip" -Force

# Find 7z
$7z = "C:\Program Files\7-Zip\7z.exe"
if (-not (Test-Path $7z)) {
    $7z = "C:\Program Files (x86)\7-Zip\7z.exe"
}

if (Test-Path $7z) {
    Write-Host "Creating 7Z..." -ForegroundColor Green
    & $7z a -t7z "test-files\archives\test.7z" "test-files\source\*" | Out-Null

    Write-Host "Creating ZIP with password (test123)..." -ForegroundColor Green
    & $7z a -tzip -ptest123 "test-files\archives\test-password.zip" "test-files\source\*" | Out-Null

    Write-Host "Creating 7Z with password (test123)..." -ForegroundColor Green
    & $7z a -t7z -ptest123 -mhe=on "test-files\archives\test-password.7z" "test-files\source\*" | Out-Null

    Write-Host "Creating TAR.GZ..." -ForegroundColor Green
    & $7z a -ttar "test-files\archives\temp.tar" "test-files\source\*" | Out-Null
    & $7z a -tgzip "test-files\archives\test.tar.gz" "test-files\archives\temp.tar" | Out-Null
    Remove-Item "test-files\archives\temp.tar" -Force

    Write-Host "Creating TAR.BZ2..." -ForegroundColor Green
    & $7z a -ttar "test-files\archives\temp.tar" "test-files\source\*" | Out-Null
    & $7z a -tbzip2 "test-files\archives\test.tar.bz2" "test-files\archives\temp.tar" | Out-Null
    Remove-Item "test-files\archives\temp.tar" -Force

    Write-Host "Creating TAR.XZ..." -ForegroundColor Green
    & $7z a -ttar "test-files\archives\temp.tar" "test-files\source\*" | Out-Null
    & $7z a -txz "test-files\archives\test.tar.xz" "test-files\archives\temp.tar" | Out-Null
    Remove-Item "test-files\archives\temp.tar" -Force
} else {
    Write-Host "7-Zip not found. Only ZIP created." -ForegroundColor Yellow
}

Write-Host "`nTest files created in: test-files\archives" -ForegroundColor Cyan
Write-Host "Password for encrypted files: test123" -ForegroundColor Yellow

Start-Process explorer.exe "test-files\archives"
