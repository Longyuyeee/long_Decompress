# Long解压 v1.2.3 发布审计

日期：2026-09-02

## 发布结论

`v1.2.3` 已完成需求、实现、真实媒体执行、实际软件观察、版本身份、安装包、CI、公开资产及应用内更新闭环。下一次开发必须从最新 `master` 和公开 `v1.2.3` 开始。

## 需求与真实验证

- 图片设置使用主题化下拉；视频质量与分辨率独立，默认保持原尺寸；图片、视频、PDF 任务区均可纵向滚动且不压扁卡片。
- FFmpeg、FFprobe、qpdf 等生产子进程统一隐藏窗口启动；视频探测按路径、字节和修改时间缓存，单项配置不清空真实探测事实。
- 三类特殊压缩均进入统一任务栏并显示准确工作负载；PDF 重复空态说明和窗口最外层 1 px 白边已删除。
- Release Tauri/WebView2 图片、视频、PDF 门禁分别通过；视频覆盖 10 分钟和 114,842,332 B 输入、真实编码、取消清理、默认播放器和重启历史。
- computer-use 实际打开 Release 软件，逐屏确认窗口边缘、三个 Tab、图片主题下拉、视频质量/分辨率设置和 PDF 空态。

完整预期—实际差异见 [SPECIAL_COMPRESSION_RUNTIME_UX_AUDIT_1.2.3.md](SPECIAL_COMPRESSION_RUNTIME_UX_AUDIT_1.2.3.md)。

## 发布门禁

| 门禁 | 结果 |
| --- | --- |
| 本地前端 | 类型检查通过；49 文件、282/282 单元测试通过 |
| 本地 Rust Release | 主库 384/384 通过、10 项显式忽略；桌面入口与全部集成测试目标 0 失败 |
| 媒体架构/版本身份 | 17 个生产媒体文件通过；八处 `1.2.3` 身份和唯一 Shell DLL 通过 |
| 本地 NSIS | 19,407,306 B；SHA-256 `FDC8212D9D4D368E5042899D81649AF8E74ABB391553F7140CAA71AA9CB77E00`；文件/产品版本均为 `1.2.3` |
| PR CI | PR #115 五项全绿：浏览器壳层、前端、Rust/Shell、Windows 桌面构建、Windows installer |
| Release CI | run `33598489918` 全绿，标签提交 `19a1fbe2579e52e551a6bc723b7a3c3164360468` |
| 公开更新 | `v1.2.2 → v1.2.3` 25/25、失败 0 |

Playwright CLI 本轮在创建浏览器进程前持续高 CPU、未产生用例结果；`--list` 可列出 55 项。该运行器异常没有被写成通过，也没有覆盖真实 Tauri 门禁结论。

## 公开资产回下载

| 资产 | 字节 | SHA-256 |
| --- | ---: | --- |
| `latest.json` | 958 | `E6BE2FFA78CC74D3CB02B1988A38C92F040C0A5A8E22CF25A07C02FCFFD19605` |
| `Long-Decompress_1.2.3_x64-setup.exe` | 19,347,501 | `5E15108DA9BDF7E551054D6B50C8C35915E76436B26223109FA88607AF0A890A` |
| `Long-Decompress_1.2.3_x64-setup.nsis.zip` | 19,347,659 | `F741F0641754358C6677009261F5B912B71B2B93B2A8B6CFEB6BDBD0239B393E` |
| `Long-Decompress_1.2.3_x64-setup.nsis.zip.sig` | 428 | `D63249571BCE505EF2B0F071D2D9A76B2C57AF136584A61E0DBB210B2918F18A` |

`latest.json` 版本为 `1.2.3`，URL 指向本次公开 updater ZIP，内嵌签名与 `.sig` 内容一致。Release 为非草稿、非预发布，目标提交与 annotated 标签一致。

## 公开更新与最终状态

真实证据位于被忽略的 `test-results/public-update-validation/20260902-144616/result.json`：安装位置、两套用户数据、开机自启、4 个经典菜单根、17 条命令、4 条快捷动作在 updater 清理窗口后及应用退出后均保持；资源目录只剩 `long_compress_shell_extension_1_2_3.dll`。当前机器最终安装公开 `1.2.3` 于 `E:\Long\Long解压`，相关应用进程为 0。

- [PR #115](https://github.com/Longyuyeee/long_Decompress/pull/115)
- [Release v1.2.3](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.3)
- [Release workflow](https://github.com/Longyuyeee/long_Decompress/actions/runs/33598489918)
