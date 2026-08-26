# 压缩包浏览中心 A-04 审计

日期：2026-08-26

分支：`codex/archive-media-roadmap`

版本：`1.1.13`（大节点 A 尚有 A-05、A-06，本步骤不升版）

## 开发目标与完成结果

| 目标 | 实际结果 | 状态 |
| --- | --- | --- |
| 图片预览边界不回退 | ZIP/TAR 图片继续执行 8 MiB、1600 万像素、单边 8192 像素和 TAR 64 MiB 扫描上限 | 通过 |
| 有界文本预览 | ZIP/TAR 系列最多读取解压后前 1 MiB，不为预览写入临时文件 | 通过 |
| 常用文本编码 | 支持 UTF-8/BOM、UTF-16LE/BE、GBK、Big5、Windows-1252；截断的多字节尾部安全裁剪 | 通过 |
| 二进制拒绝 | NUL 和异常控制字符触发二进制判定，不把伪装 `.txt` 注入 WebView 文本区 | 通过 |
| 格式边界 | 7Z 固实块和 RAR 没有严格有界读取器时内部预览保持禁用；A-03 默认应用打开仍可使用 | 通过 |
| 用户可理解性 | 弹窗与右键菜单说明“内存只读预览”和“隔离缓存后默认应用打开”的差异 | 通过 |
| 状态稳定 | 关闭预览不修改当前目录和文件选择；图片与文本复用同一键盘/右键动作 | 通过 |

## 预期与真实结果对照

| 场景 | 预期 | 首次实际 | 修正后实际 |
| --- | --- | --- | --- |
| 图片预览测试定位 | 图片内容和“只读、不落盘”元数据都可断言 | 弹窗抽象成通用容器后，旧测试仍只在图片节点查元数据 | 测试改为图片节点校验图像、通用弹窗校验元数据；全量 225 项通过 |
| Release 桌面启动 | WebView 加载打包后的 `dist` | 首轮手工 `cargo build` 漏传 `custom-protocol`，程序尝试访问 `localhost:1420` | 使用 `custom-protocol,desktop-e2e` 重建；真实 Release WebView2 门禁通过 |
| 普通中文文本 | 内部显示 UTF-8 内容且不生成 A-03 缓存 | 真实 ZIP 内容正常显示 | 缓存目录中不存在该文件，编码和“完整显示”状态正确 |
| 超大文本 | 页面只读取前 1 MiB并明确提示截断 | 真实 1 MiB + 4 KiB ZIP 日志显示边界 | UI 显示“仅显示前 1 MiB”，未加载完整内容 |
| 伪装二进制 | 不渲染、不冻结 | 真实 ZIP 内含 NUL 的 `.txt` 被后端拒绝 | 弹窗稳定显示“无法预览/appears to be binary” |
| 本地编码 TAR | 不依赖扩展名猜测，按 BOM/编码解码 | 真实 UTF-16LE TAR 正常解码 | 返回 `UTF-16LE`、完整中文内容、`truncated=false` |
| 7Z 文本条目 | 不绕过有界读取约束 | 真实加密 7Z 条目可浏览，但预览按钮禁用 | 默认应用打开和选择性解压能力不受影响 |

## 验证证据

- `cargo test archive_preview --lib --offline`：12 项通过；真实 ZIP UTF-8、TAR.GZ GBK、超限文本、伪装二进制、图片边界和 7Z 拒绝均覆盖。
- `cargo clippy --features desktop-e2e --offline -- -D warnings`：通过。
- `npm.cmd run type-check`：通过。
- `npm.cmd run test:unit`：37 个文件、225 项通过。
- `npm.cmd run build`：生产前端构建通过。
- `cargo build --release --features custom-protocol,desktop-e2e --offline`：通过。
- `npm.cmd run test:e2e:desktop:archive-browser`：真实 Windows Release Tauri/WebView2 通过；除 A-01 至 A-03 回归外，新增验证：
  - 真实 ZIP UTF-8 文本可见且内部预览不落盘；
  - 真实超大日志显示截断状态；
  - 真实伪装二进制 `.txt` 被拒绝；
  - 真实 UTF-16LE TAR 经桌面 IPC 正确解码；
  - 真实加密 7Z 内部预览保持禁用；
  - TXT、PNG、PDF 默认应用打开及危险 CMD 默认取消仍通过。
- 可见截图：`test-results/desktop-e2e/archive-browser-a04-text-preview.png`（本地测试产物，不提交仓库）。

## 审计结论与下一步

A-04 已完成并满足路线图验收目标，可以提交并推送。大节点 A 尚未达到发布条件，因此保持 `1.1.13`，不创建安装包或 Release。下一步进入 A-05：嵌套归档只读工作区、归档链、深度/累计预算、密码隔离和返回状态恢复；A-05 完成后再执行 A-06 安装版综合矩阵。
