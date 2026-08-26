# 压缩包浏览中心 A-03 审计

日期：2026-08-26

分支：`codex/archive-media-roadmap`

版本：`1.1.13`（大节点 A 未完成，本步骤不升版）

## 开发目标与完成结果

| 目标 | 实际结果 | 状态 |
| --- | --- | --- |
| Windows 默认应用打开 | 文件双击、Enter 与右键“使用默认应用打开”复用 `open_archive_entry` | 通过 |
| 安全条目边界 | 拒绝空路径、绝对路径、`..`、ADS、设备前缀、Windows 保留设备名、目录与链接 | 通过 |
| 隔离缓存 | `%LOCALAPPDATA%/LongDecompress/preview-cache/<session>/<random-id>/`，同名文件互不覆盖 | 通过 |
| 资源上限 | 单文件 64 MiB、单会话 256 MiB、64 项、24 小时 TTL | 通过 |
| 输出复核 | 打开前要求恰好一个普通文件，路径和归档元数据一致，大小完全一致 | 通过 |
| 互联网来源标记 | 复用事务解压并把源归档 `Zone.Identifier` 传播到缓存文件 | 通过 |
| 主动内容确认 | EXE/MSI/BAT/CMD/PS1/JS/VBS/LNK/SCR 等第一次调用不解压、不启动；对话框默认聚焦取消 | 通过 |
| 生命周期 | 应用状态释放时尽力删除本会话；占用导致删除失败时由下一次启动按 24 小时 TTL 重试 | 通过 |

## 预期与真实结果对照

| 场景 | 预期 | 首次实际 | 修正后实际 |
| --- | --- | --- | --- |
| 本地文件默认打开 | 安全提取后由 Windows 文件关联处理 | Tauri `shell.open` 的 URL 正则拒绝本地路径 | 改为校验后直接调用 `ShellExecuteW`；TXT、PNG、PDF 均被 Windows 接受 |
| 连续打开多格式 | 前一项完成后下一项可立即打开 | 测试只等待缓存生成，前端尚未执行 `finally`，下一按钮短暂禁用 | 同时等待字节落盘和可见“打开完成”状态；三种格式连续通过 |
| 密码输入清空 | 从加密 7Z 切回普通 ZIP 时为空 | WebDriver `clear()` 未稳定触发 Vue input | 派发真实 `input` 事件；普通 ZIP 不再残留前一密码 |
| 互联网来源 | 缓存文件保留 ZoneId=3 与来源信息 | 未在 A-03 专项中单独验证 | 真实 NTFS ADS 字节逐字一致 |
| 危险 CMD 取消 | 不解压、不执行 | — | 缓存无 CMD，`%TEMP%/long-a03-danger.marker` 不存在 |

## 验证证据

- `cargo test archive_entry_open --lib`：6 项通过；覆盖真实恶意 ZIP 路径、主动内容、文件复核、TTL 与会话释放。
- `cargo check --features desktop-e2e`：通过。
- `cargo build --release --features custom-protocol,desktop-e2e`：通过。
- `npm.cmd run type-check`：通过。
- `npm.cmd run test:unit -- --run src/views/__tests__/ArchiveBrowserView.test.ts`：完整前端单元集 37 个文件、224 项通过。
- `npm.cmd run test:e2e:desktop:archive-browser`：真实 Windows Release Tauri/WebView2 通过：
  - 中文八层长路径 ZIP 选择解压；
  - 加密 7Z 与固定加密 RAR 选择解压及字节/哈希校验；
  - 中文及空格文件名 TXT、PNG、PDF 的默认应用打开；
  - 三个缓存文件与归档源字节一致，三个 NTFS `Zone.Identifier` 与源归档一致；
  - CMD 未确认时没有缓存和执行标记。
- 可见截图：`test-results/desktop-e2e/archive-browser-a03-safe-open.png`（本地测试产物，不提交仓库）。

## 审计结论与下一步

A-03 已满足计划定义，可收口推送；没有提升版本或创建 Release，因为大节点 A 仍有 A-04 有界文本查看、A-05 嵌套归档和 A-06 安装版综合矩阵。下一步只进入 A-04，不提前引入图片/视频/PDF 压缩引擎。
