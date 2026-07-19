# 胧解压 · 方便助手

面向 Windows 的本地压缩与解压工具，使用 Vue 3、TypeScript、Tauri 和 Rust 构建。支持批量任务、密码本、智能密码尝试、托盘运行、资源管理器右键菜单与一键解压。

## v1.0.7 更新

- 修复所有格式共用的加密状态误判：未加密归档不会再触发字典攻击或显示“密码破解成功”。
- 解压流程加入事务式暂存、原子提交与失败回滚，并统一冲突策略、文件筛选、扁平化及“仅解压较新文件”语义。
- 增加磁盘空间、文件数量、展开体积、压缩比、符号链接和 Windows 重解析点安全限制。
- 标准分卷 ZIP 使用 `.zip.001` 结构；压缩输出、TAR AES 与旧式 AES 路径补齐原子写入和资源限制。
- 外部引擎与密码字典支持可靠取消；RAR 密码创建增加进程参数风险确认，后台命令继续保持无控制台窗口。
- 真实归档矩阵、后端全目标测试、前端单元与端到端测试、类型检查、生产构建及 Clippy 全部通过。

## v1.0.6 更新

- 内置完整 7-Zip 26.02 引擎，并根据运行时能力动态开放格式。
- 新增 WIM 创建，以及 QCOW2、VDI、VMDK、APFS、EXT、GPT、AR、PPKG 等首批扩展格式读取。
- 创建 RAR 缺少编码器时，可选择改用 7Z、通过 winget 安装 WinRAR、打开官方下载页或取消；已安装 WinRAR 时支持密码 RAR 创建。
- 设置中心新增引擎版本、读取/创建格式数量、RAR 编码器状态和重新检测入口。
- 新增真实 7Z、ZIP、TAR、WIM、密码 7Z 与 AR 样本回归矩阵。

## 归档能力

应用内置完整的 7-Zip 26.02 Windows x64 引擎，并在运行时读取引擎实际报告的格式能力。需要完整引擎的格式只会在当前环境确实可用时出现，避免界面宣称支持、实际机器却无法处理。

设置中心会显示当前引擎版本、可读取后缀数、可创建格式数和 RAR 编码器状态，并支持手动重新检测。

### 创建归档

- 常用格式：ZIP、7Z、TAR、TGZ、TBZ、TXZ、TZST
- 单文件流格式：GZ、BZ2、XZ、Zstd、LZMA
- Windows 映像：WIM（由动态能力检测决定是否显示）
- 密码格式：加密 ZIP、7Z、RAR，以及 TAR/GZ/BZ2/XZ/Zstd 的 `.aes` 安全封装
- RAR：必须使用用户安装的 WinRAR/Rar.exe；支持普通与密码 RAR，不随应用分发专有编码器

创建密码 RAR 前会明确提示：受 WinRAR 命令行接口限制，密码可能在执行期间短暂出现在本机进程参数中。用户可以继续创建、改用 7Z 或取消任务。

### 解压与浏览

除上述格式外，第一批完整引擎扩展覆盖：

- 镜像与虚拟磁盘：ISO、IMG、DMG、VHD/VHDX、QCOW/QCOW2、VDI、VMDK
- 安装包与容器：CAB、DEB、RPM、MSI/MSP/MSM、NSIS、PPKG
- 文件系统与固件：APFS、EXT2/3/4、FAT、NTFS、HFS/HFSX、GPT、MBR、UEFI、UDF、CramFS
- 传统归档：AR/A、ARJ、LZH/LHA、CHM、CPIO、SquashFS、XAR、Unix Z
- ZIP 容器：JAR、EPUB、APK、APPX、DOCX、XLSX、PPTX 等

具体能力以当前引擎动态检测结果为准。某些镜像或文件系统格式只支持读取和解压，不支持创建。

## RAR 创建策略

选择 RAR 后，如果系统没有可用的 Rar.exe，应用会暂停当前任务并提供以下选择：

1. 将本次 RAR 任务改为 7Z，并继续原任务；
2. 通过 winget 安装官方 WinRAR，安装完成后重新检测并继续；
3. 打开 WinRAR 官方下载页；
4. 取消本次任务。

应用不会静默安装第三方软件，也不会拆分或转售 RAR 编码器。WinRAR 的试用期与商业许可由 RARLAB 管理。

## 真实样本回归矩阵

Rust 集成测试会调用实际随包引擎，而不是 mock：

| 场景 | 创建 | 解压/验证 | 内容比对 |
| --- | --- | --- | --- |
| 7Z | 是 | 是 | 是 |
| ZIP | 是 | 是 | 是 |
| TAR | 是 | 是 | 是 |
| WIM | 是 | 是 | 是 |
| 密码 7Z | 是 | 密码测试 | 是 |
| AR | 手工生成真实样本 | 是 | 是 |

运行矩阵：

```powershell
cd src-tauri
cargo test --test archive_engine_matrix_test
```

## 开发与验证

环境要求：Node.js、npm、稳定版 Rust 与 Windows WebView2。

```powershell
npm install
npm run dev
npm run type-check
npm run test:unit
npm run build
cd src-tauri
cargo test
```

Tauri 开发模式：

```powershell
npm run tauri dev
```

## 项目结构

```text
src/                         Vue 前端与交互
src-tauri/src/               Rust 后端与 Tauri 命令
src-tauri/resources/         随包归档引擎及第三方许可
src-tauri/tests/             后端能力与真实样本回归
scripts/                     构建和系统集成脚本
tests/                       前端集成与性能测试
```

## 许可证

项目代码使用 MIT License。随包 7-Zip 组件适用其自身的 LGPL/BSD 与 unRAR restriction 条款，原始许可和说明位于 `src-tauri/resources/archive-engine/`。WinRAR 不包含在本项目中。
