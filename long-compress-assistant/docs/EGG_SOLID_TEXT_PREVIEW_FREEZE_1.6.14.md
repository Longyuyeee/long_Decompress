# v1.6.14 EGG 小型固实文本封板记录

## 用户交付与范围

未加密 Deflate / LZMA 固实文本沿原双栏、双击/眼睛/右键/Ctrl+Enter 打开原弹层。保留滚动、关闭、另一栏状态、单条展开、多选弹窗、50/50 双栏、工具中心和主题；范围说明继续放在原问号内，没有新增浏览中心或常驻表单。

helper 0.6.0 为 preview-text 增加固实分支，复用完整固实解码与文件/块路由，仅把所选文本收集到内存。所选文本不超过 1 MiB，所有共享块压缩数据、解码总量和所有普通文件声明总量分别不超过 64 MiB；非所选数据仍参与 CRC/大小/终止检查但不保留，不创建临时提取目录。所有依赖通过后才返回，空文件同样检查完整共享流。实际读取计数须与共享压缩载荷相符；预算不是整个进程总内存承诺。

主应用先识别共享依赖，对加密固实在读取保险箱候选前给提取指引；普通固实即使保险箱未初始化也可读。复用预览整体 30 秒、关闭取消与读取前后来源核对，缺卷/来源变化/损坏各自保留原因。现有非固实文本/图片、完整解压/检查/转换与解码器成功条件保持，不开放固实图片、加密固实、EGG 创建/修复或大文本前缀。

## 来源与增量检查

普通固实原件沿用 egg-153/upstream/solid.egg（Deflate）和 egg-154/original-alzip/solid-deflate.egg / solid-high.egg（Deflate / LZMA），所选新预览字节对照既有独立 SHA-256；不重复整包回解。覆盖原中文路径、说明和空文件。

[生成器](../scripts/prepare-egg-solid-text-1614.py)构造共享后段 CRC、结束/尾随、所选 1 MiB 与完整依赖 64 MiB 范围、未知方法和其他文件持有加密头等受控样本。[清单](../tests/fixtures/archives/egg-solid-text-1614/manifest.json)区分来源。三成员分卷使用未改变的原 Deflate 共享载荷加受控原生卷头；不是原工具直接创建的分卷，不扩大历史互通。原工具在当前工作区不可用，未重复下载/执行安装器；已有原密码/卷型证据沿用。真实超过 64 MiB 的输入文件只在忽略目录生成并在解码前拒绝，不跟踪大载荷。

- 4 项新增 Rust 通过：原 Deflate/LZMA 中文和空文件、精确 1 MiB；共享后段 CRC/截断/尾随/依赖超限/未知方法/其他文件加密；受控中间卷/取消/成员变化与缺卷；锁定保险箱时的普通固实读取和加密固实提前拒绝。见[日志](evidence/egg-solid-text-preview-1.6.14/rust.log)。1097 项其余检查筛出，不冒称全量运行。
- 2 项不同新增前端通过：原完整预览和共享范围问号、加密固实提取提示无部分内容。首次脚本断言尚未展开的悬停帮助，只补鼠标进入并重跑失败的 1 项；通过项保留，产品未变。见[汇总](evidence/egg-solid-text-preview-1.6.14/frontend-checks.json)，未把筛出项计入通过。
- 13 项新 helper 操作通过：4 项固定原件文本/空文件字节、1 项受控中间卷与原载荷哈希、8 项依赖/大小/方法/加密/结束拒绝。见[收据](evidence/egg-solid-text-preview-1.6.14/helper.json)和[脚本](../scripts/audit-egg-solid-text-1614.py)。
- 4 组实际桌面通过：原弹层滚动及宽窄等宽；原中文 LZMA 与受控中间卷正式命令；共享后段 CRC/加密固实友好提示与超限；取消/来源变化及新结果接续。见[收据](evidence/egg-solid-text-preview-1.6.14/desktop.json)。
- 3 张已实看：[1100×650](evidence/egg-solid-text-preview-1.6.14/solid-text-wide.png)、[760×520](evidence/egg-solid-text-preview-1.6.14/solid-text-narrow.png)、[760×520 原中文 LZMA](evidence/egg-solid-text-preview-1.6.14/original-lzma-narrow.png)。文本可滚动、关闭和底部信息可见，无页面横向溢出，双栏等宽。

桌面候选 SHA-256：1f2cf8f380e824461209a828071deb8320b7a67f7cca73292448a4617658a703；helper SHA-256：3a0310cb1d1507c3c427f8daba4f4e08b7b5072760f539bbed1ac58d919fd467。helper、类型/Vite、Shell、最终桌面编译与八处版本身份/引擎清单通过，见[构建记录](evidence/egg-solid-text-preview-1.6.14/checks.json)。本地桌面使用 custom-protocol,desktop-e2e、opt-level=1/LTO=false；正式发布配置保持。未复跑旧算法矩阵、安装或真实盘满。

## 发布与后续

已达到可发版程度。README、Release 和支持口径与本版身份同步，通过正常受保护 PR / Windows package / 正常合并 / v1.6.14 标签发布，发布身份在本版 PR 接续。[v1.6.15 小型固实图片](ARCHIVE_ROADMAP_1.6.15.md)已接续开发，非固实大文本另拆。ALZ 方法 3、原工具图片/历史 Bzip2 大包、EGG 加密固实继续保留来源条件。

前版 v1.6.13 已由 [PR #274](https://github.com/Longyuyeee/long_Decompress/pull/274) 合并并推送标签，源码/合并树一致，必需 Windows package 成功；截至本版封板记录，[正式运行 39848948261](https://github.com/Longyuyeee/long_Decompress/actions/runs/39848948261)仍在打包，附件未确认生成，同树 master CI 39848876365 已确认取消，后续状态在发布回执接续。
