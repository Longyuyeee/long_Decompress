# 测试所有格式的压缩解压功能
# 创建测试目录和文件

$ErrorActionPreference = "Stop"
$testRoot = "E:\Project\AIProject\GeminiProject\long_Decompress\format-test"

# 清理旧测试目录
if (Test-Path $testRoot) {
    Remove-Item -Recurse -Force $testRoot
}

# 创建测试结构
New-Item -ItemType Directory -Force -Path "$testRoot\source" | Out-Null
New-Item -ItemType Directory -Force -Path "$testRoot\archives" | Out-Null
New-Item -ItemType Directory -Force -Path "$testRoot\extracted" | Out-Null

# 创建测试文件
@"
This is a test file for compression and decompression.
Testing all supported formats.
"@ | Out-File -FilePath "$testRoot\source\test.txt" -Encoding UTF8

"Binary data test" | Out-File -FilePath "$testRoot\source\data.bin" -Encoding UTF8

New-Item -ItemType Directory -Force -Path "$testRoot\source\subfolder" | Out-Null
"Nested file" | Out-File -FilePath "$testRoot\source\subfolder\nested.txt" -Encoding UTF8

Write-Host "✓ 测试文件创建完成" -ForegroundColor Green

# 测试格式列表
$formats = @(
    @{Name="ZIP"; Ext=".zip"; NeedsCLI=$false},
    @{Name="7Z"; Ext=".7z"; NeedsCLI=$false},
    @{Name="TAR"; Ext=".tar"; NeedsCLI=$false},
    @{Name="TAR.GZ"; Ext=".tar.gz"; NeedsCLI=$false},
    @{Name="TAR.BZ2"; Ext=".tar.bz2"; NeedsCLI=$false},
    @{Name="TAR.XZ"; Ext=".tar.xz"; NeedsCLI=$false},
    @{Name="GZ"; Ext=".gz"; NeedsCLI=$false},
    @{Name="BZ2"; Ext=".bz2"; NeedsCLI=$false},
    @{Name="XZ"; Ext=".xz"; NeedsCLI=$false}
)

Write-Host "`n开始格式测试..." -ForegroundColor Cyan
Write-Host "=" * 60

$results = @()

foreach ($format in $formats) {
    $name = $format.Name
    $ext = $format.Ext
    $archivePath = "$testRoot\archives\test$ext"

    Write-Host "`n测试格式: $name" -ForegroundColor Yellow

    $result = @{
        Format = $name
        Compression = "未测试"
        Decompression = "未测试"
        Error = ""
    }

    try {
        # 注意: 这个脚本只创建测试结构
        # 实际的压缩解压需要在应用中手动测试或通过Tauri命令测试
        Write-Host "  → 需要在应用中测试此格式" -ForegroundColor Gray
        $result.Compression = "需要手动测试"
        $result.Decompression = "需要手动测试"
    }
    catch {
        $result.Error = $_.Exception.Message
        Write-Host "  ✗ 错误: $($_.Exception.Message)" -ForegroundColor Red
    }

    $results += $result
}

Write-Host "`n" + ("=" * 60)
Write-Host "测试摘要" -ForegroundColor Cyan
Write-Host ("=" * 60)

$results | Format-Table -AutoSize

Write-Host "`n测试目录: $testRoot" -ForegroundColor Green
Write-Host "请使用应用界面进行实际的压缩解压测试" -ForegroundColor Yellow
