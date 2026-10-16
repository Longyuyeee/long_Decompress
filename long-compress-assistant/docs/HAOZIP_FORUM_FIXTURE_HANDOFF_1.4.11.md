# v1.4.11 加密论坛卷样本交接

## 2026-10-16 当前机器接续

本次在 `E:\Project\AIProject\GeminiProject\long_Decompress` 接续。旧文档下面的 `E:\ProjectSave\ClaudeProject` 目录属于另一台机器，本机不存在；请使用新准备的目录：

`E:\Project\AIProject\GeminiProject\long_Decompress\long-compress-assistant\test-results\haozip-forum-manual-iCzUI4`

已通过已安装的 `E:\Haozip\HaoZip.exe`（6.5.3.11255）进入 `原文件` 并选中中文目录和说明文件。关联提示已取消。鼠标点击“添加”被操作工具两次判定为落在 `Weixin.exe`，重新激活并刷新后仍失败，停止该点击路径，未生成论坛卷。已向用户请求完成一次生成；无需再次确认允许启动，也不要求重新安装软件。好压命令行帮助只有标准分卷参数，未发现论坛模式入口，没有猜参数生成替代样本。

新增桌面入口 `--haozip-forum-encrypted-only` 复用已有用户链，只更换为真实论坛卷输入。运行前设置 `LONG_HAOZIP_FORUM_MANIFEST` 为上述目录的 `fixture-inputs.json`，`LONG_HAOZIP_GUI` 为实际生成器，`LONG_BANDIZIP_C` 为独立校验工具。脚本校对原输入哈希和连续卷名，将原始论坛卷按原名复制到隔离目录，检查所有成员的错密码、正确密码、解压、完整性及转换，再检查补卷重试、独立目录和原失败保留；最终核对原生成物未变。没有样本时明确失败，不自动创建或改名标准卷。

该入口已完成脚本语法检查，尚未执行真实样本，不能计作兼容完成。截图与结果文件使用独立的 `haozip-forum-encrypted-1411` 前缀，不覆盖先前标准卷证据。

```powershell
$env:LONG_HAOZIP_FORUM_MANIFEST = (Resolve-Path 'test-results/haozip-forum-manual-iCzUI4/fixture-inputs.json').Path
$env:LONG_HAOZIP_GUI = 'E:/Haozip/HaoZip.exe'
$env:LONG_BANDIZIP_C = (Resolve-Path 'test-results/bandizip-forum-validator/current/bz.x64.exe').Path
node scripts/test-tauri-desktop.mjs --haozip-forum-encrypted-only
```

其中 Bandizip 使用从官方 7.47 安装包解出的命令行工具及同目录库，只用作测试，不运行安装器。桌面运行仍需现有 `EDGE_DRIVER_PATH`、WebView2 及隔离 E2E 程序配置。

## 当前需要完成的操作

请用好压把下面的非私人测试文件生成一组**真正的加密论坛 ZIP 分卷**，保留全部原始卷名，再告知保存目录。本机操作工具两次把条目点击判定到其他程序，按恢复限制不能继续点击；并非缺少启动许可，也不要求换电脑。应用的完整兼容验收仍由开发端完成。

本机已准备的目录是：

`E:\ProjectSave\ClaudeProject\Long-Decompress\long-compress-assistant\test-results\haozip-forum-manual-SL3PpG`

其中 `原文件` 包含中文说明、163,871 字节二进制及一个空目录；`好压生成的论坛卷` 目前为空。`fixture-inputs.json` 保存预期文件名、长度与 SHA256，状态明确为 `awaiting-manual-creation`，它不是归档产物或验收通过证据。

## 在好压中生成

1. 打开上述 `原文件` 目录，选中里面的 `说明.txt` 和 `中文目录`，点击“添加”。不要选择个人文件。
2. 选择 **ZIP**，分卷大小设为 **64 KB / 65,536 字节**，启用“生成论坛专用分卷”，设置测试密码 **`ForumTest-1411`**。若提供加密算法选择，优先使用传统 ZIP 密码（ZipCrypto）。
3. 将压缩包保存到同级 `好压生成的论坛卷`，名称使用 `论坛密码验收.zip`。不需要删除源文件、修改默认关联或运行安装器。
4. 完成后保留 `论坛密码验收.haozip01.zip`、`...haozip02.zip` 等**全部卷**。请回复“已生成”，或告知其他保存目录及实际测试密码。若输出是 `.z01/.z02/.zip`，不要自行改名；说明实际结果即可。找不到论坛选项时可提供压缩设置窗口截图。

“生成论坛专用分卷”才会产生 `.haozipNN.zip`；普通 ZIP 多卷使用 `.zNN/.zip`。参见 [Bandizip 官方说明](https://www.bandisoft.com/bandizip/howto/open-haozip01.zip/)。本版只验收 ZIP 论坛卷，不扩到 7Z 论坛卷。

## 重新准备测试输入

在 `long-compress-assistant` 目录执行：

```powershell
node scripts/prepare-haozip-forum-fixture.mjs
```

脚本每次新建一个独立临时测试目录，打印 `sourceDirectory`、`archiveOutputDirectory` 和清单路径，不覆盖已有文件，不启动好压、不生成归档。说明和二进制内容确定，换电脑后预期 SHA256 相同；二进制采用非重复块，实际 Deflate 后仍超过两个 64 KiB 卷的容量。脚本自身两项检查通过，包括内容/清单一致、压缩后大小及重复准备不覆盖旧输出。

```powershell
node --test scripts/prepare-haozip-forum-fixture.test.mjs
```

## 拿到原始卷后的验收

记录生成器版本、实际加密选项、每卷名称/大小/SHA256，先由独立工具检查、回解并与输入清单比对，再从各成员执行应用浏览、错误密码后纠正、完整解压、检查及转 ZIP。随后按真实顺序验证缺卷原因、补卷后重试、独立输出设置与原失败记录保留；涉及页面按常用及窄窗口查看。原始卷保持不变，不拿改名标准卷替代。

本版产品功能提交 `b55bab622fd9b266a1c8958c8ae3e9ca8f93d379` 的 [Windows package](https://github.com/Longyuyeee/long_Decompress/actions/runs/38803602996) 已成功。现有功能、实际页面和测试证据见 [本版实施审计](SFX_AND_VOLUME_COMPATIBILITY_1.4.11.md)。完成真实加密论坛卷验收后，才接统一版本、README、中文 Release、最终安装器检查和发布；当前仍未封板。
