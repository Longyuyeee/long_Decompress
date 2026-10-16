# v1.4.14 封板记录

最新状态（2026-10-17）：本轮功能与证据已随 PR #169 完整合入主线，PR #168 自动标为合入；与 v1.4.15 合并交付，不单独发布 v1.4.14。正式标签、安装包与发布接续见 [v1.4.15 封板记录](ARCHIVE_PERFORMANCE_FREEZE_1.4.15.md)。下方及[暂停交接](DEVELOPMENT_PAUSED_1.4.14_2026-10-16.md)保留当时快照，不覆盖此次恢复和合入；范围、独立回解及实际页面仍见[实施记录](ZIP_OUTPUT_METHODS_1.4.14.md)。

## 本版范围

- ZIP 可选 Zstandard，保留兼容 ZIP 默认行为。
- ZIPX 使用 XZ 创建，空文件 Store，提供普通包及 AES / ZipCrypto 密码数字分卷。
- 单条/分组/批量压缩、转换、配置组、任务模板、快速预设、草稿与历史保留方法；分析使用对应方法。
- 单条展开、多选弹窗、左右等宽与两列滚动保持。

## 收口状态

- [x] 功能与版本 1.4.14 接入。
- [x] 相关前端、后端定向检查与前端构建；版本来源和 Shell DLL 一致。
- [x] 15 组真实压缩、3 个转换与独立回解；3 个未开放组合明确拒绝；12 张截图已查看。
- [x] 候选 `16106b4f49f39d0788dd48d8e860563589c03b6b` 的 [Windows package / 38878587465](https://github.com/Longyuyeee/long_Decompress/actions/runs/38878587465) 成功，17:20:18（香港时间）结束；[安装器附件 12036523615](https://github.com/Longyuyeee/long_Decompress/actions/runs/38878587465/artifacts/12036523615) 为 `windows-nsis-installer`，20,166,373 字节。

暂停时提出取消，构建已结束，因此没有取消成功或重跑。上述结果对应 `16106b4`，暂停文档提交使用 `[skip ci]`，不另触发检查。[PR #168](https://github.com/Longyuyeee/long_Decompress/pull/168) 保持未合并，正式标签、Release 与更新附件尚未创建；恢复与发布待用户后续指令。

代码冻结提交：`5227fb15fd8f25b100e656ffa1495a6111ae212e`。此后只整理候选状态文档，实际归档、界面和已通过检查保持。候选提交完整身份见 PR。

## 实际程序与证据

- 实际 Windows 桌面为 Debug / `desktop-e2e` 程序，最终 ZIPX 与转换页面程序 SHA-256：`7ec6575e94a6a314c6e02c6e4923960c86a28b0c1752b5d019295dd5700bd3b4`。这不是安装器哈希，也不宣称是安装后 Release 程序验证。
- Shell DLL：`long_compress_shell_extension_1_4_14.dll`，SHA-256 `59a878d0c278a43f2f1438a35a2c2c64ebcf40739928c7b9b8a78e4a71ec46b3`；八处版本来源均为 1.4.14。
- [最终桌面结果和几何数据](evidence/zip-output-1.4.14/zip-output-1414-results.json)、[原生阶段](evidence/zip-output-1.4.14/native-results.json)、[空 XZ 诊断](evidence/zip-output-1.4.14/zip-output-1414-xz-diagnostic.txt)。原生及单条阶段在较早构建已通过，后续仅界面帮助/配置范围变化，复用记录包含在结果与 [首次日志](evidence/zip-output-1.4.14/zip-output-1414-desktop.txt)、[原生接续](evidence/zip-output-1.4.14/zip-output-1414-final.txt)、[页面接续](evidence/zip-output-1.4.14/zip-output-1414-ui-final.txt)中，不将最终页面哈希套用于所有早期产物。
- 页面后仅修正非 ZIP 混合草稿的继承，相关三项与最终生产前端构建通过；布局和原生代码未改，原结果继续复用。

本地复现使用 `VITE_DESKTOP_E2E=1` 的前端与带 `desktop-e2e` 特性的 Windows 程序；运行 `node scripts/test-tauri-desktop.mjs --zip-output-only` 前显式设置 `TAURI_APP_BINARY` 指向该程序。可用 `LONG_ZIP_OUTPUT_RESUME_ROOT` 复用已保存阶段、`LONG_ZIP_OUTPUT_UI_FROM=zipx` 接续剩余页面，避免重跑通过的检查。完整步骤见脚本和实施记录。

## 保留边界

ZIPX 标准多卷创建暂不提供；用户可以选择 ZIP 标准分卷或 ZIPX 数字分卷。未扩大其他 ZIPX 专有算法、任意缺卷救援或大包性能承诺。Windows N 暂不保证支持。
