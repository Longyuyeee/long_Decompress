# B-02 图片工作区验证基础设施审计

日期：2026-08-27

分支：`codex/archive-media-roadmap`

进入基线：`7cfd572`（B-02 图片工作区暂停 checkpoint）

公开版本：`1.1.14`（本步骤不升版、不发布）

## 1. 结论

B-02 恢复验证前发现并修正两类测试基础设施偏移：Windows CRLF 使图片依赖锁门禁误报，以及图片桌面门禁错误依赖完整视频/PDF/FFmpeg 夹具。现在图片门禁只准备 B-02/B-01 真正需要的五个固定图片和一个真实 PDF 拒绝样本；完整媒体夹具命令仍保留视频、PDF、FFmpeg 与 Poppler 全矩阵，没有降低 B-00 证据。

本步骤没有修改产品图片工作区、归档压缩执行路径、媒体引擎集成状态或版本。B-03 仍被冻结。

## 2. 预期—首次实际—纠偏—最终实际

| 场景 | 预期 | 首次实际 | 纠偏 | 最终实际 |
| --- | --- | --- | --- | --- |
| 图片锁文件门禁 | Windows 与 CI 使用同一 Cargo.lock 事实 | `core.autocrlf=true` 时锁文件为 CRLF，脚本固定匹配 LF，误报 `libcaesium lock entry is missing` | 读取后仅规范化换行再匹配，依赖名称、版本和禁止项检查保持不变 | `test:image-baseline` 与 `test:release-identity` 通过 |
| 图片桌面夹具准备 | 只准备 JPEG/PNG/WebP、GIF 和非图片拒绝样本 | `test:e2e:desktop:image-workspace` 调用完整媒体夹具，必须先下载 170,676,191 B GPL FFmpeg 测试工具 | 新增 `--images-only`；只安装固定 Pillow、生成五图片和真实 PDF 拒绝样本并复核字节、SHA-256、属性与 PDF 魔数 | 图片夹具 9 秒内完成，不接触 FFmpeg、视频或 PDF 优化引擎 |
| 完整媒体工具下载 | 中断不留下可误用缓存，停滞可恢复 | 首次只写入 467,520 B 后无网络活动，脚本无限等待且直接写最终文件 | 改为 `.part`、Range 续传、60 秒无数据超时、四次重试、进度输出、固定大小/SHA-256 后原子改名 | 已验证部分文件只保留为 `.part`，图片门禁不再被该外部下载阻塞；完整 B-00 命令继续使用同一固定资产 |
| 需求范围 | B-02 不提前执行编码 | 为了准备图片 UI 样本间接拉取视频测试工具，扩大了本步骤外部依赖 | 将 `test:image-baseline:real` 和 `test:e2e:desktop:image-workspace` 都绑定图片专用夹具命令 | B-02 只验证图片工作区；完整媒体基线仍由 `test:fixtures:media` 承担 |

## 3. 实现边界

- `scripts/check-image-baseline.mjs` 只消除换行差异，不放宽版本或 feature 锁。
- `scripts/generate-media-fixtures.py --images-only` 复用同一图片生成函数，不建立第二份图片真相源。
- 图片专用 PDF 仅验证“真实非空 PDF 被输入边界拒绝”，不作为 D 节点 PDF 优化样本或能力声明。
- `scripts/prepare-media-test-fixtures.mjs` 的完整模式行为保持：固定 FFmpeg、完整视频/PDF事实、Poppler 渲染和差异比较仍必须执行。
- 下载中的 `.part` 和所有生成夹具位于忽略的 `test-results`，不会进入安装包或提交。

## 4. 验证结果

- `npm.cmd run test:fixtures:media:images`：5 图片属性、固定字节/SHA-256和真实 PDF 魔数通过；
- `npm.cmd run test:image-baseline:real`：JPEG/WebP/PNG 真实编码与重新解码、GIF 拒绝通过，峰值工作集 10,309,632 B；
- `npm.cmd run test:release-identity`：通过；
- 图片/压缩/视图聚焦 Vitest：3 文件、33 项通过；
- `npm.cmd run type-check`：通过；
- 媒体架构、依赖、指标、Release 门禁：全部通过；
- Node 与 Python 脚本语法检查、`git diff --check`：通过。

## 5. 下一步

按 [B02_IMAGE_WORKSPACE_PAUSE_AUDIT.md](B02_IMAGE_WORKSPACE_PAUSE_AUDIT.md) 继续运行隔离 Release/WebView2 双尺寸桌面门禁并人工检查两张截图；随后补一次原生选择或拖放。两项均通过后才能把 B-02 状态改为完成。
