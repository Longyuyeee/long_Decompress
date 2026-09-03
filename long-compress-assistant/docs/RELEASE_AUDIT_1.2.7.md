# Long解压 v1.2.7 发布审计

日期：2026-09-03

分支：`codex/decompression-table-layout`

## 发布范围

- 解压任务名称、状态对齐、自然排序、存储预检和低高度版本徽标。
- 全局省略文本完整提示与动态内容刷新。
- 解压中心、压缩中心、特殊压缩的紧凑标题和工作区留白。
- 双栏文件浏览器在窄窗口中固定左右排列。

## 关键预期—实际

| 门禁 | 预期 | 候选实际 |
| --- | --- | --- |
| 解压名称与状态 | 名称区域可用、终态居中、活动态轨道稳定 | Chromium 与真实 Tauri 通过 |
| 自然排序 | 中文名称中的数字按数值排序 | 单元测试通过 |
| 存储预检 | 关键容量/介质信息完整且 760×520 不被遮挡 | 真实 Tauri 截图通过 |
| 省略文本 | 仅真实溢出时显示全文，不覆盖已有标题，动态更新 | 单元测试通过 |
| 核心工作区密度 | 顶部 ≤17 px、标题 ≤58 px、底部 ≤17 px | Chromium 920×620 三页面通过 |
| 双栏布局 | 920、760、390 px 均左右排列且根容器无横向溢出 | Chromium 三档、真实 Tauri 两档通过 |

## 候选验证

- 类型检查：通过。
- 前端单元：52 文件、290/290 通过。
- 前端集成：6/6 通过；性能门禁：17/17 通过。
- 跨浏览器 E2E：45 通过、20 项按项目条件跳过、0 失败；Chromium 桌面子集 13/13 通过。
- 真实 Windows Tauri 文件浏览器门禁：通过。
- 真实 Windows Tauri 920×620 / 760×520 响应式门禁：通过。
- Rust Release：主程序与集成共 495 通过、14 项专用环境门禁显式忽略、0 失败。
- 主程序与 Shell 扩展严格 Clippy：`-D warnings` 通过；Shell 扩展 5/5 通过。
- 生产前端构建：通过，产物中不存在桌面 E2E 桥。
- 正式 NSIS 候选构建：通过；没有安装或替换用户正在运行的正式版。

## 本机候选产物

| 产物 | 字节 | SHA-256 |
| --- | ---: | --- |
| `Long解压_1.2.7_x64-setup.exe` | `19,401,605` | `F29806225A5C4E94A25C1219F1ACC352F2648BCE90DB8390DEF963F6B7552558` |
| `Long解压.exe` | `29,775,872` | `B46FE9E6E57B165A5FEC546A02AD4BF7D49FB0BCF0F96364CFAEE5D6C882228F` |
| `long_compress_shell_extension_1_2_7.dll` | `246,784` | `9E1D926FF49E821EC9F455CA6A22606BF74CE7C7EBC8A4BD41F15CDB61BEB1AC` |

本机候选只用于身份与可构建性审计；正式公开安装包和 updater 资产必须由 tag 触发的干净 GitHub Actions Runner 重新生成并回下载校验。

## 正式发布状态

- [PR #120](https://github.com/Longyuyeee/long_Decompress/pull/120) 的五项 CI 全部通过，并以 merge commit `c4d2adc2c1df1237d3674dc7ac76f6507c6979e4` 合入 `master`；annotated tag `v1.2.7` 精确指向该提交。
- [Release workflow 33739220209](https://github.com/Longyuyeee/long_Decompress/actions/runs/33739220209) 于 2026-09-03 成功完成。正式 [Long解压 v1.2.7](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.7) 已公开，且不是 draft 或 prerelease。
- 四项公开资产已回下载并逐项核对：

| 公开资产 | 字节 | SHA-256 |
| --- | ---: | --- |
| `latest.json` | `932` | `070649E4B5C55004433F883E710A39BA3AA4E4C91FF2AD47DF6727F901E84BEE` |
| `Long-Decompress_1.2.7_x64-setup.exe` | `19,373,672` | `6494562BD76D53C801C062123EACAC78B78002513CAB529BF894A9D4451E8449` |
| `Long-Decompress_1.2.7_x64-setup.nsis.zip` | `19,373,830` | `E02663023F47933145C3A8EBB77A637009394925E524529DE04899B503311E16` |
| `Long-Decompress_1.2.7_x64-setup.nsis.zip.sig` | `428` | `60B932AC890254402F0AA12D565BC0362AD685B2852553A18045F552F7BC5F7D` |

- `latest.json.version` 为 `1.2.7`；Windows x86_64 URL 精确指向本标签的 updater ZIP；manifest 内签名与独立 `.sig` 的 428 字节内容逐字一致。GitHub API 公布的四项资产摘要与回下载 SHA-256 全部一致。
- 本机没有执行会停止或替换用户正式版的安装/升级生命周期，因此不把该项写成通过；它不影响由干净 GitHub Runner 生成并公开的本次发布资产结论。

`v1.2.7` 的功能、回归、CI、版本身份、标签、Release 和公开资产对账至此全部关闭。下一开发节点从最新 `master` 重新立项，不从旧候选分支接续。

## 明确边界

- Windows 11 第一层资源管理器菜单需要生产代码签名证书；无签名发布不生成 MSIX。
- Windows N 仍缺少真实支持证据。
- 本机安装生命周期会停止并替换用户正在运行的正式版；候选阶段不执行该破坏性门禁，交由干净 CI Runner 构建发布资产。
