<div align="center">

<img src="icon.png" width="112" alt="胧解压图标">

# 胧解压·方便助手

一款面向 Windows 的现代化压缩、解压与归档管理工具。

[![Version](https://img.shields.io/badge/version-1.0.8-0ea5e9?style=flat-square)](https://github.com/Longyuyeee/long_Decompress/releases/latest)
[![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011-0078d4?style=flat-square&logo=windows)](https://github.com/Longyuyeee/long_Decompress/releases/latest)
[![License](https://img.shields.io/badge/license-MIT-22c55e?style=flat-square)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-1.5-f59e0b?style=flat-square&logo=tauri)](https://tauri.app)

[下载最新版](https://github.com/Longyuyeee/long_Decompress/releases/latest) ·
[使用说明](#使用说明) ·
[格式支持](#格式支持) ·
[问题反馈](https://github.com/Longyuyeee/long_Decompress/issues)

</div>

---

## v1.0.8 更新亮点

- 新增应用内更新：启动后每天最多静默检查一次，也可在设置中心手动检查。
- 提供更新说明、稍后提醒、跳过版本、失败重试和中英文自定义更新弹窗。
- 更新包安装前必须通过 Tauri 数字签名验证，无法验证或被篡改的包会被拒绝。
- 压缩或解压任务运行期间禁止安装更新，避免退出覆盖程序时造成任务中断或数据损坏。
- GitHub Release 流水线自动生成 NSIS 安装包、更新 ZIP、签名和 `latest.json` 更新清单。
- 主程序与 Windows 11 右键扩展统一升级至 1.0.8，并完成单元、端到端、Rust 全目标和签名构建回归。

> 1.0.7 及更早版本尚未内置更新公钥，需要手动安装一次 1.0.8；以后即可在软件内完成更新。

完整变更请查看 [v1.0.8 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.0.8)。

## 为什么选择胧解压

| 能力 | 说明 |
| --- | --- |
| 压缩与解压 | 支持 ZIP、7Z、RAR、TAR、GZ、BZ2、XZ、Zstandard 等常见格式 |
| 批量任务 | 多文件排队处理，实时显示进度、阶段、速度和结果 |
| 加密归档 | 支持加密 ZIP、7Z、RAR 等格式解压，以及 ZIP、7Z、RAR 和专用 AES-256-GCM 格式加密压缩 |
| 密码保险箱 | 本地保存常用密码，遇到加密压缩包时自动匹配 |
| 文件完整性 | 计算与验证 CRC32、MD5、SHA256，支持导入、导出校验文件 |
| Windows 集成 | 支持文件拖放、快捷键、系统托盘、资源管理器现代右键菜单、开机启动和签名应用内更新 |
| 个性化 | 深浅主题、强调色、界面缩放、减少动效和无障碍选项 |

## 下载与安装

1. 前往 [GitHub Releases](https://github.com/Longyuyeee/long_Decompress/releases/latest)。
2. 下载名称以 `x64-setup.exe` 结尾的安装程序。
3. 双击安装，完成后从开始菜单启动“胧解压·方便助手”。

系统要求：

- Windows 10 1809 或更高版本、Windows 11
- x64 处理器
- 至少 4 GB 内存，建议 8 GB
- 建议预留 200 MB 磁盘空间

> 当前安装包尚未进行商业代码签名。如果 Windows SmartScreen 显示保护提示，请确认文件来自本项目 Releases 页面，再选择“更多信息”继续运行。

应用已经内置 7-Zip 命令行组件，无需额外安装 7-Zip。只有创建 RAR 文件时需要安装 [WinRAR](https://www.rarlab.com/download.htm)；解压 RAR 不受影响。

## 使用说明

### 解压文件

1. 打开左侧“解压中心”。
2. 拖入一个或多个压缩包，或点击浏览文件。
3. 选择输出目录、目录结构和文件冲突处理方式。
4. 点击开始解压，在任务列表中查看进度与日志。

遇到加密文件时，应用会先尝试密码保险箱中的密码；无法匹配时会提示手动输入。

### 压缩文件

1. 打开“压缩中心”。
2. 添加文件或文件夹，可为不同任务单独设置参数。
3. 选择格式、压缩级别、输出目录及是否分卷。
4. 如需加密，填写密码或使用密码生成器。
5. 点击开始压缩。

普通 TAR、GZ、BZ2、XZ、Zstandard 或 LZMA 格式本身不支持密码。为这些格式设置密码时，应用会明确创建加密的 `.7z` 文件；如需保留原压缩算法，可选择对应的 `.aes` 格式。

### 密码保险箱

1. 使用 `Ctrl+Shift+V` 打开密码保险箱。
2. 首次使用时完成保险箱初始化。
3. 添加常用密码并按名称或分组管理。
4. 解压加密文件时，应用会在本地自动尝试匹配。

密码数据保存在本机应用数据目录中，不会上传到网络。

### 文件完整性

使用 `Ctrl+I` 打开“文件完整性”：

- 选择文件后计算 CRC32、MD5 或 SHA256
- 导出 `.sfv`、`.md5`、`.sha256` 校验文件
- 导入已有校验文件并批量验证

### 软件更新

- 软件启动后会按设置静默检查正式版本，每 24 小时最多一次。
- 在设置中心的“软件更新”区域可以查看当前版本并立即检查。
- 发现更新后可查看更新内容、稍后提醒、跳过当前版本或下载并安装。
- 更新包必须通过应用内置公钥验证；任务运行期间不会执行安装。

### 右键菜单

在设置中心启用资源管理器右键菜单后，可以直接：

- 解压到当前目录
- 解压到同名文件夹
- 压缩为 ZIP 或 7Z
- 测试压缩包完整性

## 常用快捷键

| 快捷键 | 功能 |
| --- | --- |
| `Ctrl+O` | 解压中心 |
| `Ctrl+N` | 压缩中心 |
| `Ctrl+Shift+V` | 密码保险箱 |
| `Ctrl+I` | 文件完整性 |
| `Ctrl+,` | 设置中心 |

## 格式支持

| 操作 | 格式 |
| --- | --- |
| 常用压缩 | ZIP、7Z、TAR、TAR.GZ、TAR.BZ2、TAR.XZ、TAR.ZST、GZ、BZ2、XZ、ZST、LZMA |
| 加密压缩 | ZIP、7Z、RAR、TAR.AES、TGZ.AES、TBZ.AES、TXZ.AES、TZST.AES、GZ.AES、BZ2.AES、XZ.AES、ZST.AES |
| 常用解压 | ZIP、ZIPX、7Z、RAR、TAR、GZ、BZ2、XZ、ZST、LZMA |
| 兼容归档 | CAB、ISO、WIM、DMG、VHD/VHDX、DEB、RPM、MSI、ARJ、LZH、XAR、CPIO 等 |
| Office 与应用包 | DOCX、XLSX、PPTX、ODT、ODS、EPUB、APK、IPA、APPX、JAR 等 |

具体能力会受到文件本身、加密方式以及系统环境影响。RAR 创建需要 WinRAR；创建加密 RAR 时，应用会在执行前明确说明密码会短暂出现在本机进程参数中，并由用户确认是否继续。

## 常见问题

<details>
<summary><strong>为什么创建 RAR 时提示缺少编码器？</strong></summary>

RAR 是专有格式。请安装 WinRAR，重启应用后再创建 RAR；也可以改用 7Z。

</details>

<details>
<summary><strong>升级会删除密码和设置吗？</strong></summary>

不会。直接安装新版本即可，应用数据会继续保留。建议在重要升级前导出密码保险箱备份。

</details>

<details>
<summary><strong>任务失败后如何处理？</strong></summary>

展开任务日志查看具体原因。常见原因包括输出目录无写入权限、磁盘空间不足、密码错误、压缩包损坏或分卷不完整。

</details>

<details>
<summary><strong>如何恢复默认设置？</strong></summary>

前往设置中心逐项恢复；如果应用无法启动，可先备份应用数据，再清理配置目录后重新运行。

</details>

## 开发

<details>
<summary><strong>本地构建</strong></summary>

```powershell
git clone https://github.com/Longyuyeee/long_Decompress.git
cd long_Decompress\long-compress-assistant
npm install
npm run tauri dev
```

发布构建：

```powershell
npm run tauri build
```

主要技术栈：Vue 3、TypeScript、Pinia、Tauri 1.5、Rust。

</details>

## 反馈与许可

- 使用问题或功能建议：[提交 Issue](https://github.com/Longyuyeee/long_Decompress/issues/new)
- 项目主页：[Longyuyeee/long_Decompress](https://github.com/Longyuyeee/long_Decompress)
- 开源许可：[MIT License](LICENSE)

本项目包含按各自许可证分发的第三方组件，包括 Tauri、Vue、Rust crates 与 7-Zip 命令行工具。
