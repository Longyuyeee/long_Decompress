# B-02 图片系统选择路径审计

审计日期：2026-08-28

开发分支：`codex/archive-media-roadmap`

公开版本：`1.1.14`（本步骤不升版、不发布）

## 1. 结论

系统选择器入口的实际代码偏移已修正：`EnhancedFileDropzone` 过去只在测试桥或 Tauri 原生拖放时调用 `get_file_info`，真实 `dialog.open` 返回路径后却把大小保留为 `0`。现在文件/目录系统选择、测试桥和原生拖放统一读取磁盘元数据；图片工作区使用明确标题“选择图片文件”，并允许用户选中不支持格式后由统一业务边界给出 GIF 拒绝原因，而不是只在系统过滤器中悄悄隐藏。

可重复的 Windows Release/WebView2 门禁已通过真实工作区的可见“浏览文件”点击入口，验证 JPEG 路径、真实磁盘字节、方向后尺寸和预览，以及 GIF Toast 拒绝和不入队。该门禁只用测试桥替代系统对话框返回值，不冒充人工系统选择已完成。

## 2. 预期—实际—修正

| 检查 | 预期 | 首次实际 | 修正 | 当前证据 |
| --- | --- | --- | --- | --- |
| 系统选择后文件大小 | 使用磁盘元数据，不能显示 0 B | `dialog.open` 分支调用 `handleRawPaths(..., false)`，不执行 `get_file_info` | 所有原生路径统一执行 `get_file_info` | 组件测试验证 JPEG/GIF 两条路径与真实大小；桌面入口断言 JPEG 输入字节等于 `stat` |
| GIF 明确拒绝 | 用户可得到拒绝原因 | 图片选择器过滤 GIF，只有拖放能触发业务拒绝 | 图片工作区使用 unfiltered picker，由 `addImageCandidates` 统一拒绝 | 桌面入口显示包含文件名和 GIF 的错误 Toast，队列只保留 JPEG |
| 系统对话框交互 | 至少人工选入 JPEG 和 GIF，并检查焦点返回 | 自动化可打开、枚举并预选真实 `#32770` 对话框，但当前 Codex 宿主阻止后台进程完成受信任点击 | 所有实验性 Win32/UIA 调度代码均已撤回，不进入仓库 | 仍需一次有人值守的系统选择操作；完成前 B-02 不收口 |

## 3. 已通过命令

```powershell
npm.cmd exec vitest run src/components/__tests__/FileUpload.test.ts src/views/__tests__/CompressionView.test.ts
npm.cmd run test:unit
npm.cmd run type-check
npm.cmd run test:image-baseline
npm.cmd run test:media-architecture
npm.cmd run test:media-dependencies
npm.cmd run test:media-metrics
npm.cmd run test:media-release-gates
npm.cmd run test:release-identity
npm.cmd run test:e2e:desktop:image-workspace
cargo test --manifest-path src-tauri\Cargo.toml --lib
cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets --features custom-protocol,desktop-e2e -- -D warnings
```

全量前端回归为 42 个测试文件、243 项测试全部通过；Rust 为 300 项通过、0 失败、4 项需要外部真实样本或固定性能环境而按条件忽略，Clippy 零警告。最后一次桌面门禁使用清理实验钩子后重新构建的 `desktop-e2e` Release 二进制通过；图片夹具 5 个和 PDF 拒绝边界同步通过。测试截图和构建产物不提交。

## 4. 剩余人工步骤

1. 在 Windows Release 测试应用进入“压缩中心 → 图片压缩”，清空图片列表并收起批量设置。
2. 点击“浏览文件”，在真实“选择图片文件”对话框先选 `exif-orientation.jpg`，确认显示非零大小、360×640 和原图预览。
3. 再点击“添加文件”选择 `animated.gif`，确认 Toast 明确包含文件名与 GIF 拒绝原因，且 GIF 不进入队列。
4. 确认对话框关闭后焦点返回应用、键盘和鼠标仍可操作。记录结果后再运行 B-02 全量回归并收口；失败则继续修复，不得进入 B-03。
