# B-04.1 图片输入/输出事实契约审计

审计日期：2026-08-28（Asia/Shanghai）

开发分支：`codex/archive-media-roadmap`

公开基线：`v1.1.14`；本节点不升版、不生成 Release，不启用图片执行按钮。

## 1. 开发目标与边界

B-04.1 只解决图片命令、前端响应类型和持久化历史之间缺少输入/输出双侧事实的问题。目标是让后续 B-04 编排只消费 Rust 对真实源文件和真实候选输出重新解码得到的事实，不使用浏览器预览尺寸或预计体积冒充历史结果。

本节点不增加阶段事件，不实现批量、冲突策略和终态队列，不修改现有禁用按钮，也不扩大 GIF、视频、PDF 或删除源文件范围。

## 2. 预期、实际与修正

| 检查项 | 预期 | 修改前实际 | 本次修正与复核 |
| --- | --- | --- | --- |
| 图片命令结果 | 同时返回输入与输出的真实格式、字节、编码矩阵、可见尺寸、方向、帧数和 Alpha | `ImageCompressionFacts` 只表达输出侧，输入事实已在服务内解码但未返回 | `ImageCompressionOutcome` 改为明确的 `input/output`；结果不更小时返回 `input/candidate`，没有丢失已验证候选事实 |
| 前端契约 | 与 Rust camelCase 序列化完全同构 | 没有 `compress_image_file` 的请求/响应类型 | 新增 `ImageCompressionRequest/Facts/Outcome`；仍不提前调用命令 |
| 历史指标 | 旧历史继续可读，新图片历史能保存双侧事实 | `MediaMetricsV1` 只有单组通用宽高、容器和 Alpha | 保留旧可选字段，新增可选 `media.image.input/output`；Rust 同步 `deny_unknown_fields`，限制格式、非零尺寸/帧数和方向 1–8 |
| 指标来源 | 只允许后端重新解码事实 | 指标门禁只声明单侧 `width/height`，未禁止浏览器尺寸成为历史来源 | `media-metric-sources.json` 明确双侧 16 项事实及 `never-browser-preview`，机器门禁逐项检查 |
| 格式化门禁 | 本节点改动可审计且不夹带历史改写 | 全仓 `cargo fmt --check` 会报告大量本节点之外的既有格式差异 | 未批量格式化仓库；使用聚焦 Rust 测试、全量 Clippy `-D warnings` 和 `git diff --check` 控制本次质量 |

## 3. 真实测试证据

固定真实输入存在且非空：

| 文件 | 字节 | SHA-256 | 实际复核 |
| --- | ---: | --- | --- |
| `exif-orientation.jpg` | 15,788 | `291F614DD1832BEA94B884F116A9063C5D3BFA0FBE089BA8FCB9E1E7AA32AA0D` | 输入编码矩阵 `640×360`、可见尺寸 `360×640`、方向 6；同格式输出保持双侧事实；转 PNG 后方向归一为 1且可见尺寸不变 |
| `transparent.png` | 1,546 | `008A670ACED1D3446A6DCA8DC232164B1269C652A98524A1F8DE1EC024106632` | PNG 输入/输出均保持 Alpha；转 JPEG 时输入 Alpha=true、输出 Alpha=false |
| `photo.webp` | 3,884 | `12403FEAF86D9322FCDD3867E9D7FEF641C17CE770503697F5E4D72B1B3FFEF1` | 结果不更小时返回真实 `input/candidate` 且不发布目标文件 |

真实图片服务 9 项全部通过，覆盖 JPEG/PNG/WebP、方向、透明度、格式转换、缩放、GIF 拒绝、扩展名伪装、取消、仅更小策略、目标竞争和暂存清理。历史聚焦 8 项通过，覆盖旧 JSON 默认 `archive`、严格未知字段拒绝、双侧图片事实保存和无效方向拒绝。

全量结果：

- TypeScript 类型检查通过；
- 前端 42 个文件、244 项通过；
- 生产前端构建通过；
- Rust `--lib` 312 项通过、4 项显式忽略、0 失败；
- `cargo clippy --all-targets --all-features -- -D warnings` 通过；
- 媒体架构、依赖、指标、发布和固定图片基线门禁全部通过；
- `git diff --check` 通过。

## 4. 审计结论与下一接续点

B-04.1 已完成，需求与真实代码对齐。双侧事实已经可由命令返回并可安全进入后续历史模型，但当前前端仍未调用图片命令，按钮继续禁用，不能把本节点描述为用户可执行功能。

下一步严格进入 **B-04.2 阶段事件**：在图片命令中建立解码、可选缩放、编码、验证、发布的真实阶段日志；编码器没有可信字节回调，因此不生成平滑百分比、速度或 ETA。之后才进入 B-04.3 安全批量编排。
