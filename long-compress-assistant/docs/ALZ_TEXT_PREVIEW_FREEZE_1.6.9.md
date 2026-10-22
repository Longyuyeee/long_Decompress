# v1.6.9 ALZ 小文本预览封板记录

## 用户交付

ALZ 中的 TXT、日志、配置等常用文本可在原双栏直接预览，沿用眼睛按钮、双击、右键、Ctrl+Enter 和现有只读弹层。保持单条展开、多选弹窗、左右等宽、各栏位置/选择和工具中心；没有增加常驻配置区。图片能力单独控制，ALZ 图片和连续阅读仍不开放。

helper 0.5.0 新增 `preview-text`，精确选择一条普通文件，在内存接收完整输出；解码不超过 1 MiB，压缩输入不超过 64 MiB，主应用与 helper 均约束。沿用既有 Store/Deflate/Bzip2 解码、完整大小/CRC、结束标记与尾随检查，解码算法未改。大文本提供提取指引，不把前缀显示成整文件已校验；无关条目只读目录，不解码、不尝试其密码，不创建临时提取文件。

## 密码、失败与来源

沿用保险箱、手动密码及显式编码。命令先约束所选范围，再尝试密码；使用现有浏览取消登记处理“取消先于登记”、关闭或切换预览，过期结果不会显示。所选 helper 读取有 30 秒期限。来源版本在格式探测前及读取前后核对，无法确认时提示关闭预览并刷新此栏；CRC、非文本、错误密码等仍保留各自原因。

受控 Store/Deflate 样本、UTF-8/GBK/UTF-16、空文件、1 MiB 边界、CRC/二进制负样与无关方法 3 条目由[生成器](../scripts/prepare-alz-text-169.py)构造，身份见[清单](../tests/fixtures/archives/alz-text-169/manifest.json)，不作为原工具互通证据。Bzip2 使用原有 `alz-152/bzip2.alz`；密码单包及中间原生卷使用 v1.5.12 保留的原 ALZip Deflate 来源 `alz-1512/originals/m1-key-single` 与 `m2-key-volumes`。不外推历史 Bzip2 密码/卷型或方法 3 支持。

## 本轮依据与限定接续

- 四项不同 Rust 针对性检查通过。首次来源变化检查发现先解析损坏尾部会掩盖变动原因，仅修正该路径并续跑失败项；没有重新运行已过解码组合，日志保留首次失败及修复接续。
- 45 项不同视图检查与一项能力检查通过。新增 ALZ 内部预览、关闭取消/忽略迟到结果、来源变化友好提示；同时修正 v1.6.0 文案收纳后遗留的旧断言。按失败项接续，汇总不把过滤项计为通过，见[前端汇总](evidence/alz-text-preview-1.6.9/frontend-checks.json)。
- helper 直接调用的输出/输入边界均拒绝；输入负样实际承载 64 MiB + 1 字节受控数据，不是仅修改头部声明。见[边界记录](evidence/alz-text-preview-1.6.9/helper-boundaries.json)。
- 六组实际桌面完成：原双栏文本与图片门槛、原 Bzip2、原 ALZip 中文密码保险箱/中间卷及错密、大文本提取提示、来源变化/提前取消/新预览、大包中仅取 70 字节说明。最后一项沿用 v1.6.8 受控大包（无关条目压缩数据 282,956,568 字节），本轮预览约 0.54 秒，仅作该样观察，不承诺统一性能。
- 1100×650、760×520 三张真实截图均已实看，双栏等宽、关闭按钮可见、无横向溢出。见[桌面收据](evidence/alz-text-preview-1.6.9/desktop.json)及[宽窗](evidence/alz-text-preview-1.6.9/small-text-wide.png)、[窄窗](evidence/alz-text-preview-1.6.9/small-text-narrow.png)、[大文本提示](evidence/alz-text-preview-1.6.9/oversize-text-narrow.png)。

桌面首次候选 SHA-256 为 `d4d468e949148e1628a1eae2c670cb431264bb57e78ee3b375ebeb4a0fe105bc`，前四组和截图在此候选完成。来源变化失败后，只将来源核对前移到格式探测之前，并整理该错误的展示文案；未改布局、helper 或正常预览路径。最终候选 `aa1841484dd4d5f30be07082f8932afa84bc892580c052d129eee4628594139c` 只续跑来源/取消/新预览及大包所选读取，收据逐项标记应用身份，保留此前未受影响的通过依据。helper 两候选均为 `dcbe8ff673ff375ad5ea3eb129da4ce6e98515a418ccb6ba72272cf7ecb6c58a`。

普通类型/Vite 构建、Shell 与八处版本身份已过，最终原生候选编译完成。本地桌面为 `custom-protocol,desktop-e2e`、opt-level=1/LTO=false；正式打包保持发布配置。只补本轮相关增量，旧算法、独立大包完整回解、真实盘满和安装依据沿用，不重复铺开。

## 发布与后续

已达到可发版程度。README、版本身份、中文 Release、支持口径及交接同步；按正常受保护 PR / Windows package 检查 / 合并 / v1.6.9 标签发布，正式源码与流水线身份在本版 PR 补充。前版 v1.6.8 正式运行 [39823005872](https://github.com/Longyuyeee/long_Decompress/actions/runs/39823005872) 成功，安装器、更新包、签名及清单四附件齐备。

下一版 [v1.6.10 ALZ 图片预览](ARCHIVE_ROADMAP_1.6.10.md)仅规划。大文本前缀、历史原工具 Bzip2 大包、ALZ 方法 3 与 EGG 加密固实分别保留后续条件，未实现部分不扩大口径。
