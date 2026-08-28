# B-05.1 图片三格式真实样本矩阵审计

审计日期：2026-08-28（Asia/Hong_Kong）

开发分支：`codex/archive-media-roadmap`

公开基线：`v1.1.14`；B-05 整体尚未完成，本节点不升版、不更新公开 Release。

## 1. 目标与边界

B-05.1 只收口“每个公开图片格式至少 3 个真实样本”的可重复生产测试。公开格式按产品能力固定为 JPEG、PNG 无损和 WebP；样本覆盖小图、大图、透明语义和 EXIF Orientation 元数据，并保留真实三帧 GIF 的明确拒绝边界。每个输入冻结实际字节数与 SHA-256，必须调用生产 `image_compression_service::compress_single_image`，发布后再由独立 Pillow 解码器核对结果。

100 张混合批量、超大像素上限、中文长路径、冲突、磁盘不足和取消属于 B-05.2；安装版完整交互属于 B-05.3。本节点不虚构尚不存在的图片“删除源文件”产品选项。

## 2. 预期、开发前实际与修正

| 检查项 | 预期 | 开发前实际 | 修正后实际 |
| --- | --- | --- | --- |
| 格式样本数 | JPEG/PNG/WebP 各至少 3 个 | 旧 B-01 基线各 1 个，只证明候选编码器 | 新增独立 9 样本清单，实际计数 3/3/3；输入字节与 SHA-256 全部冻结 |
| 生产链路 | 样本经过应用真实服务和原子发布 | `run-image-baseline` 使用隔离工具，不等于生产服务 | 专用 Rust 测试直接调用 `compress_single_image`，9 个目标均真实发布 |
| 透明 WebP | 有损 WebP 保持 Alpha 且可重新解码 | 首轮实际发布前复验失败：无元数据候选仍被容器重写，VP8X Alpha 标志丢失，解码器拒绝 `ALPH` 块 | 编码结果已符合目标元数据状态时不再重写容器；透明 WebP 发布成功，Alpha 平面完全一致 |
| 质量比较 | 比较用户可见像素，并保留明确下限 | 统一 28 dB 将透明区域隐藏 RGB 计入，透明 WebP 被误报 21.454 dB；高对比 96×64 JPEG 实际 25.361 dB | 有损样本使用冻结逐样本下限；透明图先合成白底再比较可见 RGB，同时单独要求 Alpha 完全一致，实际透明 WebP 35.932 dB |
| PNG 无损 | 解码像素与 Alpha 不变 | 仅有 1 个透明 PNG 样本 | 80×60、256×256、2048×1280 三例像素完全一致，Alpha 语义一致 |
| 结果事实 | 格式、尺寸、方向、元数据、磁盘字节均来自实际输出 | 旧基线只覆盖三张 | 9 个输出的服务事实与文件 metadata 一致，独立重新解码差异为 0 |

## 3. 真实结果

| 格式 | 真实输入 | 输入 → 输出字节 | 质量/无损事实 |
| --- | --- | --- | --- |
| JPEG | `small-detail.jpg`、`exif-orientation.jpg`、`large-photo.jpg` | 3,265→2,071；15,788→7,701；242,609→55,696 | PSNR 25.361/41.223/42.717 dB，均高于各自冻结下限；Orientation 6 与元数据保留 |
| PNG | `opaque-small.png`、`transparent.png`、`large-alpha.png` | 384→223；1,546→601；18,323→8,312 | 三例解码像素完全一致；透明与不透明语义均保持 |
| WebP | `alpha-small.webp`、`photo.webp`、`large-photo.webp` | 278→1,160；3,884→3,600；11,850→11,772 | PSNR 35.932/44.029/47.809 dB；透明例 Alpha 完全一致。体积增加被如实记录，不冒充节省 |

结构化证据写入 `test-results/b05-image-format-matrix/result.json`，该目录为可再生成测试产物，不作为源码提交。冻结输入身份与验收阈值位于 `tests/fixtures/media/b05-image-format-matrix.json`。

## 4. 门禁与复现

核心真实命令：

```powershell
npm run test:image-matrix:real
```

该命令先生成并验证 11 个图片夹具及 PDF 拒绝边界，再检查 9 个矩阵输入的字节与 SHA-256，调用 Rust 生产服务，最后以 Pillow 对发布文件重新解码。成功条件固定为 JPEG/PNG/WebP 3/3/3、总计 9、解码差异 0。

静态媒体架构门禁同时锁定命令入口、9 样本计数、输入哈希和生产测试函数；媒体发布门禁新增 `three-samples-per-public-format` 必需案例，防止后续从发布矩阵中移除。

本节点最终质量结果：

- `npm run test:image-matrix:real`：9/9 真实生产压缩通过，JPEG/PNG/WebP 为 3/3/3，重新解码差异 0；真实三帧 GIF 被拒绝且不生成输出；
- `npm exec vitest run`：47 个文件、276 项通过；夹具扩展后首次运行发现旧接受清单滞后，已改为由真实 manifest 生成支持格式预期；
- `cargo test --lib`：318 项通过、4 项显式忽略、0 失败；
- `cargo clippy --all-targets --all-features -- -D warnings`：通过，0 警告；
- `npm run type-check` 与 `npm run build`：通过；
- 媒体架构、依赖、指标、发布契约和旧图片冻结基线全部通过；
- `git diff --check`：通过。

## 5. 审计结论与下一接续点

B-05.1 已满足需求：不是模拟测试，也不是只核对扩展名；9 个冻结真实输入全部通过生产编码、候选验证、原子发布和独立重新解码。测试实际发现的透明 WebP 容器破坏已在生产代码修复，质量比较口径也按可见效果纠正，最终预期与实际差异为 0。

下一步严格进入 **B-05.2 百图批量与故障边界**。B-05.2 完成并审计后，才进入 B-05.3 安装版拖入—配置—对比—执行—历史—重开输出完整流程；B-05 整体完成前不提升版本、不更新公开 Release。
