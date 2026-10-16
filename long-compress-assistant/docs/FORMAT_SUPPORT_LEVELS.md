# 归档格式支持等级（当前权威口径）

更新时间：2026-10-16（当前正式版 v1.4.11）。此文档定义 README、设置中心和发布说明使用“支持”一词时的证据等级。下载见 [v1.4.11 正式 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.11)，历次能力按对应实现和实际产物记录。

## 当前边界速查（覆盖下方历次演变描述）

- v1.4.12 候选新增 LZIP v1 / TAR.LZIP 的内置创建、浏览、解压、检查与转换，兼容 `.lz`、`.tar.lz`、`.tlz`，读取连续多成员 LZIP；不提供原生密码、分卷创建或损坏数据恢复。自有载荷由独立 GNU lzip 生成并由应用读取，应用创建/转换另经 GNU lzip / tar 回解；准确页面及安装包状态见 [本版记录](LZIP_AND_TAR_LZIP_1.4.12.md)。正式下载仍为 v1.4.11。

- v1.4.11 新增 ZIP/7Z/RAR SFX 只读识别、浏览、密码、解压、检查及转换，9 组外部单 EXE 样本与 6 组错密码通过；不创建或执行输入 EXE。好压论坛 ZIP 的公开明文原卷及 GUI 原生 ZipCrypto 三卷通过任意成员读取和完整回解，错误密码纠正、独立目录、转换及补卷重试均通过实际桌面链；不扩大到好压 AES/7Z 论坛卷、全部 RAR4 SFX 或外置数据卷。实现和来源见 [本版审计](SFX_AND_VOLUME_COMPATIBILITY_1.4.11.md)，原生加密卷现已补齐，下方旧“缺样本”文字为历史状态。
- v1.4.10 ZIP 的所选条目、筛选、密码判断、候选校验、容量与读取统一；未选的不同密码/未知方法不阻挡本次任务。新增原生 Shrink（1）、Reduce factor 4（5）、Implode（6）固定只读样本，不扩大全部旧工具密码承诺。Zstandard 的 AES/ZipCrypto 各覆盖单包、数字卷和标准卷；真实 4,294,979,641 字节文件的 AES/ZipCrypto ZIP64 数字卷已独立创建、全量验证及应用回解，不代表所有 ZIP64 密码/卷型均完成。来源、结果与限制见 [本版审计](ARCHIVE_COMPATIBILITY_1.4.10.md)。下方“未有加密 Zstandard 样本”等是早期状态。
- ZIP 完整标准卷组丢目录、普通流式 Deflate 描述符及边界明确的 AES/ZipCrypto 条目救援已随 v1.4.8 发布；任意缺卷、未知加密边界仍不支持。下方旧版本的“不支持标准磁盘卷丢目录”只表示当时范围。
- RAR5 恢复记录及 `.rev` 修复、恢复卷创建/补充生成已发布，需要本机 Rar.exe。RAR4 恢复没有独立完整证据，不扩大承诺。
- v1.4.9 不新增格式或扩大上述恢复范围。本版修正 RAR 管道输入为工具使用的 **ANSI** 而非 OEM 字符集；不可表示的字符仍拒绝。另改善大文件校验取消与批量工作区接续，见 [实施记录](ARCHIVE_CONTINUITY_1.4.9.md)。

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
- LZ4、Brotli、TAR.LZ4、TAR.BR（v1.4.3 新增，真实 Windows Release Tauri 逐字节回解通过）；
- 加密 ZIP、加密 7Z；
- TAR.AES、TAR.GZ.AES、TAR.BZ2.AES、TAR.XZ.AES、TAR.ZST.AES；
- GZ.AES、BZ2.AES、XZ.AES、ZST.AES。

RAR 创建属于条件能力：必须使用用户自行安装并授权的 WinRAR/Rar.exe；v1.4.3 接通内容/文件名密码及原生分卷，转换—回解—合卷通过真实 Windows Release Tauri 载荷对比；无编码器时只能保持明确失败或由用户主动改用 7Z。RAR 解压属于 B 级，并有固定加密 RAR 的正确/错误密码门禁。

v1.4.7 增加条件恢复能力：本机 Rar.exe 在副本中修复恢复记录或使用已有 `.rev` 重建卷组，组装完好卷后逐条读取校验，结果保存到新的 `.repaired` 目录。新增实测范围为 RAR5，覆盖普通/内容密码/文件名密码包、损坏分卷及恢复卷补齐首/中/末卷。恢复保护可选 3/5/10%，默认关闭；不创建 `.rev`，不将未实测 RAR4 恢复升级为完整能力。密码字符能否经 RAR 控制台传递取决于本机 OEM 编码；不改变既有 RAR 创建、读取及密码权限口径。

本轮新增格式及卷组证据见 [v1.4.3 收口记录](ARCHIVE_UPGRADE_FREEZE_1.4.3.md)。普通 TAR/TGZ/TBZ/TXZ/TZST 与 GZ/BZ2/XZ/ZST、LZ4/Brotli 的数字切卷可由应用整组回解；它们不是上述格式的原生卷标准。普通 TAR 和流格式没有原生密码，7Z 容器和本应用 AES 包装的兼容范围单独说明。

v1.4.4 增加标准 ZIP `.z01/…/.zip` 真正创建闭环，普通/AES/ZipCrypto 桌面创建与独立回解已验证；旧默认仍是数字切卷。ZIP/ZIPX 的 PPMd 等常用 7 方法 × 3 加密模式已实测，Zstandard 另有无密码样本，不宣称所有 ZIPX 专有方法、加密 Zstandard 或全部 >4 GiB 密码组合都已覆盖。真实 >4 GiB 无密码 ZIP64、RAR4/5 新旧卷型与 7Z 固实/加密组合见 [本版审计](ARCHIVE_RELIABILITY_1.4.4.md)。

ZIP 恢复为独立能力：普通 ZIP/完整数字卷的可信本地头目录重建，以及目录仍可读的普通标准卷完整条目救援。1.4.5进一步接通目录可读的 AES/ZipCrypto ZIP（含 ZIPX、混合包、完整数字卷和标准卷），用户提供密码后只救回完整文件，新包沿用填写的密码并使用 AES-256 / UTF-8。必须至少有一个完整加密文件通过校验，混合包的明文文件不能证明密码正确。原包总数不明或存在未恢复项时明确部分恢复；目录丢失的密码包能力见下段，缺卷和未知边界/数据描述符猜测恢复仍不支持。这不是新建一种容器。

1.4.6进一步覆盖中央目录丢失、但本地条目边界明确的 AES/ZipCrypto ZIP 及完整数字卷救援。新包沿用填写的密码，显示原包总数无法确认和部分恢复；本地尺寸未知的数据描述符、缺卷、标准磁盘 ZIP 卷丢目录仍不支持。本轮实际定向证据及封板状态见 [实施记录](FAILURE_RECOVERY_AND_LOST_ZIP_1.4.6.md)；源码完成不等于新安装包已发布。

### B：非空真实载荷只读闭环

v1.4.8 增加救援与保护能力：完整标准 ZIP 卷组可在目录丢失时重建边界可靠的完整条目，贯通普通与 AES/ZipCrypto 读取；未加密流式 Deflate 的结束位置与普通/ZIP64 数据描述符一致时也可救援。密码条目的未知边界、缺卷与不支持方法仍不恢复。RAR 分卷可附带或补充创建 `.rev`，新增以 RAR5 为实测范围，需本机 Rar.exe。当前实施及证据见 [本版记录](ARCHIVE_RECOVERY_1.4.8.md)，不把源码准备当作正式发布状态。

1.4.5新增 **ARJ、Unix `.Z`、UDF**：真实 Windows Release Tauri 浏览、解压、转换 ZIP 后回解均逐字节通过。ARJ 有存储及方法 1～4 的固定样本；UDF 有包含目录和实际文件的 1.02 镜像；Unix `.Z` 是单文件压缩流，不提供展开大小时页面显示未知。固定来源/哈希及证据见 [本版记录](PASSWORD_ZIP_RESCUE_AND_FORMATS_1.4.5.md)。不宣称创建、原生密码/分卷、所有 UDF 变体或任意 ARJ 扩展特性。

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
