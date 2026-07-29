# 全格式真实环境验证

本文档定义 Long解压 的格式验收口径。格式不能仅凭界面可选、函数返回成功或模拟测试判定为可用；可创建格式必须在 Windows Tauri 正式后端完成真实压缩与解压，并逐字节比对源文件和输出文件。

## 2026-07-29 验收结果

测试环境为 Windows 11、WebView2、Release Tauri 二进制和项目随包提供的完整 7-Zip 引擎。测试使用独立数据目录与系统临时目录，不读取或修改正式版密码保险箱和用户设置。

### 可创建格式

以下 25 个场景已经完成“压缩 → 解压 → 逐字节比对”：

- ZIP、7Z、WIM
- TAR、TAR.GZ、TAR.BZ2、TAR.XZ、TAR.ZST
- GZ、BZ2、XZ、ZST、ZSTD 别名、LZMA
- 带密码的 ZIP、带密码的 7Z
- TAR.AES、TAR.GZ.AES、TAR.BZ2.AES、TAR.XZ.AES、TAR.ZST.AES
- GZ.AES、BZ2.AES、XZ.AES、ZST.AES

测试脚本会读取 `FORMAT_CAPABILITIES` 中声明的可创建格式并与桌面矩阵对账。以后新增格式但没有真实场景时，测试会直接失败。

RAR 创建依赖用户另外安装的 WinRAR `Rar.exe`。当前机器没有该编码器，因此验收的是依赖缺失路径：操作必须失败、错误必须明确指出 WinRAR/Rar.exe 缺失，并且不得遗留伪成功的 `.rar` 文件。RAR 解压仍由随包 7-Zip 引擎支持。

### 只读格式

只读格式由随包完整 7-Zip 引擎统一解码。当前桌面测试使用真实生成的样本完成下列逐字节验证：

- 应用包：JAR、XPI、IPA、APK、APPX
- Windows 包：CAB
- 传统归档：AR

引擎能力测试同时确认随包版本提供 APFS、AR、EXT、QCOW、VDI、VMDK、WIM 等动态处理器。ISO、DMG、VHD/VHDX、QCOW、VDI、VMDK、RPM、MSI、文件系统和固件镜像无法由 Long解压或 Windows 基础组件安全反向生成，因此它们的逐文件验收需要可信的非空样本库。没有真实样本时，不得把后缀识别测试写成“完整解压通过”。

## 用户操作路径

除自动化桌面闭环外，正式安装版还按真实用户路径进行了检查：

1. 进入压缩中心；
2. 通过 Windows 原生文件选择器导入真实文件；
3. 确认条目显示压缩包名称、源文件路径、压缩状态与进度；
4. 展开条目，确认左侧为配置详情，右侧为阶段、进度和实时执行日志；
5. 确认“使用同名压缩包”入口可见。

实际压缩运算在隔离 Release Tauri 中完成，避免在用户项目目录留下测试压缩包。

## 运行方式

```powershell
$env:VITE_DESKTOP_E2E = "1"
npm.cmd run build
cargo build --release --manifest-path src-tauri/Cargo.toml --features custom-protocol,desktop-e2e
$env:EDGE_DRIVER_PATH = "C:\path\to\matching\msedgedriver.exe"
npm.cmd run test:e2e:desktop
```

成功结果必须包含：

- 所有可创建格式的真实归档闭环；
- 真实 7Z 中间进度与取消；
- 只读真实样本解压；
- RAR 无编码器的明确失败路径；
- 取消后残留清理、退出确认、更新阻塞、托盘与第二实例恢复。

## 下一阶段

建立经过授权、可再分发、带 SHA-256 的只读样本库，逐步覆盖 ISO、DMG、虚拟磁盘、Linux 软件包、文件系统和固件镜像。每个样本必须至少包含一个已知文件及其哈希；测试完成后逐字节核验，损坏样本和空壳文件不计为通过。

## 2026-07-29 第二阶段增量

新增并通过以下真实解压场景：

- Windows `tar.exe` 离线生成：ISO9660、XAR、CPIO；
- 手工构造标准 Debian AR 容器，并继续解压其中的 `data.tar` 核验最终载荷；
- WSL `mke2fs` 离线生成：EXT2、EXT3、EXT4，每个镜像均包含已知文件；
- libarchive 官方测试仓库固定提交样本：RAR5、LHA/LZH、RPM。

上游样本锁定提交 `19ff56da4f4790064346579a1a7f18a0230b0ac6`。下载器同时校验 UUencoded 源文件 SHA-256 与解码后归档 SHA-256；桌面解压完成后再次校验已知输出文件 SHA-256。缓存位于忽略提交的 `test-results/external-archive-fixtures`。

正式安装版还通过 Windows 原生文件选择器导入固定哈希 RAR5 样本，点击“开始解压队列”后界面从“等待中 / 0%”变为“已完成 / 100%”。输出 `helloworld.txt` 的 SHA-256 为 `FEF9AD8CF601B43F76C6320075F62267C6E5C0A526D750A70B80C919A4A0AAD8`，与上游已知值一致。

新增样本准备命令：

```powershell
npm.cmd run test:fixtures:archives
```

尚待真实载荷样本覆盖：DMG、VHD/VHDX、QCOW/QCOW2、VDI、VMDK、APFS、FAT、NTFS、HFS/HFSX、SquashFS、固件镜像以及 MSI/MSP/MSM。它们仍只能记录为“随包引擎声明具备处理器”，不能标记为逐文件验收完成。
