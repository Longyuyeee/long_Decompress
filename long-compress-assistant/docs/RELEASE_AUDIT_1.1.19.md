# Long解压 v1.1.19 发布审计

> 审计日期：2026-09-01  
> 状态：正式发布关闭

## 目标对齐

本版本只收口用户明确要求的文件浏览方向：原“压缩包浏览”默认页改为可访问电脑目录的双栏文件管理器，提供跨栏文件操作；原安全归档工作区继续作为压缩包操作，不重写归档引擎。版本由 `1.1.18` 提升 `0.01` 至 `1.1.19`。

## 候选证据

| 门禁 | 结果 |
| --- | --- |
| 八处版本身份与唯一 Shell DLL | `1.1.19` 一致，`long_compress_shell_extension_1_1_19.dll` 唯一 |
| Rust 真实文件系统 | 3/3；复制、移动、BLAKE3、属性、冲突零落地和边界阻断通过 |
| 相关 UI / 归档回归 | 41/41；Browser shell Chromium 9/9 |
| 全量单元 | 273/273；失败 0 |
| TypeScript / Rust / Vite | 全部通过 |
| 无测试桥本地 NSIS | `Long解压_1.1.19_x64-setup.exe` 已生成；本机无签名环境，未生成 Windows 11 identity package |
| 隔离 Release 真实桌面聚焦门禁 | 通过；两个文件栏可见，真实 IPC 复制/移动/属性均精确得到 2 文件、2 目录、9 B |
| 旧安全归档工作区 | 单元 17/17；真实桌面复验已通过 18 万条目取消、导航、ZIP/7Z 精确选择解压和内部文本预览；默认应用缓存等待环境段未完成，未计全绿 |

## 正式发布闭环

- [x] PR [#111](https://github.com/Longyuyeee/long_Decompress/pull/111) 五项 CI 全绿，以 merge commit `f003ac675b250cf9c3f923ca5fbdb9905d6b5932` 合入受保护 `master`。
- [x] annotated `v1.1.19` 标签指向上述合并提交并已推送。
- [x] Release workflow [33472694078](https://github.com/Longyuyeee/long_Decompress/actions/runs/33472694078) 成功，生成 NSIS、updater `.nsis.zip`、`.sig` 和 `latest.json`；无签名环境按预期跳过 Windows 11 identity package。
- [x] [v1.1.19 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.19) 为非草稿、非预发布，四项公开资产均已回下载。
- [x] `latest.json` 版本为 `1.1.19`，Windows URL 指向公开 updater ZIP，清单签名与 `.sig` 逐字一致；ZIP 内唯一安装包与独立 NSIS 的 SHA-256 完全一致。
- [x] 真实执行公开 `v1.1.18 → v1.1.19` 应用内更新，24/24、失败 0；验证安装路径、两套用户数据、4 个菜单根/17 条命令/4 条快捷动作、自启动和旧 DLL 替换。
- [x] README 已改为“当前公开稳定版”，补录最终提交、工作流和公开资产证据。

## 公开资产回下载证据

| 资产 | 字节 | SHA-256 |
| --- | ---: | --- |
| `latest.json` | 1,069 | `CAB228220CE2DF419EC08378A353C7FC8FEFBDD361F82694430FC345052047CC` |
| `Long-Decompress_1.1.19_x64-setup.exe` | 19,370,630 | `DD50BB999A1AB706CF022B6ED1CAF8C59270AA046BAC89FF8653951618B10D05` |
| `Long-Decompress_1.1.19_x64-setup.nsis.zip` | 19,370,790 | `CFF6B5BD56518A61FD4E5D2525CAEDDA0790BE3E1426C308C77AC8A2EF91F1AE` |
| `Long-Decompress_1.1.19_x64-setup.nsis.zip.sig` | 428 | `279FB4AF83A6A5A62E297EF5CA23E9AF83C7D6FBFFBDFE7B43D301602BBF8764` |

公开更新证据保存在被忽略的本机目录 `test-results/public-update-validation/20260901-133638/result.json`。更新后安装版本与主程序产品版本均为 `1.1.19`，安装位置保持 `E:\Long\Long解压`，唯一 Shell DLL 为 `long_compress_shell_extension_1_1_19.dll`，验证结束相关应用进程为 0。

## 结论

`v1.1.19` 已完成需求、真实文件操作、受保护主线、公开资产和真实应用内更新闭环，可以作为当前公开稳定版。实现与安全边界见 [双栏文件浏览器审计](DUAL_PANE_FILE_BROWSER_AUDIT.md)。Windows N 仍无实机证据；无代码签名环境不宣称 Windows 11 第一层身份菜单支持。
