# C-01.2.1 视频运行时准入审计

审计日期：2026-08-28

范围：冻结 FFmpeg 产品资源、许可/来源载荷、后端字节身份与能力预检、长期真实视频夹具和未签名 NSIS 包内回读。正式安装执行、Windows N 分类、同提交签名 NSIS/updater 精确增量属于 C-01.2.2。

结论：**C-01.2.1 已满足准入目标；C-01 尚未整体关闭，视频入口仍禁用，版本保持 1.1.15。**

## 1. 目标与需求对齐

| 需求 | 实现与真实结果 | 状态 |
| --- | --- | --- |
| 产品运行时必须来自 C-01.1 可复现候选 | `ffmpeg.exe` 12,349,440 B / `35c3c8bb...8672eb`，`ffprobe.exe` 12,131,840 B / `2c1df07c...ba1d98`，与双构建候选一致 | 通过 |
| 许可、来源和构建信息随包交付 | 安装包含 LGPL 2.1/3、MinGW-w64、GCC Runtime Library Exception/版权原文，以及 `SOURCE.txt`、`BUILD-CONFIGURATION.txt` | 通过 |
| 缺失或替换必须在执行前拒绝 | 后端对 8 个资源先检查普通文件、大小和 SHA-256，再以参数数组直接启动进程；缺失和篡改负向测试均通过 | 通过 |
| 能力不能只靠文件名或版本声明 | 真实执行 `-version`、`-encoders`、`encoder=h264_mf`、`-filters`；确认 9.0.1、LGPL-only、h264_mf、AAC、软件默认和五个必需过滤器 | 通过 |
| 视频夹具必须长期稳定且是真实容器 | 两个跟踪 MP4 分别覆盖 H.264/VFR/AAC/90°/字幕和 H.265/无音频；逐字节锁定并由产品 `ffprobe.exe` 探测 | 通过 |
| 安装包必须读取自身载荷 | 真实 NSIS 完整性测试、解包、8/8 身份核验、包内 ffmpeg/ffprobe 执行和真实 MP4 探测，差异 0 | 通过 |
| 正式安装和签名增量不得伪造 | `compressedInstallerDeltaBytes` 保持 `null`；本节点只记录跨提交未签名聚合测量，正式结论留给 C-01.2.2 | 对齐 |

## 2. 预期、实际、差异与修正

| 项目 | 预期 | 首次实际 | 修正 | 最终实际 |
| --- | --- | --- | --- | --- |
| 视频夹具来源 | 可长期重复执行 | 旧 BtbN nightly 资产连续 4 次 HTTP 404 | 提交两个合成真实容器并锁定字节；准备脚本复制后使用产品 ffprobe 探测 | 11 图片、2 视频、6 PDF 完整矩阵通过 |
| 法律载荷 | 分发所涉声明完整 | 首版资源只有 FFmpeg LGPL 文本，缺 MinGW/GCC 声明 | 补入上游 MinGW-w64 copyright 和 GCC Runtime Library Exception 原文，并纳入后端/NSIS 哈希门禁 | 8/8 包内资源一致 |
| 安装体积 | 取得真实测量且不越权命名 | C-01.1 的隔离 LZMA2 约 3.74 MB 不能代表产品安装包 | 下载父提交 CI NSIS 作明确基线，重建当前未签名 NSIS；保留正式签名字段为空 | 8,658,819 B → 15,554,236 B，聚合增加 6,895,417 B |
| Rust 全回归 | 0 失败 | 321 通过、1 失败、4 忽略；watch-folder 测试预期 1 个稳定文件，实际偶发 2 个 | 去除依赖线程调度的 30ms 写入竞态，在第一次快照后由测试钩子同步改写，生产判定不放宽 | 定向 10/10；完整 322 通过、0 失败、4 个环境型忽略 |
| Rust 格式门禁 | 本节点代码格式正确 | 全仓 `cargo fmt --all --check` 暴露约 17,000 行历史格式差异 | 不制造无关全仓改写；对本节点新增 Rust 和改动的 task-template 文件单独检查 | 本节点文件通过；全仓历史格式债务未在本节点冒充已解决 |

## 3. 实际安装包测量

- 父提交：`705fcdd5b828ca3edd7113568e2b406365ad5287`，GitHub Actions run `33152716382`；NSIS 8,658,819 B，SHA-256 `c1754d0c...9cf04`。
- 当前未签名本地 NSIS：15,554,236 B，SHA-256 `6e2f4efb...4c3fd`；相对父提交聚合增加 6,895,417 B。
- 运行时 8 文件展开总量：24,631,334 B。`test:video-runtime-package:real` 的机器报告位于忽略目录 `test-results/video-runtime-package/result.json`，其中保存完整 expected、actual、differences。
- 上述差值同时包含本节点代码/资源及本地构建条件差异，不能替代“同提交、同配置、签名 NSIS/updater”的 C-01.2.2 测量。

## 4. 验证清单

- `npm run test:fixtures:media`：11 图片、2 视频、6 PDF 通过；
- `npm run test:media-dependencies:real`：6 个锁定依赖、FFmpeg PGP 和产品资源身份通过；
- `npm run test:video-runtime-package:real`：8 个包内资源、真实能力和真实 MP4 探测通过，差异 0；
- `cargo test --lib`：322 通过、0 失败、4 个明确环境型忽略；
- `npm run test:unit`：44 文件、254 测试通过；
- `cargo clippy --all-targets -- -D warnings`：通过；
- `npm run type-check`、`npm run build`、`npm run test:release-identity`、媒体架构/依赖静态门禁：通过；
- `npm run tauri build`：真实未签名 NSIS 构建通过，签名环境缺失按既有策略跳过 Windows 11 identity package。

## 5. 下一接续点

C-01.2.2 只完成剩余合规闭环：从正式安装目录调用生产预检和真实软件转码；缺失/替换安装文件拒绝；Windows N 无 Media Foundation 的稳定分类；同一提交、同一配置的无/含 FFmpeg 签名 NSIS 与 updater 精确差值。通过前不启用视频 UI、不提升版本、不创建 Release。随后才进入 C-02 探测与配置模型。
