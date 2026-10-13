# v1.4.3 归档升级实现与检查记录（开发暂停）

## 2026-10-13 恢复开发：标准 ZIP 多卷读取

用户已授权恢复并推进到发布。标准 `.z01/.../.zip` 的共享只读读取层已接通浏览、图片/文本预览、密码验证与原生密码解压；按 ZIP/ZIP64 的磁盘相对偏移定位原文件，只重建目录元数据，不合并复制载荷。终卷目录声明的卷数用于识别缺失的最高编号卷，避免误报完整。

标准 ZIP 定向 3 项通过（显式包含外部样本检查）：跨卷中央目录、任意卷浏览/预览、ZipCrypto/AES 正误密码、ZIP64 尾记录及缺失最后 `.zNN`；外部 WinRAR 7.23 生成真实多卷，并由随包 7-Zip 独立检查通过。卷组检测原有 5 项通过。数字卷回归发现 `open_seven_zip` 的错误分类前缀曾被重构丢失，已恢复原有语义，随后复核。ZIP64 当前证据为小载荷强制 ZIP64 目录，不宣称完成全部 >4 GiB 物理档案组合。

桌面使用链与最后 RAR 同名来源/默认格式刷新复核仍在接续，尚未据此给最终封板结论。格式参照 [PKWARE ZIP specification](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT)；复现外部样本用 `scripts/standard-zip-fixtures.mjs`，原生显式检查设置 `LONG_STANDARD_ZIP_FIXTURES` 后运行 `cargo test --manifest-path src-tauri/Cargo.toml --lib standard_zip -- --include-ignored`。

基线：已发布 v1.4.2，主线 `fa611a8`。实施分支：`codex/archive-compatibility-upgrade`。本轮主要功能已实现，用户已要求暂停，尚未最终封板；正式 Release 仍为 v1.4.2，未创建 v1.4.3 标签。

当前状态及剩余工作见 [暂停审计](DEVELOPMENT_PAUSED_AUDIT_2026-10-13.md)。本记录描述已经发生的实现和检查，不代表已经发布。

## 已实现范围

| 用户任务 | 实现 |
| --- | --- |
| 导入多卷，避免逐卷重复处理 | 共享卷组描述与名称处理，转换按组去重、选择实际首卷、显示缺卷并保留配置；标准 ZIP 主包也检测关联卷 |
| RAR 保留原包目录 | 排除编码器的绝对来源基目录，转换不会带入临时目录；真实目录和空目录保留 |
| 接着处理历史/待处理任务 | 卷组来源及最初选中的参考卷写入草稿/历史；启动前按所选任务重读卷组，新来的右键任务不混入或拖住当前批次 |
| 创建和调整密码 RAR 分卷 | 普通压缩、RAR 批量格式、转换、编码器检测、固实/文件名加密、真实卷清单及事务发布共同接通 |
| 中文 RAR 密码读写一致 | 本地 `unrar 0.5.8` Rust 适配采用宽字符密码回调，打开、浏览、密码验证和回解不经过旧本地编码转换；公开 API 不变；卷切换只读取实际 NUL 结尾名称，修复越界读取 |
| 更多适合分享的格式 | LZ4/Brotli 单文件、TAR.LZ4/TAR.BR 多文件/目录，创建/浏览/解压及 TAR 转换目标 |
| TAR 与单文件流分卷 | TAR/TGZ/TBZ/TXZ/TZST、GZ/BZ2/XZ/ZST 与新格式的数字切卷、跨卷读取、完整性校验和产物发布 |
| 避免解压后难以打开 | 单文件默认名称保留 `.txt` 等原后缀，数字卷回解也保持正确名称 |
| 截断、取消与兼容处理 | TAR 消费压缩流尾部以暴露 CRC/截断问题，大条目逐块检查取消；数字卷支持四位编号和很小首卷；兼容引擎升级 26.04 |
| 熟悉且清晰的界面 | 单条行内展开、多选配置弹窗、自然默认值继承；新格式只扩展已有控件，LZ4 固定快速模式不显示可调强度 |

RAR 编码器不随软件分发。已有密码命令行说明沿用本版的原有交互，临时确认不写入历史配置。RAR 的完整卷组使用 `.partN.rar`；TAR/流格式的数字切卷不是这些格式的原生卷标准。原格式没有密码时，不承诺其原生加密能力。

## 与范围对应的检查

- 前端针对转换、草稿、解压/压缩中心、配置控件的回归通过；新建任务实际参数覆盖单文件原后缀、RAR 密码分卷、TAR 新格式及 AES 密码缺失提示。
- Rust 原生路径检查真实载荷：LZ4/Brotli/TAR 组合及九种已有 TAR/单文件流，分卷、发布、回解、中文和空目录，逐字节比较；同时覆盖取消、缺卷、四位编号、截断尾部、LZ4 校验和与产物冲突。核心分组 32 项通过；新增预览与编解码分组 16 项通过（含重叠用例，不相加计数）。
- 真实 RAR 编码器检查：WinRAR 7.23 临时解出的 Rar.exe，无密码/内容密码/文件名密码三种 RAR5 分卷；应用内回解、归档浏览、正确/错误密码验证。密码包含中文、空格、符号和 emoji；使用隔离测试保险箱；追加同名来源回归，RAR 自动区分包内名称，不覆盖另一来源。
- 浏览器交互检查 4 项通过：RAR 卷组、TAR.BR 批量修改、单条展开、多选弹窗、草稿和双密码恢复；760/1100 宽度无横向溢出，截图已查看。
- 前端生产构建、Windows Release 编译与 Windows 原生菜单 DLL 构建通过，版本标识统一为 1.4.3。Windows 11 可选身份包在本地没有签名环境时按既有行为跳过。
- 真实 Windows Release Tauri 检查通过：ZIP → 带 Unicode 文件名密码的 RAR 分卷、TAR.LZ4 分卷、TAR.BR 分卷、TGZ 分卷 → 回解 → 合卷 ZIP → 再回解；LZ4/Brotli 单文件创建—回解，均逐字节匹配随机非空载荷，中文目录/空目录保留，原包 SHA-256 不变。具体结果位于 `test-results/desktop-e2e/archive-upgrade-results.json`。

相关命令：

```powershell
npx vitest run src/stores/__tests__/archiveConversion.test.ts src/stores/__tests__/pendingDrafts.test.ts src/views/__tests__/CompressionView.test.ts src/views/__tests__/DecompressView.test.ts src/components/compression/__tests__/CompressionSettingsPanel.test.ts src/utils/__tests__/compressionFormat.test.ts
cargo test --manifest-path src-tauri/Cargo.toml --lib -- codec_stream::tests numeric_volume_reader::tests split_archive_detector::tests output_publish_transaction::tests compression_verification::tests compression_format::tests split_compression::tests
# RAR 显式实测需要在 PATH 放置外部 Rar.exe；不会安装或修改用户编码器。
cargo test --manifest-path src-tauri/Cargo.toml --lib rar_native_volumes_roundtrip_with_content_and_header_passwords -- --ignored --nocapture
$env:PLAYWRIGHT_PORT='30123'
npx playwright test e2e/batch-conversion-drafts.spec.ts --project=chromium --workers=1 --reporter=line
# 使用隔离桌面测试数据；仅执行本次新增归档链。
node scripts/test-tauri-desktop.mjs --archive-upgrade-only
```

不重复运行未受影响的安装、卸载、媒体或完整外部格式矩阵。ARJ/Z/UDF 公开读取、原生标准 ZIP 多卷创建、AES 包装数字分卷继续作为后续候选，不阻塞本版，也不提前宣传为新增功能。既有 ZIP/7Z 密码分卷与 7Z 文件名加密不重复作为新增能力。

原始范围与来源见 [升级提案](ARCHIVE_COMPATIBILITY_AND_WORKFLOW_UPGRADE_PROPOSAL_2026-10-13.md)，用户说明见 [更新说明](RELEASE_NOTES_1.4.3.md)，引擎来源和哈希见 `src-tauri/resources/archive-engine/SOURCE.md`。
