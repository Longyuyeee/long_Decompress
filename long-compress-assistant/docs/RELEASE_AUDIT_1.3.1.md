# Long解压 v1.3.1 发布审计

## 发布定位

- v1.3.1 是 v1.3.0 后的 RAR 密码判定与源文件稳定性补丁。
- 发布提交为 `b4efcfb1a18dc0b68c7d603a070ad4b634c7dad6`，由 [PR #123](https://github.com/Longyuyeee/long_Decompress/pull/123) 合入 `master`。
- annotated 标签 `v1.3.1` 精确指向该合并提交。

## 修复与本机验证

- 本机历史界面共识别 29 条失败记录、18 个去重 RAR；4 个源文件已不存在，14 个仍可读取。
- 14 个仍存在的文件均完成无落盘完整读取。13 个加密 RAR 与当前保险箱候选不匹配，修复版没有误报“找到密码”；1 个未加密 RAR 的 168 个条目全部可读，其历史失败属于后续输出提交问题。
- Rust 主库 389/389、RAR 聚焦回归 3/3、固定真实加密 RAR 的错误/正确密码及完整解压、自动密码来源 6/6、严格 Clippy均通过。
- v1.3.1 发布身份检查通过，Shell 扩展资源为 `long_compress_shell_extension_1_3_1.dll`。

## GitHub Release

- [Release workflow 33904390189](https://github.com/Longyuyeee/long_Decompress/actions/runs/33904390189) 于 14 分 20 秒完成，所有步骤通过。
- [Long解压 v1.3.1](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.3.1) 已公开，非草稿、非预发布。
- 公开资产共 4 项：`Long-Decompress_1.3.1_x64-setup.exe`、`Long-Decompress_1.3.1_x64-setup.nsis.zip`、独立 `.sig` 以及 `latest.json`。
- 安装器大小为 19,375,876 字节，GitHub 公布的 SHA-256 为 `b00dd0ebf739b40f8ddfc152e6dfd2bf99a645b1318846fd69e36e33d69c7d87`。

## 接续边界

- 密码误报修复和 v1.3.1 发布阶段已关闭，下一轮从最新 `master` 接续。
- 那个完整读取成功、但在输出提交阶段失败的未加密 RAR 是独立问题；本补丁没有把它错误归入密码问题，也没有宣称已经修复该提交阶段故障。
