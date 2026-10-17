# Info-ZIP 旧编码密码原包

2026-10-17 在 Windows 11 上由 Info-ZIP Zip 3.0 实际创建，载荷是本项目自行编写的 `payload.txt`。压缩源码未修改，ZIP 头及密文未手工改写；只给生成器副本嵌入进程代码页清单。三个原始 ZIP 随仓库保存，换机器可直接复用。

| 原包 | 进程代码页 | 自有测试密码 |
| --- | --- | --- |
| `cp932.zip` | 日文 932 | `日本語パスワード+!1416` |
| `cp936.zip` | 简体中文 936 | `简体密码测试+!1416` |
| `cp950.zip` | 繁体中文 950 | `繁體密碼測試+!1416` |

每包仅一个非空 Deflate + ZipCrypto 文件，原文 359 字节。使用 `-X -j` 不携带机器的 NTFS 安全描述符/路径；不覆盖密码 AES、分卷、乱码文件名或任意旧工具。完整原卷与载荷 SHA256、生成器身份和独立回解结果见 `manifest.json`。

## 来源与生成方式

[Info-ZIP 官方项目](https://infozip.sourceforge.net/Zip.html)链接到 [Zip 3.0 源码](https://sourceforge.net/projects/infozip/files/Zip%203.x%20%28latest%29/3.0/zip30.zip/download)。下载的 `zip30.zip` 为 1,287,223 字节，SHA256 `7061ceac0407682b6dc54bb480347205f680f4e56cf34fe1423df2309f18968a`。本地路径为忽略目录 `test-results/zip-encoding-1416/tools/infozip-source/zip30`，上游源码不随产品分发。

在 VS 2022 Developer PowerShell 的源码目录，使用上游 makefile：

```powershell
nmake /nologo /f win32/makefile.w32 NOASM=1 CRTLIB=-MT 'LDFLAGS=user32.lib advapi32.lib /INCREMENTAL:NO' 'LOCAL_ZIP=-D_CRT_SECURE_NO_WARNINGS -D_CRT_NONSTDC_NO_DEPRECATE -FIwindows.h' zip.exe
```

MSVC 19.44 x64 / Windows SDK 10.0.26100.0。`-FIwindows.h` 让 SDK 先于老源码的 `CR` 宏加载，解决与新版 ARM64 头字段冲突；不改压缩实现。上游版本输出固定写“32-bit”，不能据此当作实际编译架构。源码原文件逐一与下载 ZIP 核对，编译器的既有尺寸转换警告保留。

给两个测试副本分别嵌入 `generator/ja-JP.manifest` 和 `generator/zh-TW.manifest`，使用 SDK 的 `mt.exe -manifest <文件> '-outputresource:<副本.exe>;#1'`。这是微软支持的 [Windows 11 进程代码页机制](https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests#activecodepage)，没有修改系统语言、区域设置或本应用清单。未加清单的对照进程为 ACP/OEMCP 936，两个带清单进程分别为 932/950；`generator/codepage-probe.c` 实测参数字节与独立编码器一致。

从应用目录向新的输出位置执行（勿覆盖本目录已保存原卷）：

```powershell
& <日文副本.exe> -X -j -P '日本語パスワード+!1416' <新目录/cp932.zip> tests/fixtures/archives/legacy-zip-password-1416/payload.txt
& <未改清单副本.exe> -X -j -P '简体密码测试+!1416' <新目录/cp936.zip> tests/fixtures/archives/legacy-zip-password-1416/payload.txt
& <繁体副本.exe> -X -j -P '繁體密碼測試+!1416' <新目录/cp950.zip> tests/fixtures/archives/legacy-zip-password-1416/payload.txt
```

ZipCrypto 随机盐和编译日期会使重新生成的归档/程序哈希不同，不能替换原卷后继续套用旧哈希。载荷及编码密码字节应保持。测试密码是公开的自有资料，不是用户密码。

## 独立验证

运行 `python scripts/verify-legacy-zip-password-fixtures.py`。Python 标准库 `zipfile` 使用正确编码的密码完整读取、检查 CRC，并与载荷逐字节对比；UTF-8、其他可表示编码和错误密码均拒绝，不可表示时不替换字符。

应用基线仅支持 UTF-8/本机 ACP。原始 Rust 用例先用固定已知字节验证包有效，再确认默认入口的本机编码边界；它不代表新编码已经接入所有用户页面。
