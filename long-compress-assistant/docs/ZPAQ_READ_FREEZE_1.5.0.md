# v1.5.0 封板与发布记录

功能已封板，正式构建和发布尚未完成。14 项不同 Rust、69 项相关前端、类型检查及 6 组真实桌面矩阵通过；16 张常用/窄窗口截图已逐张查看。不得将升版前测试 EXE 或候选安装包当正式版。

功能冻结提交 `1c40f36a8ffec2b65c7db6bde7742a4d509c5033`。八处版本统一到 1.5.0，接下来仅执行普通生产构建、候选与发布，不扩张 ZPAQ 创建范围。

普通生产构建及 `node scripts/check-release-identity.mjs --expected 1.5.0` 已通过，未启用 desktop-e2e。本地 EXE 为 ProductVersion 1.5.0、33243648 字节、SHA256 `62410167253b91ba27899bea8bfbf792190821613a7d55076c30ae4c4d98af23`；Shell DLL 为 248832 字节、SHA256 `21a290052f04b7c0a7e35fa5449eee0d669639e1099d1d6930c256ab856b9d06`。旧版生成 DLL 由正常构建替换，可从 Git 恢复；本机未配置 Windows 11 身份包签名，按原规则跳过，不宣称生成。

## 冻结边界

ZPAQ 当前文件浏览、原生密码、完整/所选读取、检查与转换输入；日志式与流式分别验证。无创建、原生多部分、增量备份和历史恢复承诺。算法沿用作者 7.15，适配器增加机器目录、标准输入凭据及严格错误传播，源码/二进制/许可证哈希锁定，Git 原始字节已核对。

## 验证与发布接续

[PR #181](https://github.com/Longyuyeee/long_Decompress/pull/181) 已受保护合入，候选 `debe7c242715984bc044c67e325856d9a96d16ee`，合入和标签固定于 `1a1d6d02099179341df99bdb8d9a95742c2716e4`，两者文件树同为 `0e555e23e0f62cfa47cc5bdb615b60cd47be3444`。必需 [Windows package / 39076966449](https://github.com/Longyuyeee/long_Decompress/actions/runs/39076966449) 成功；重复 master 推送 CI `39077462557` 已取消，不影响已通过的候选门禁。

候选安装器 ProductVersion 1.5.0、20524431 字节、SHA256 `3d9f1861a36b2f4bd3451d654abfcea8c31aabf925063eb6ae553c7664ba698d`。已从 NSIS 实际提取 `resources/zpaq-engine/long-zpaq.exe` 和 COPYING，与锁定哈希一致，并直接用包内引擎读取普通及中文密码原包的三个当前文件，见[候选收据](evidence/zpaq-read-1.5.0/candidate.json)。未执行安装器，不改变个人安装。

正式 [Release 构建 / 39077482272](https://github.com/Longyuyeee/long_Decompress/actions/runs/39077482272) 已启动，尚待完成和正式四资产回下载，未把候选当正式版。

最终[桌面证据](evidence/zpaq-read-1.5.0/results.json)完成于 UTC `2026-10-17T22:50:18.074Z`；升版前 EXE ProductVersion 1.4.20，SHA256 `646537d2549f3f8e38233289c6dade6520b53f4b4f4357ec7b5b60dee12d6b06`。记录功能冻结提交后，再统一八处版本、普通生产构建、候选必需 Windows package、受保护合入和固定标签。正式发布后回下载四资产核对版本、哈希、更新 ZIP、签名、地址及中文说明，发布后文档单独合入，不移动标签。

过程问题与纠正见[实施审计](ZPAQ_READ_1.5.0.md)。个人安装、设置与历史保持；旧版生成 Shell DLL 由正常构建替换，可从 Git 恢复。Windows N 实机门禁不恢复。
