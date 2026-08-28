# B-00.6 媒体安装态与 Release 门禁审计

日期：2026-08-27

分支：`codex/archive-media-roadmap`

公开版本：`1.1.14`（前置门禁不升版、不发布）

## 1. 结论

B-00.6 已完成。`config/media-release-gates.json` 固定 B/C/D 各节点的公开格式、真实安装场景、输出复核、失败注入、旧版恢复和公开更新要求；`scripts/check-media-release-gates.mjs` 已接入 Release 身份门禁。发布证据统一使用 [MEDIA_NODE_RELEASE_EVIDENCE_TEMPLATE.md](templates/MEDIA_NODE_RELEASE_EVIDENCE_TEMPLATE.md)。

本节点复用现有 `test-installed-release.ps1` 的用户数据备份、覆盖安装、EXE 字节核对、生产启动、菜单核对、卸载和前一版本恢复能力，没有新增第二套安装器或回滚实现。

## 2. 节点门禁

| 节点 | 公开范围 | 必须补齐的节点特有证据 |
| --- | --- | --- |
| B 图片 | JPEG、WebP、无损 PNG；GIF 明确保留或拒绝 | EXIF 方向、Alpha、WebP、100 文件批量、9600 万像素上限、真实解码复核 |
| C 视频 | H.264 MP4、H.265 MP4 | VFR/AAC/旋转/字幕、H.265、长任务进程树取消、中文长路径、纯音频拒绝、FFprobe 复核 |
| D PDF | PDF 结构优化 | 矢量、扫描、透明、表单保持、签名拒绝、加密拒绝、qpdf/page/render 复核 |

每个节点共同执行六类失败：处理中取消、引擎非零退出、损坏输出、发布前目标竞争、磁盘不足、系统回收站失败。成功发布前不得删除源文件；失败后不得留下最终半成品或暂存目录。

## 3. 预期—实际—修正

| 检查 | 预期 | 首次实际 | 修正 | 最终实际 |
| --- | --- | --- | --- | --- |
| 证据边界 | 发布前和发布后都有统一、机器可验证的证据 | 现有生成器只聚合已发布 GitHub Release/更新证据，无法约束节点开发期的真实格式和故障矩阵 | 增加 B/C/D 可执行契约与统一证据模板，并接入 Release 身份检查 | 静态门禁通过，缺格式/回滚/证据字段会失败 |
| 无证书边界 | 允许普通无签名 NSIS，不允许无签名原生第一层菜单包 | 既有文档分散描述 | 在契约中固定 `unsignedWindowsBuildAllowed=true`、`unsignedNativeContextMenuPackageAllowed=false` | 与当前无商业证书事实一致 |
| 真实安装回滚 | 当前正式安装可覆盖、运行、卸载并恢复，用户数据不变 | 首次完整运行即通过，无产品差异 | 无产品修正 | `1.1.14 → 1.1.14` 基线 44/44，通过后恢复 `E:\Long\Long解压` 和版本 1.1.14 |

本机忽略证据：`test-results/installed-release-validation/20260827-164731/result.json`。脚本复核 `succeeded=true`、失败项 0、44 项检查、恢复安装位置存在；不能用该 B-00 基线替代未来 B/C/D 的媒体操作矩阵。

## 4. 版本与 Release 决策

- 只有完整用户可见大节点通过，才提升补丁版本；内部前置、半成品 UI 或单引擎实验不升版。
- 正式 NSIS 安装态必须使用生产桥；测试注入、忽略项、空文件和估算指标不能算通过。
- 合入 master 后才打标签；Release 四项资产、updater 签名、`latest.json` 和上一正式版本公开更新完成后，节点才最终关闭。
- 没有商业 Authenticode 证书是已知边界，不阻断普通 NSIS；但继续阻断 Windows 11 原生第一层菜单身份包。

## 5. 下一步

B-00.1 至 B-00.6 均已完成。下一步执行 B-00 总审计；通过后进入 B-01，只做图片引擎依赖、固定输入哈希、安装增量和质量/性能基线，不提前实现 B-02 页面。
