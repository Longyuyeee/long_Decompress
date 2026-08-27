# B-00.3 媒体依赖身份与许可审计

日期：2026-08-27

分支：`codex/archive-media-roadmap`

公开版本：`1.1.14`（本工作项不升版、不发布）

## 1. 结论

B-00.3 已收口的是“可信候选与失败关闭门禁”，不是媒体压缩能力。`config/media-dependencies.json` 固定图片、视频、PDF 候选的精确版本、官方来源、字节数、SHA-256、许可证、链接/进程边界、Windows 平台、禁用功能、安装体积测量阶段和安全更新责任；`test:media-dependencies` 在字段缺失、非 HTTPS、来源主机越界、哈希缺失或运行时被提前启用时失败，CI 和 Release 已有的版本身份门禁会先执行该检查。

当前四项候选仍为 `integrationAllowed=false`，生产源码和资源中也没有媒体引擎。安装包增量无法在尚未集成时伪造：清单保留实测候选载荷，并将最终 NSIS 增量设为后续候选构建的强制测量项；字段为空即表示不得发布该引擎。

## 2. 候选决策

| 工作负载 | 固定候选 | 已验证事实 | 当前边界 |
| --- | --- | --- | --- |
| JPEG/WebP | `libcaesium 0.21.0` | crates.io 归档 33,105 B，SHA-256 与注册表校验一致，归档包含 Cargo 元数据和许可证 | 仅允许未来以 `default-features=false` 评估 `jpg,webp`；默认/GIF/PNG 功能被门禁禁止 |
| 无损 PNG | `oxipng 10.2.0` | crates.io 归档 76,578 B，SHA-256 一致，MIT 许可证存在 | 与有损 PNG 分离；只能在 B-01 实际构建后填写安装增量并放行 |
| 视频 | `FFmpeg 9.0.1` 官方源码 | 官方 `tar.xz` 12,036,420 B；SHA-256 固定；WSL GnuPG 使用官方发布密钥验证 detached signature 为 Good signature，指纹为 `FCF986EA15E6E293A5644F10B4322F04D67658D8` | FFmpeg 不提供官方 Windows 运行时；C-01 必须可复现构建最小 LGPL 运行时，禁止 GPL、nonfree、x264、x265，完成前阻断 |
| PDF 结构优化 | `qpdf 12.4.0` 官方 MinGW64 | 官方 ZIP 24,063,155 B，SHA-256 与上游 checksum 文件一致；真实 `qpdf.exe` 返回 12.4.0，crypto 为 OpenSSL/native；最小运行子集 12,637,211 B | D-01 必须随包保留 Apache/NOTICE 和 GCC 运行时义务，并以真实 NSIS 候选测量压缩后增量 |
| PDF 重渲染 | Ghostscript | AGPL/商业许可超出当前再分发边界 | 明确拒绝，不得作为隐式后备 |

顶层许可证不能代表传递依赖。真实审计发现 `libcaesium` 默认功能会带入 AGPL-3.0-or-later 的 `gifski`，PNG 路径会带入 GPL-3.0-or-later 的 `imagequant`。因此原先“libcaesium 统一覆盖图片”的设想已纠正为 JPEG/WebP 与 MIT 无损 PNG 两条显式路径；动图保持原文件或拒绝处理，直到另有独立许可和质量方案。

## 3. 预期、首次实际、修正与最终实际

| 检查 | 预期 | 首次实际差异 | 修正 | 最终实际 |
| --- | --- | --- | --- | --- |
| crate 内容 | 直接列出 Cargo 元数据与许可证 | `.crate` 的第一层是 gzip，7-Zip 首次只显示内层 tar 流，门禁正确失败 | 真实解开第一层后再列出 tar，不用扩展名臆测内容 | 两个 crate 均验证 Cargo.toml 和 LICENSE |
| FFmpeg 来源 | 固定官方源码且验证发布者签名 | Windows 环境没有原生 GnuPG，单纯固定 `.asc` 哈希不能证明签名有效 | 使用本机真实 WSL GnuPG、隔离临时 keyring、核对官方指纹后验证源码 | `ffmpeg-9.0.1.tar.xz` 返回 Good signature；临时 keyring 已删除 |
| qpdf 二进制 | 不只相信 ZIP 名称，要实际启动 | 官方完整包 24.1 MB，不能直接等同安装包增量 | 提取运行子集，逐文件计量并真实执行 `--version`、`--show-crypto` | 版本 12.4.0、OpenSSL/native；运行子集精确 12,637,211 B |
| 失败关闭 | 缺身份信息必须阻断 | 只写文档无法阻止后续误接入 | 增加四组内存负向控制并扫描生产源码/资源 | 缺哈希、缺许可、HTTP 来源、提前启用均被拒绝；生产中 0 个媒体引擎 |

## 4. 可复验命令

```powershell
npm.cmd run test:media-dependencies
npm.cmd run test:media-dependencies:real
npm.cmd run test:media-architecture
npm.cmd run type-check
```

真实命令会下载固定上游文件到被忽略的 `test-results/media-dependency-audit`，逐字节和 SHA-256 校验，并执行 FFmpeg PGP 与 qpdf 运行时验证。CI/Release 使用不依赖网络下载的静态门禁；正式发布某个媒体引擎前，还必须在发布审计中附上同版本的 `:real` 结果与真实安装包增量。

## 5. 需求对齐与下一步

- 没有新增媒体页面、伪进度或占位成功状态；压缩、解压、浏览、保险箱和历史主线未改变。
- 不是“先选库再补许可”，而是将来源、许可、功能开关、再分发和体积作为进入实现前的门槛。
- B-00 尚未整体完成，因此不升版本、不打包、不创建 Release。
- 下一步为 B-00.4：建立无隐私、可再生成并带精确预期属性的真实图片、视频、PDF 固定样本；首次实际结果必须与预期逐项比较，不能用空文件或仅扩展名样本代替。
