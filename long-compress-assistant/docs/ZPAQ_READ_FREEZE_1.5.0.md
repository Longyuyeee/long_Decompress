# v1.5.0 封板与发布记录

2026-10-18 已封板并[正式发布 v1.5.0](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.0)。14 项不同 Rust、69 项相关前端、类型检查及 6 组真实桌面矩阵通过；16 张常用/窄窗口截图已逐张查看。正式四资产、中文说明与包内 ZPAQ 引擎已回下载核对，不将升版前测试 EXE 或候选安装包当正式版。

功能冻结提交 `1c40f36a8ffec2b65c7db6bde7742a4d509c5033`。八处版本已统一到 1.5.0，普通生产、候选与正式发布构建完成，未扩张 ZPAQ 创建范围。

普通生产构建及 `node scripts/check-release-identity.mjs --expected 1.5.0` 已通过，未启用 desktop-e2e。本地 EXE 为 ProductVersion 1.5.0、33243648 字节、SHA256 `62410167253b91ba27899bea8bfbf792190821613a7d55076c30ae4c4d98af23`；Shell DLL 为 248832 字节、SHA256 `21a290052f04b7c0a7e35fa5449eee0d669639e1099d1d6930c256ab856b9d06`。旧版生成 DLL 由正常构建替换，可从 Git 恢复；本机未配置 Windows 11 身份包签名，按原规则跳过，不宣称生成。

## 冻结边界

ZPAQ 当前文件浏览、原生密码、完整/所选读取、检查与转换输入；日志式与流式分别验证。无创建、原生多部分、增量备份和历史恢复承诺。算法沿用作者 7.15，适配器增加机器目录、标准输入凭据及严格错误传播，源码/二进制/许可证哈希锁定，Git 原始字节已核对。

## 验证与发布接续

[PR #181](https://github.com/Longyuyeee/long_Decompress/pull/181) 已受保护合入，候选 `debe7c242715984bc044c67e325856d9a96d16ee`，合入和标签固定于 `1a1d6d02099179341df99bdb8d9a95742c2716e4`，两者文件树同为 `0e555e23e0f62cfa47cc5bdb615b60cd47be3444`。必需 [Windows package / 39076966449](https://github.com/Longyuyeee/long_Decompress/actions/runs/39076966449) 成功；重复 master 推送 CI `39077462557` 已取消，不影响已通过的候选门禁。

候选安装器 ProductVersion 1.5.0、20524431 字节、SHA256 `3d9f1861a36b2f4bd3451d654abfcea8c31aabf925063eb6ae553c7664ba698d`。已从 NSIS 实际提取 `resources/zpaq-engine/long-zpaq.exe` 和 COPYING，与锁定哈希一致，并直接用包内引擎读取普通及中文密码原包的三个当前文件，见[候选收据](evidence/zpaq-read-1.5.0/candidate.json)。未执行安装器，不改变个人安装。

正式 [Release 构建 / 39077482272](https://github.com/Longyuyeee/long_Decompress/actions/runs/39077482272) 成功，发布时间 UTC `2026-10-17T23:17:56Z`（北京时间 10 月 18 日 07:17:56）。正式四资产均已回下载并与 GitHub digest/字节数核对；安装器 ProductVersion 1.5.0，更新 ZIP 内安装器与独立安装器相同，清单签名、下载地址、发布时间及中文说明一致。仅更新清单 notes，未改变签名、地址或安装包；见[正式收据](evidence/zpaq-read-1.5.0/release.json)。正式 NSIS 包内引擎与许可证哈希也一致，独立执行包内引擎读取普通及中文密码原包通过，没有执行安装升级或更改个人数据。

最终[桌面证据](evidence/zpaq-read-1.5.0/results.json)完成于 UTC `2026-10-17T22:50:18.074Z`；升版前 EXE ProductVersion 1.4.20，SHA256 `646537d2549f3f8e38233289c6dade6520b53f4b4f4357ec7b5b60dee12d6b06`。已按功能冻结、统一身份、普通生产、必需候选、受保护合入、固定标签、正式构建和回下载的顺序完成。发布后仅合入 README/审计/交接收据，不改功能、不重打包、不移动标签。下一版按批准路线推进 v1.5.1 ZPAQ 创建闭环。

过程问题与纠正见[实施审计](ZPAQ_READ_1.5.0.md)。个人安装、设置与历史保持；旧版生成 Shell DLL 由正常构建替换，可从 Git 恢复。Windows N 实机门禁不恢复。

## 正式资产

| 文件 | 字节 | SHA256 |
| --- | ---: | --- |
| Long-Decompress_1.5.0_x64-setup.exe | 20523062 | `6e95a51be8a3bfe36df3dad9e18a77139879d03135cf309c790f27747d6da4db` |
| Long-Decompress_1.5.0_x64-setup.nsis.zip | 20523220 | `bddc60f4064de0d68479ffef37f2b000e5e0642093d6cbf14a655760e9ee52c5` |
| Long-Decompress_1.5.0_x64-setup.nsis.zip.sig | 428 | `677e2fb307c8eeb21cbf8cf3fffc53c29eb97d71dc3432605d3be19e9df980e3` |
| latest.json | 2880 | `68079da8717dbed34aee55258051f161a6c51e342e56b6a279249c2c13c41a9a` |

以上为正式资产，与候选安装器哈希不同；不要混用。
