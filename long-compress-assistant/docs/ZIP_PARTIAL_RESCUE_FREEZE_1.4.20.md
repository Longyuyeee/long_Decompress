# v1.4.20 封板与发布记录

2026-10-18 已封板并[正式发布 v1.4.20](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.20)。功能冻结 `4388e9d`，八处身份、普通生产构建和必需 Windows package 成功；PR #179 已受保护合入，正式标签 `64e65f7` 与候选文件树相同，正式四资产和中文说明均已回下载核对。不把 E2E 程序或候选包当正式资产。

## 冻结范围与证据

真实好压 ZipCrypto 论坛 ZIP 缺首/中卷部分抢救，以及 ZIP 各恢复路径的逐文件结果分类。仅凭完整目录、原生卷偏移及现有连续卷段决定可尝试条目，实际校验通过后才进入副本；不猜缺终卷、未知卷长度或加密边界。完整普通解压仍要求全部原卷。

40 项不同 Rust、29 项相关前端及类型检查通过。9 组真实桌面命令矩阵、独立工具回解 SHA256 比对、失败任务纠正密码/另存/重新入队、补原卷完整解压及批量页通过。12 张常用/窄窗截图已逐张查看。过程中的测试编排错误和实看后汇总纠偏见[实施审计](ZIP_PARTIAL_RESCUE_1.4.20.md)，不为验收修改业务结果。

[最终证据](evidence/zip-partial-rescue-1.4.20/results.json)记录升版前隔离 E2E EXE SHA256 `6b3902a2423667c088964b5bb5a262d3477acd5516f3c31ef22e596d0a992ff1`、ProductVersion 1.4.19，完成时间 `2026-10-17T21:43:31.345Z`。原始论坛卷来源和哈希沿用[固定样本](../tests/fixtures/archives/haozip-forum-encrypted-1411/manifest.json)，不重新制造或改名。

## 发布接续

功能冻结提交为 `4388e9decab06d4432088b33da825553f4927954`。八处身份已统一到 1.4.20，普通生产构建与身份检查通过，未启用 `desktop-e2e`。本地 EXE ProductVersion `1.4.20`、33034752 字节、SHA256 `b5fb5520c086547a70ff01563f2ceb236cac958bf4b2347bef06887bbf24833a`；Shell DLL `long_compress_shell_extension_1_4_20.dll` 为 248832 字节、SHA256 `ce188a14287ec350a8189095410f5e4f95fd47ebb8f31dc86dfdc1926b42996c`。旧版本生成 DLL 由正常构建替换，可从 Git 恢复。Windows 11 身份包因本机无签名配置按既有规则跳过，不宣称生成。

已按既定顺序统一身份、构建普通生产程序及版本化 Shell DLL、完成必需 Windows package、受保护合入和固定标签，再正式发布及回下载四资产。发布后文档单独提交，不改功能、不重打包、不移动标签。

下一步按既定路线进入 v1.5.0 ZPAQ 浏览、原生密码读取与转换，不提前宣称支持、不扩张增量备份或历史恢复交互。个人安装、设置和历史保持。

## 候选与受保护合入

[PR #179](https://github.com/Longyuyeee/long_Decompress/pull/179) 已受保护合入，候选 `306c50b6a44b736f155332140724c5980f9f993d`，合入及标签 `v1.4.20` 固定于 `64e65f712cb9908188a27a41ac5538c3ae7f7699`；已核对两者 Git 文件树相同。没有移动旧标签或绕过分支保护。

必需 [Windows package / 39072422783](https://github.com/Longyuyeee/long_Decompress/actions/runs/39072422783) 成功。实际回下载的候选 `Long解压_1.4.20_x64-setup.exe` 为 ProductVersion `1.4.20`、20251716 字节、SHA256 `71a40d36036499227a164faddde2d630b44935d8992c0ac59397eb6556e6d839`；候选附件 ID `12116295405`、20237600 字节、附件摘要 `1974e03d24aae2e47726cbd98bc398091e331bf8acec02b3be1f08e0fbf9395a`，不冒充正式发布资产。

[正式构建 / 39072953191](https://github.com/Longyuyeee/long_Decompress/actions/runs/39072953191) 成功，发布时间 UTC `2026-10-17T22:10:37Z`（北京时间 10 月 18 日 06:10）。正式四资产已实际回下载，安装器 ProductVersion `1.4.20`；更新 ZIP 中安装器与下载页 EXE 哈希一致，签名/URL 与清单一致，中文 Release 与 latest.json 说明一致。清单只替换 notes，版本、签名、地址和发布时间保留。相同源码的重复主线 CI `39072934534` 已确认取消。

## 正式资产

文件大小及 SHA256 与 GitHub 资产摘要逐项核对，机器记录见 [release.json](evidence/zip-partial-rescue-1.4.20/release.json)。

| 资产 | 字节 | SHA256 |
| --- | ---: | --- |
| `Long-Decompress_1.4.20_x64-setup.exe` | 20256899 | `d1fd13b911d0d04edd3134a04b19219209fdbdd0b19cf1642167154b58ae0426` |
| `Long-Decompress_1.4.20_x64-setup.nsis.zip` | 20257059 | `aa96a475ca71bd88d7b68cd492a7ca487656f6d4a52f43163493d8bb9314d761` |
| `Long-Decompress_1.4.20_x64-setup.nsis.zip.sig` | 428 | `6059bb022c7fb2559aa9a08e6a442bd95f08e9616eee9d465a9fea5d170ff4c0` |
| `latest.json` | 2580 | `23c60f28df27aea941719c481ac2a91c9263150c368cde3a37abd662e64d2495` |
