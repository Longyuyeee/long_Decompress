# Long解压 v1.3.2 发布审计

## 发布定位

- v1.3.2 是 v1.3.1 后的损坏 RAR 识别与失败任务最终原因补丁。
- 修复由 [PR #124](https://github.com/Longyuyeee/long_Decompress/pull/124) 合入，发布提交为 `4cc792ef9b1e754112e07a707c2facecf7560eae`。
- annotated 标签 `v1.3.2` 精确指向该合并提交。

## 问题与修复

- 部分损坏或未写完的 RAR 会在不完整目录输出中暴露 `Encrypted = +`，同时目录命令以非零状态结束。旧逻辑先信任局部加密标记，导致损坏文件错误进入密码保险箱和密码字典流程。
- 新逻辑仅在目录读取完整成功后信任普通加密元数据；明确的纯密码错误仍进入解锁，其他非零结果直接报告“RAR 文件损坏或不完整，无法读取完整目录”。
- 所有失败任务现在都在进入终态和持久化前写入错误摘要，并保证最后一条错误日志为“最终失败原因”。前端状态层、后端事件和历史保存层均有兜底，避免失败记录只有状态而没有原因。
- 修复前已经写入且错误原文为空的旧历史记录无法还原当时异常；修复对之后新产生和重新执行的失败任务生效。

## 本机验证

- 对同批 12 个近期真实损坏 RAR 逐项运行产品目录探测路径：12/12 直接报告损坏或不完整，0 个进入密码搜索。
- 前端任务与历史聚焦回归 29/29 通过，TypeScript 类型检查通过。
- Rust 的损坏目录分类、真实损坏 RAR 和历史最终原因回归通过；严格 Clippy 通过。
- 版本身份检查通过，版本统一为 `1.3.2`，Shell 扩展资源为 `long_compress_shell_extension_1_3_2.dll`。

## GitHub Release

- [Release workflow 33908245518](https://github.com/Longyuyeee/long_Decompress/actions/runs/33908245518) 于 15 分 22 秒完成，结论为 `success`。
- [Long解压 v1.3.2](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.3.2) 已公开，非草稿、非预发布，发布时间为 2026-09-05 03:07（Asia/Shanghai）。
- 公开资产共 4 项：`Long-Decompress_1.3.2_x64-setup.exe`、`Long-Decompress_1.3.2_x64-setup.nsis.zip`、`Long-Decompress_1.3.2_x64-setup.nsis.zip.sig` 和 `latest.json`。
- 安装器大小为 19,399,505 字节，SHA-256 为 `cead66e9fbe5a08c490e1c7e18e6fb160eac49b31a75fd28e491ad414d1a2d3e`。
- updater ZIP 大小为 19,399,663 字节，SHA-256 为 `a3656989cca369a59f420d19c704b7fb11eaf20cc90f8110ffb2046ed6e73246`；独立签名为 428 字节；`latest.json` 为 949 字节。

## 接续边界

- 损坏 RAR 误入密码流程、未来失败任务缺失最终原因和 v1.3.2 正式发布均已关闭。
- 下一轮开发必须从最新 `master` 接续，不应从 PR #124 的功能分支或 v1.3.2 标签继续提交。
- 本轮按用户要求没有安装或替换当前正在使用的本机版本；公开安装包和自动更新资产由 GitHub Release 提供。
