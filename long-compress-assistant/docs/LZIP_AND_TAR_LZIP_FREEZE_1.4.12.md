# v1.4.12 功能封板与正式发布

日期：2026-10-16（Asia/Hong_Kong）。分支 `codex/lzip-tar-lzip-1-4-12`，基线为已正式发布 v1.4.11 后的主线 `0bc3779`。

## 正式发布

[PR #166](https://github.com/Longyuyeee/long_Decompress/pull/166) 已合并；合并提交及正式标签 `v1.4.12` 均为 `a231b643063b303f373ec58c24f78f75e603948d`。产品功能冻结在 `8a3fe7b`，最终分支提交 `c704dacc99597ed97d75dfeb1632c31246dd0e40` 仅补原始日志；其 [Windows package](https://github.com/Longyuyeee/long_Decompress/actions/runs/38856433554) 已通过，普通安装包附件为 `12027349317`（`windows-nsis-installer`，ZIP 20,172,016 字节）。

[正式发布构建](https://github.com/Longyuyeee/long_Decompress/actions/runs/38860840426) 从上述标签生成 Windows x64 安装器、更新 ZIP、更新签名和 `latest.json`，下载见 [v1.4.12](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.12)。中文 Release 与更新清单的版本说明一致；更新包 URL 和签名保持发布构建的原值。公开资产的大小、摘要及发布时间见 Release，本记录不把候选安装包 ZIP 大小冒称正式 EXE 大小。

正式版于 2026-10-16 14:34:59（Asia/Hong_Kong）公开，发布构建于 14:35:36 成功结束；Release 为稳定版且已设为最新版本。

| 公开附件 | 大小（字节） |
| --- | ---: |
| `Long-Decompress_1.4.12_x64-setup.exe` | 20,186,792 |
| `Long-Decompress_1.4.12_x64-setup.nsis.zip` | 20,186,952 |
| `Long-Decompress_1.4.12_x64-setup.nsis.zip.sig` | 428 |
| `latest.json` | 3,120 |

安装器 GitHub SHA-256 为 `e9fac604f2e6e067fccfafedb2188dbd6d52df83a0cab8eb82d436580d7895e1`。说明同步脚本首次按默认响应解析比较中文时未通过；改用公开附件原始 UTF-8 内容读取，确认版本、中文正文、更新包地址及签名全部一致，没有重传安装器或重跑功能检查。

发布只执行必要的正式打包；重复主线 CI `38860816519` 已取消，未重跑下述功能、桌面或安装生命周期检查。两份 README、格式说明、路线及交接已同步；v1.4.13 尚未启动。

## 已收口范围

- LZIP v1 单文件与连续多成员读取；TAR.LZIP / `.tlz` 浏览、预览、选择解压、检查与转换。
- 内置 LZIP / TAR.LZIP 创建、中文名称、空文件及目录保留，小输入字典调整和实际编码抽样。
- 原行内配置与多选弹窗接入新格式；不适用的密码、分卷、固实、文件名加密关闭，恢复草稿的旧密码必填状态同步清除。
- 浏览失败原因持续显示；LZIP 校验失败与文件不完整有中文原因，取消后的旧错误不覆盖当前状态。
- 右键格式识别、过滤与能力说明同步；既有交互、图标、等宽详情和小窗口滚动保持。

## 已有检查直接复用

定向前端、后端、固定 GNU 原始样本、实际命令与独立回解已完成。原生命令首次程序为 `94DDE39538AAB79840819AD6D4C2C8687461CFAE1077751E302335925ECB46F9`；错误显示修复后 UI 程序为 `69bd1acaa54864555bd68473a9f6dbdd3931723958d21821ea1a17e5f1d95a74`，版本均为 1.4.12，分别记录，未冒称正式安装器。10 张最终常用/最小窗口截图已逐张查看，阶段接续、早期失败和复用范围见 [实施记录](LZIP_AND_TAR_LZIP_1.4.12.md)。

## 功能冻结时的发布条件（已满足）

八处版本来源与 `long_compress_shell_extension_1_4_12.dll` 一致。两份 README、格式说明、路线、交接和 [中文 Release](RELEASE_NOTES_1.4.12.md)同步；该分支 PR 最新提交的 **Windows package 成功即达到本轮可发版目标**。该条件已由上述最终分支运行满足，用户随后授权合入与正式发布，不重跑已通过的格式矩阵。

正式发布从合并后的版本提交生成安装器与签名更新附件，准确身份和运行见上方“正式发布”。

## 本版边界

只承诺 LZIP v1 与 TAR.LZIP 的上述实际组合；不提供 LZIP 原生密码、分卷创建或损坏载荷恢复。GNU lzip 是独立测试工具，不进入安装包。下一版损坏包部分抢救尚未启动；Windows N 暂不保证支持。
