# B-01 图片依赖与固定哈希基线审计

日期：2026-08-27

分支：`codex/archive-media-roadmap`

公开版本：`1.1.14`（B-01 是隔离实验，不升版、不发布）

## 1. 结论

B-01 已完成。隔离工具 `tools/image-baseline` 使用 `libcaesium = 0.21.0` 且仅启用 `jpg,webp`，使用 `oxipng = 10.2.0` 且仅启用 `parallel,zopfli`。Cargo.lock、feature tree、73 个注册表包的 SPDX 元数据和真实构建均已检查；没有 gifski、imagequant 或独立 GPL/AGPL 依赖进入实际候选树。

候选仍为 `integrationAllowed=false`，没有进入 `src`、`src-tauri`、正式 EXE 或 NSIS。B-03 接入共享发布事务时才测最终 NSIS 增量；当前只报告隔离静态候选相对最小 Rust 进程的载荷增量，不把它冒充最终安装包数字。

## 2. 固定输入

五个合成输入连续重新生成两次，字节数和 SHA-256 完全一致：

| 输入 | 字节 | SHA-256 | 用途 |
| --- | ---: | --- | --- |
| `exif-orientation.jpg` | 15,788 | `291f614dd1832bea94b884f116a9063c5d3bfa0fbe089ba8fcb9e1e7aa32aa0d` | JPEG、EXIF Make、Orientation=6 |
| `photo.webp` | 3,884 | `12403feaf86d9322fcdd3867e9d7fef641c17ce770503697f5e4d72b1b3ffef1` | WebP 渐变和锐利边缘 |
| `transparent.png` | 1,546 | `008a670aced1d3446a6dca8dc232164b1269c652a98524a1f8de1ec024106632` | PNG Alpha 与逐像素无损 |
| `animated.gif` | 7,224 | `47670a2220f659fdfd0ace11b99a500aabaa79bc841b92827fe6076259fe962d` | 三帧 GIF 拒绝边界 |
| `ultra-large.png` | 1,617,665 | `af894765a2a03d446e0b30f0ceaa97fca7c52c9c4412948721866a21a1f843f7` | 12000×8000、9600 万像素资源上限 |

清单位于 `tests/fixtures/media/image-baseline.json`，静态门禁会拒绝空文件、哈希漂移或 GIF/超大图边界改变。

## 3. 真实输出与性能事实

| 路径 | 输入→输出 | 结构/质量复核 | 单次引擎耗时 |
| --- | ---: | --- | ---: |
| JPEG quality 80 | 15,788→7,701 B | 640×360；EXIF Make 与 Orientation=6 保持；PSNR 41.223 dB | 21.564 ms |
| WebP quality 80 | 3,884→3,600 B | 800×500；PSNR 44.029 dB | 24.262 ms |
| PNG 无损 preset 3 | 1,546→601 B | 256×256；透明语义保持；RGBA 解码逐像素一致 | 15.421 ms |
| GIF | 7,224→0 B | 0.490 ms 明确拒绝；没有输出文件 | 0.490 ms |

最终合格运行中，当前机器单个冷候选进程连同三项处理的墙钟时间为 121.97 ms，峰值工作集 10,702,848 B（约 10.2 MiB）；四个独立进程并发完成时间为 191.47 ms，4/4 成功。数值是本机基线，不是跨机器性能承诺。

隔离候选 EXE 为 2,872,320 B，最小 Rust EXE 为 117,760 B，原始增量 2,754,560 B；以仓库固定 7-Zip ZIP `-mx=9` 压缩后增量 1,077,127 B。最终 NSIS 增量保持 `null`，必须在 B-03 正式集成时重新测量。

## 4. 预期—实际—修正

| 检查 | 预期 | 首次实际 | 修正 | 最终实际 |
| --- | --- | --- | --- | --- |
| oxipng 构建 | 锁定 API 可编译 | 10.2.0 的输出路径为 `Option<PathBuf>`，成功值为大小元组，旧调用编译失败 | 按锁定版本 API 传 `Some(path)` 并显式映射成功值 | Release 构建通过 |
| Cargo 元数据 | JSON 可直接解析 | Cargo 下载提示来自 stderr，与 stdout JSON 拼接后解析失败 | stdout 作为机器数据，stderr 只用于失败诊断 | 73 包许可与精确版本可解析 |
| 许可证阻断 | 禁止 GPL/AGPL，允许 LGPL 许可表达式 | 初版子串正则把 `LGPL-2.1-or-later` 误报为 GPL | 按 SPDX 运算符边界匹配独立 GPL/AGPL 标识 | 无禁止许可；r-efi 的可选 LGPL 不再误伤 |
| PNG Alpha | 无损保持透明像素 | oxipng 合法改写为索引色+tRNS，显式 A 通道不存在，但 RGBA 像素一致 | 透明验收改为 A 通道或透明表，并继续要求 RGBA 逐像素一致 | Alpha=true，pixelsIdentical=true |

## 5. 可复验命令

```powershell
npm.cmd run test:image-baseline
npm.cmd run test:image-baseline:real
npm.cmd run test:media-dependencies:real
npm.cmd run test:release-identity
npm.cmd run test:media-architecture
```

真实结果写入忽略目录 `test-results/image-baseline/result.json`，包含输入/输出哈希、解码属性、PSNR、耗时、内存、并发与候选载荷；`differences` 必须为空。

## 6. 需求对齐与下一步

- 对齐用户要求的真实测试：三种公开写入格式均真实编码并重新解码，不使用空文件或改扩展名；GIF 真实拒绝。
- 对齐现有能力：候选仍隔离，未复制任务、历史、事务、系统回收站或安装更新逻辑。
- 没有提前扩大格式：TIFF、HEIC、GIF 动画编码仍不公开。
- 下一步进入 B-02 前端工作区，只实现图片模式的任务与配置交互；真正执行仍要等 B-03 接入共享事务后才能宣称可用。
