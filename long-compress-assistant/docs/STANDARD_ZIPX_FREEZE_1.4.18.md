# v1.4.18 封板与发布记录

2026-10-18 已封板并[正式发布 v1.4.18](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.18)。功能冻结 `688f1fc298244a8c1c14f70f6939a552b2981185`，八处身份统一、普通生产构建和必需 Windows package 通过；PR #175 已合入，标签 `9b700e1008df523d5a92f5fa67b7764acb092795` 与候选文件树一致。四项正式资产与中文更新说明已实际回下载核对。

## 冻结范围

ZIPX（XZ）标准 `.zxNN/…/.zipx` 的任意成员归组、浏览/预览、解压、检查与转换；无密码/AES-256/ZipCrypto 创建、改密/重新分卷/合卷、卷型保存与历史，以及缺卷准确原因和补回新任务。沿用可靠目录与已知卷边界的部分救援，不推测未知数据。保持数字卷默认值及完整校验，不新增 ZIPX 专有方法或特殊卷系统关联。

独立原卷由 Bandizip 7.38 Beta 原样生成，产品输出由 WinRAR 7.23 原名完整回解。Bandizip Beta 对产品生成内容异常退出、7-Zip 不识别 ZIPX 标准卷名的边界明确写入帮助和更新说明；未关闭校验、改名数字卷或修改尾记录掩盖差异。详见[实施审计](STANDARD_ZIPX_1.4.18.md)和[固定原卷](../tests/fixtures/archives/standard-zipx-1418/README.md)。

## 检查与准确身份

39 项不同 Rust 定向用例、11 文件 146 项相关前端回归通过。原生命令矩阵覆盖六种原卷成员入口、24 次结构失败、三类创建和改密/分卷/合卷；真实页面完成创建、配置组、转换、缺卷后补回，另接续文件浏览器双击中卷。全部载荷哈希一致，原失败保留。

[主要页面报告](evidence/standard-zipx-1.4.18/results.json)、[原生命令报告](evidence/standard-zipx-1.4.18/native.json)和[文件浏览器报告](evidence/standard-zipx-1.4.18/browser.json)及 11 张实看截图已入库。程序均为隔离 E2E、升版前 1.4.17，不是正式安装器：

| 覆盖范围 | SHA256 |
| --- | --- |
| 原生命令与独立互通 | 648baf3956a74f0ea4365061f84c86ffe95a3968794f71cc2458849bcb86ca64 |
| 压缩/配置组/转换/缺卷补回页面 | c423061a854715f4033ca1d8cdefd07a6cbe15c6f72f4d151a5d8cb8e57ed767 |
| 最后文件浏览器双击入口 | 0cf0f2b3eb7ea40ba0e8e87c709d5f7380221bafefbc05e60e442e48ca5a8889 |

源报告记录 UTC；主要页面完成 `2026-10-17T19:53:45.114Z`，北京时间 10 月 18 日。常用及窄窗口实际查看，新控件完整可达；截图等待异步缩窗和原生合成帧后保存。未改安装生命周期、媒体和旧引擎的证据按范围复用，个人安装、设置和历史未改。

## 发布接续

本地普通生产 EXE ProductVersion `1.4.18`，33123328 字节，SHA256 `bef2c09bcde6d62eff61082af45563f94adb1cd80a1ae5570749c4fbaf877350`。Shell 资源 `long_compress_shell_extension_1_4_18.dll` 为 248832 字节、SHA256 `b7246d3e62e98def2ff75c88f8d9c1ac1cea0b2b4c05c3e43b4fcffbc50cc6a3`。旧版本生成资源由正常构建替换，可从 Git 历史恢复；没有安装或改动个人程序。Windows 11 身份包因本机无签名配置按现有规则跳过，不冒称生成。

八处版本统一 1.4.18，普通生产构建、版本化 Shell 资源及身份检查通过，受保护 PR 的必需 Windows package 成功后核对候选、合入并确定正式标签。正式四资产齐全后回下载核对版本、更新 ZIP、签名/地址和中文说明。发布后文档另走 PR，不改标签或重打包。

下一版按已批准路线接续 v1.4.19 RAR4 旧格式恢复链，先审计真实旧卷、恢复记录/恢复卷及本机 Rar.exe 能力；后续路线尚未全部完成，不追加重复规划。

## 合入与候选产物

[PR #175](https://github.com/Longyuyeee/long_Decompress/pull/175) 已受保护合入，候选 `81d87d9421d55f7b755f90f767a798cde265916e`；合并及正式标签 `v1.4.18` 指向 `9b700e1008df523d5a92f5fa67b7764acb092795`，文件树与候选完全一致。没有绕过保护规则或移动标签；重复主线 CI `39065565311` 已取消，正式发布构建另行执行。

必需 [Windows package / 39064998601](https://github.com/Longyuyeee/long_Decompress/actions/runs/39064998601) 成功。附件 `windows-nsis-installer` ID `12112189516`、20211085 字节、附件 ZIP 摘要 `e28a4be21372da764f9c60f39e76a43026ce493d423e4a3f75e3fa7e0c5bc670`。实际下载的 `Long解压_1.4.18_x64-setup.exe` 为 20225212 字节、ProductVersion `1.4.18`、SHA256 `eff89286e7833e06f587a31570b40782a03d1caecb96c2734abda513b7b5ad4c`。

[正式发布构建 / 39065587677](https://github.com/Longyuyeee/long_Decompress/actions/runs/39065587677) 已成功，发布时间 `2026-10-17T20:25:18Z`（北京时间 10 月 18 日 04:25）。正式安装器 ProductVersion `1.4.18`；更新 ZIP 内安装器与下载页 EXE 哈希相同，签名/地址与清单一致。中文 Release 正文与重新下载的 latest.json 说明相同；仅替换清单的 notes，版本、签名、地址及发布时间不变，不重新签名或打包。

## 正式资产

以下文件均实际下载，大小及 SHA256 与 GitHub 资产摘要核对；详见[机器核对记录](evidence/standard-zipx-1.4.18/release.json)。候选与正式构建分别记录，不互相冒充。

| 文件 | 字节数 | SHA256 |
| --- | ---: | --- |
| Long-Decompress_1.4.18_x64-setup.exe | 20229064 | cdba047ce1f48cf9b0009071886667aba18116830370c04717b8a69f27dc3521 |
| Long-Decompress_1.4.18_x64-setup.nsis.zip | 20229224 | b14aa26c8e15e993e9329c62d73d664a554b8fc8ee8ab6f32ffa3b64e50f6811 |
| Long-Decompress_1.4.18_x64-setup.nsis.zip.sig | 428 | 7ce600787525e4ad0a1fcef0c3a8a6cc140e2a93a438264f8a0480ce99048637 |
| latest.json | 2327 | 802416c30e32ceeaede0b46c0fcce5786b130dbaaaf37b1f3a4a8c5125607396 |
