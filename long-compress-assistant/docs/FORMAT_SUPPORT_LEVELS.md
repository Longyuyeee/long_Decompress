# 归档格式支持等级（当前权威口径）

更新时间：2026-08-27。此文档定义 README、设置中心和发布说明使用“支持”一词时的证据等级；旧版本发布记录仅描述当时状态，不覆盖本表。

## 等级定义

| 等级 | 用户可见含义 | 必须具备的证据 |
| --- | --- | --- |
| A：完整闭环 | 可以由 Long解压创建并重新解压 | Windows Release Tauri 创建非空归档、重新解压、逐字节或 SHA-256 比对；密码、分卷或外部编码器限制必须单独验证 |
| B：真实只读 | 可以导入、浏览或解压，但不声明可创建 | 固定或可复现的非空样本、已知载荷、Windows Release Tauri 解压、逐字节或 SHA-256 比对 |
| C：引擎可识别 | 当前随包引擎报告了解码器，但尚无完整用户路径证据 | 只允许在动态能力诊断中出现；README 不得写成“完整支持” |
| D：不作为归档导入 | 文档容器或尚未公开的格式 | 解压中心主动过滤，或给出明确不支持诊断 |

后缀识别、文件头识别、空镜像、损坏样本、仅 CLI 成功或仅单元测试均不能把格式提升到 A/B。

## 当前等级

### A：完整创建—解压闭环

- ZIP、7Z、WIM；
- TAR、TAR.GZ、TAR.BZ2、TAR.XZ、TAR.ZST；
- GZ、BZ2、XZ、ZST/ZSTD、LZMA；
- 加密 ZIP、加密 7Z；
- TAR.AES、TAR.GZ.AES、TAR.BZ2.AES、TAR.XZ.AES、TAR.ZST.AES；
- GZ.AES、BZ2.AES、XZ.AES、ZST.AES。

RAR 创建属于条件能力：必须使用用户自行安装并授权的 WinRAR/Rar.exe；无编码器时只能保持明确失败或由用户主动改用 7Z。RAR 解压属于 B 级，并有固定加密 RAR 的正确/错误密码门禁。

### B：非空真实载荷只读闭环

严格桌面矩阵已覆盖应用包、传统归档、镜像、虚拟磁盘、文件系统、Windows Installer 和固件，包括 JAR/XPI/IPA/APK/APPX、CAB、AR/A、ISO、CPIO、XAR、RAR5、LHA/LZH、RPM、DEB/UDEB、DMG/HFS+、EXT2/3/4、QCOW/QCOW2、VDI、VMDK、VHD/VHDX、FAT、NTFS、SquashFS/SFS、APFS、MSI/MSM/MSP、NSIS、GPT/MBR、UEFI、CramFS、IHex、HFS 和 HFSX。

HFSX 的当前证据不是空镜像：脚本固定 `mozilla/libdmg-hfsplus@ec239599c1f234a4e01ae3fe51214d0c77e5baa3` 和上游空镜像 SHA-256，写入 `Firefox/known-payload.txt`，转换为 HFSX 后由随包 7-Zip及 Release Tauri 分别解压。已知载荷 SHA-256 为 `0A7130487543AF627E9C15512AE6DBE0A6FD9D6ED5F4C2C89942E56CB3B14023`。

### C：仅动态能力

随包 7-Zip 运行时报告、但未出现在当前严格真实矩阵中的处理器或后缀均属于 C。它们可以用于诊断和后续样本准备，不应自动进入公开导入白名单。

### D：不作为归档导入

DOCX、XLSX、PPTX、ODT、ODS 和 EPUB 虽然内部通常使用 ZIP 容器，但属于文档格式，不在解压中心作为普通压缩包导入。此前从公开白名单移除且尚未补齐非空桌面闭环的格式同样保持 D。

## 复验命令

```powershell
npm.cmd run test:fixtures:hfsx
$env:VITE_DESKTOP_E2E = "1"
npm.cmd run build
cargo build --release --manifest-path src-tauri/Cargo.toml --features custom-protocol,desktop-e2e
$env:EDGE_DRIVER_PATH = "C:\path\to\matching\msedgedriver.exe"
npm.cmd run test:e2e:desktop:hfsx
```

完整发布矩阵仍使用：

```powershell
npm.cmd run test:prepare:full-format
npm.cmd run test:e2e:desktop:full-format
```

详细样本来源、固定版本和哈希见 [FULL_FORMAT_REAL_WORLD_VALIDATION.md](FULL_FORMAT_REAL_WORLD_VALIDATION.md)。
