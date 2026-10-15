# v1.4.11 加密论坛卷样本交接

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
