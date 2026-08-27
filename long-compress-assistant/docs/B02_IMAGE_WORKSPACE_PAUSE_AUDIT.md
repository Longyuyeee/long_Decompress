# B-02 图片压缩前端工作区收口审计

审计日期：2026-08-28

开发分支：`codex/archive-media-roadmap`

进入基线：`fc4397d`（B-01 图片依赖与固定哈希基线收口）

公开版本：`1.1.14`（本阶段不升版、不发布）

## 1. 收口结论

B-02 已完成前端工作区、压缩 store 内隔离的图片草稿集合、共享配置模型、明确的格式边界和 Windows 本地图片预览安全授权。隔离 Release/WebView2 双尺寸桌面矩阵与真实 Windows“选择图片文件”路径均已通过：系统对话框返回 JPEG/GIF 真实路径后，JPEG 显示 15,788 B、360×640 和原图预览，GIF 由统一业务规则明确拒绝且不入队，对话框关闭后 WebView 重新获得焦点。

本次提交只保存可审计的阶段成果和继续开发所需门禁，不提升版本、不创建安装包、不更新 Release。B-03 的真实编码、事务发布、实际输出和历史指标仍未开始，界面中的“开始处理”保持禁用，不创建模拟任务或占位成功结果。

## 2. 已实现范围

| 范围 | 当前结果 | 对齐判断 |
| --- | --- | --- |
| 压缩中心模式入口 | 保留归档压缩为默认，新增图片工作区；视频、PDF 仅显示诚实的计划状态 | 对齐，未伪装为可用功能 |
| 图片任务模型 | 在现有 `compression` store 内增加隔离的图片草稿集合；不新增媒体 store，也不污染归档草稿 | 对齐 B-00 统一任务边界 |
| 输入边界 | 接受 JPEG/JPG、PNG、WebP；GIF、PDF、BMP、TIFF 等明确拒绝 | 对齐 B-01 已审计范围 |
| 配置模型 | 有损/无损、质量、尺寸、输出格式、元数据、输出目录和冲突策略；批量与单项覆盖使用同一模型 | 对齐 |
| 任务展示 | 文件名、应用方向后的原图可见尺寸、输入大小、输出格式、检查状态和预计范围；预计值明确不是实际输出 | 对齐 |
| 原图/结果区 | 原图通过受限本地资产协议预览；结果区明确等待 B-03 真实编码结果 | 部分完成 |
| 响应式布局 | 设计为无横向滚动、正常与窄窗口保持工作区可用 | Windows Release/WebView2 1100×720 与 760×560 已通过并人工检查 |
| 桌面验收桥 | 增加真实 JPEG/PNG/WebP 与 GIF/PDF 拒绝样本、队列隔离和双尺寸门禁 | 已实现并真实通过 |

## 3. 真实测试的预期—首次实际—修正

| 场景 | 预期 | 首次实际 | 已做修正 | 当前证据 |
| --- | --- | --- | --- | --- |
| 固定图片夹具读取 | 测试从仓库固定夹具读取真实属性和哈希 | 首轮路径依赖模块 URL，测试运行位置变化时不稳定 | 改为从仓库工作目录和冻结 manifest 读取 | 图片草稿聚焦测试通过 |
| Windows 桌面驱动 | Release 应用由与 WebView2 匹配的 EdgeDriver 操作 | 首轮启动前发现 `EDGE_DRIVER_PATH` 未配置 | 固定使用本机 Selenium 缓存中的匹配驱动路径 | 桌面门禁已能进入真实 Release/WebView2 |
| 本地原图预览 | JPEG/PNG/WebP 在 Tauri 页面中真实解码并显示尺寸 | 首次桌面运行三个文件均进入“无法读取”；失败截图保存在 `test-results/desktop-e2e/desktop-e2e-failure.png` | 启用 Tauri `protocol-asset`，默认 scope 为空；CSP 同时允许 Windows WebView2 实际使用的 `https://asset.localhost`；Rust 命令校验普通文件、1–128 MiB、魔数与扩展名一致后才单文件授权 | 三种允许格式均真实解码；Release 身份门禁锁定空默认 scope 与两种资产源 |
| EXIF 方向尺寸 | 列表尺寸应与用户看到的原图一致，同时不丢失编码校验事实 | Orientation=6 样本的 WebView 可见尺寸为 360×640，而旧门禁误按编码矩阵断言 640×360 | 将公开 `width/height` 定义为应用方向后的可见尺寸；编码矩阵 640×360 与 Orientation=6 分别保留为验证事实，夹具和指标契约同步加门禁 | 夹具属性、桌面尺寸与 B-01 编码基线语义一致 |
| 非图片拒绝 | GIF/PDF 不进入可处理队列并给出明确原因 | 单元门禁符合预期 | 无需修正 | 聚焦测试通过 |
| 队列隔离 | 在归档/图片模式间切换不丢失、不串队列 | 单元和组件门禁符合预期 | 无需修正 | 聚焦测试通过 |
| B-00 store 边界 | 媒体草稿必须进入现有压缩域，不能新建媒体任务 store | 暂停审计发现首稿新增了 `imageCompression` store，且旧门禁只匹配 `media` 文件名而漏检 | 图片草稿并入现有 `compression` store；架构门禁同时检查 `imageCompression` 命名并禁止新的 image/media store | 修正后架构门禁与聚焦测试通过 |

## 4. 本次已完成验证

- `npm.cmd exec vitest run src/stores/__tests__/imageCompression.test.ts src/stores/__tests__/compression.test.ts src/views/__tests__/CompressionView.test.ts`：3 个文件、33 个测试通过；
- `npm.cmd run test:media-architecture`：通过并实际检查 4 个媒体/图片生产文件；新增图片 store 会被失败关闭；
- `npm.cmd run type-check`：暂停提交前复验通过；
- `npm.cmd run test:e2e:desktop:image-workspace`：真实 Windows Release/WebView2 通过；JPEG/WebP/PNG 解码、GIF/PDF 拒绝、归档/图片队列隔离和双尺寸布局均通过；
- 人工检查 `image-workspace-1100x720.png` 与 `image-workspace-760x560.png`：正常与窄窗口均无横向溢出，配置/对比两列保持可读；生成截图仅作本地证据，不提交仓库；
- `cargo test image_preview_tests --lib`：1 个安全授权测试通过，覆盖真实 PNG、扩展名伪装和 GIF 拒绝；
- 浏览器本地页面已检查四模式入口、图片空工作区和计划状态；浏览器结果不代替 Windows 安装/Release 桌面结果；
- `npm.cmd run tauri build` 曾成功生成本地 `1.1.14` 测试安装包，仅用于建立真实桌面候选，不构成发布，也不会提交构建产物。

## 5. 保留边界与风险

恢复验证首先发现 Windows CRLF 门禁误报以及图片桌面门禁不必要依赖完整 FFmpeg 媒体夹具；两项已完成纠偏并通过真实图片基线，证据见 [B02_VALIDATION_INFRASTRUCTURE_AUDIT.md](B02_VALIDATION_INFRASTRUCTURE_AUDIT.md)。

1. 可见“浏览文件”入口已真实打开 Windows `#32770` 系统对话框，两个固定样本通过系统选择状态与标准 `IDOK` 返回产品；门禁未调用 `queueDesktopDialogSelections`。路径、字节、预览、GIF 拒绝、队列和焦点均通过，证据见 [B02_NATIVE_PICKER_PATH_AUDIT.md](B02_NATIVE_PICKER_PATH_AUDIT.md)。
2. 原图预览授权目前只允许 JPEG、PNG、WebP，且要求魔数与扩展名一致。这是有意的安全边界；若以后支持无扩展名图片或 TIFF，必须先更新依赖与样本审计，不能放宽成任意本地文件资产访问。
3. B-02 不执行编码，因此没有实际输出大小、压缩率、结果图或历史任务；这些只能由 B-03/B-04 的真实事实来源提供。
4. 本阶段不应提交 `target`、`dist`、测试截图或本地安装包等生成物。

## 6. 下一步严格顺序

1. B-02 已收口，下一步进入 B-03：只接入 B-01 已审计的 JPEG/WebP `libcaesium` 与无损 PNG `oxipng` 路径，并复用 B-00 共享发布事务。
2. B-03 必须生成可重新解码的真实输出并覆盖取消、失败、磁盘不足、目标竞态和“仅在更小时替换”；完成前保持执行按钮禁用，不创建模拟任务或占位成功结果。
3. 大节点 B 完整通过 B-03 至 B-05 后，才允许提升补丁版本并更新 Release 与安装包；B-02 收口本身不升版、不发布。

## 7. 需求与偏移审计

本阶段仍以归档压缩/解压稳定性为基线，没有修改现有归档执行路径；图片草稿复用现有压缩 store，并以独立集合避免污染归档草稿。审计发现并纠正了“新建图片 store 绕过 B-00 统一任务边界”、Windows 资产 CSP、方向尺寸语义和真实系统选择后大小为 0 B 四项偏移，也补强了对应静态与桌面门禁。视频/PDF 计划节点没有被伪装成完成，未经 B-03 发布事务审计的编码器也未提前进入产品运行时。B-02 现已收口，允许进入 B-03，但仍不允许升版或宣称图片压缩可用。
