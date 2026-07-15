$ErrorActionPreference = "Stop"
$pwd = "test123"

# Clean and create directories
if (Test-Path "test-files") {
    Remove-Item "test-files" -Recurse -Force
}
New-Item -ItemType Directory -Force -Path "test-files\source\subfolder" | Out-Null
New-Item -ItemType Directory -Force -Path "test-files\archives" | Out-Null

# Create test files
"Long Decompress Test File - 胧解压测试文件" | Out-File "test-files\source\readme.txt" -Encoding utf8
'{"app":"Long Decompress","version":"1.0"}' | Out-File "test-files\source\data.json" -Encoding utf8
"console.log('test');" | Out-File "test-files\source\script.js" -Encoding utf8
"Nested file content" | Out-File "test-files\source\subfolder\nested.txt" -Encoding utf8

Write-Host "=== Creating Test Archives ===" -ForegroundColor Cyan

# 1. ZIP (no password)
Write-Host "[1] ZIP..." -ForegroundColor Green
Compress-Archive -Path "test-files\source\*" -DestinationPath "test-files\archives\test.zip" -Force

# 2. TAR
Write-Host "[2] TAR..." -ForegroundColor Green
tar -cf "test-files\archives\test.tar" -C "test-files\source" .

# 3. TAR.GZ
Write-Host "[3] TAR.GZ..." -ForegroundColor Green
tar -czf "test-files\archives\test.tar.gz" -C "test-files\source" .

# 4. TAR.BZ2
Write-Host "[4] TAR.BZ2..." -ForegroundColor Green
tar -cjf "test-files\archives\test.tar.bz2" -C "test-files\source" .

# Find 7-Zip
$7z = $null
$paths = @(
    "C:\Program Files\7-Zip\7z.exe",
    "C:\Program Files (x86)\7-Zip\7z.exe"
)
foreach ($p in $paths) {
    if (Test-Path $p) {
        $7z = $p
        break
    }
}

if ($7z) {
    Write-Host "`nFound 7-Zip: $7z`n" -ForegroundColor Yellow

    # 5. 7Z (no password)
    Write-Host "[5] 7Z..." -ForegroundColor Green
    & $7z a -t7z "test-files\archives\test.7z" "test-files\source\*" -r | Out-Null

    # 6. 7Z with password
    Write-Host "[6] 7Z with password ($pwd)..." -ForegroundColor Green
    & $7z a -t7z -p"$pwd" -mhe=on "test-files\archives\test-password.7z" "test-files\source\*" -r | Out-Null

    # 7. ZIP with password
    Write-Host "[7] ZIP with password ($pwd)..." -ForegroundColor Green
    & $7z a -tzip -p"$pwd" "test-files\archives\test-password.zip" "test-files\source\*" -r | Out-Null

    # 8. TAR.XZ
    Write-Host "[8] TAR.XZ..." -ForegroundColor Green
    & $7z a -ttar "test-files\archives\temp.tar" "test-files\source\*" -r | Out-Null
    & $7z a -txz "test-files\archives\test.tar.xz" "test-files\archives\temp.tar" | Out-Null
    Remove-Item "test-files\archives\temp.tar" -Force

    # 9. GZ (single file)
    Write-Host "[9] GZ (single file)..." -ForegroundColor Green
    & $7z a -tgzip "test-files\archives\readme.txt.gz" "test-files\source\readme.txt" | Out-Null

    # 10. BZ2 (single file)
    Write-Host "[10] BZ2 (single file)..." -ForegroundColor Green
    & $7z a -tbzip2 "test-files\archives\readme.txt.bz2" "test-files\source\readme.txt" | Out-Null

    # 11. XZ (single file)
    Write-Host "[11] XZ (single file)..." -ForegroundColor Green
    & $7z a -txz "test-files\archives\readme.txt.xz" "test-files\source\readme.txt" | Out-Null

} else {
    Write-Host "`n7-Zip not found. Only basic formats created.`n" -ForegroundColor Yellow
}

# Find WinRAR
$rar = "C:\Program Files\WinRAR\WinRAR.exe"
if (Test-Path $rar) {
    Write-Host "[12] RAR..." -ForegroundColor Green
    & $rar a -ep1 -r "test-files\archives\test.rar" "test-files\source\*" | Out-Null

    Write-Host "[13] RAR with password ($pwd)..." -ForegroundColor Green
    & $rar a -ep1 -r -p"$pwd" "test-files\archives\test-password.rar" "test-files\source\*" | Out-Null
}

Write-Host "`n=== Summary ===" -ForegroundColor Cyan
Write-Host "Location: test-files\archives`n" -ForegroundColor Yellow

$archives = Get-ChildItem "test-files\archives" | Sort-Object Extension, Name
foreach ($a in $archives) {
    $size = [math]::Round($a.Length / 1KB, 1)
    $encrypted = if ($a.Name -match "password") { " [PASSWORD: $pwd]" } else { "" }
    Write-Host "  $($a.Name.PadRight(30)) $($size.ToString().PadLeft(6)) KB$encrypted" -ForegroundColor White
}

Write-Host "`nPassword for encrypted files: $pwd" -ForegroundColor Yellow
Start-Process explorer.exe "test-files\archives"
