<div align="center">

<img src="../icon.png" width="120" height="120" alt="Long解压软件图标">

# Long解压 · 开发说明

Vue 3 / TypeScript / Pinia / Tauri 1.8 / Rust / SQLite

[软件介绍与界面](../README.md) · [下载安装](https://github.com/Longyuyeee/long_Decompress/releases/latest) · [开发路线](docs/PRODUCT_ENHANCEMENT_ROADMAP.md) · [开发交接](docs/DEVELOPMENT_HANDOFF.md)

</div>

---

Long解压将归档压缩、批量解压、文件与压缩包浏览、密码管理和媒体体积优化放进同一桌面工作区。界面围绕用户的文件与任务组织：单条点击后行内展开，所选任务集中批量配置，按状态找到待处理和成功结果，批量修改前查看所选清单与改动摘要；完成结果直接打开或定位，失败后可带入原配置，调整后继续处理。

## 功能概览

| 工作区 | 用户可以做什么 |
| --- | --- |
| 解压中心 | 导入多包、配置目录与子文件夹、使用密码候选、处理文件冲突、查看阶段和结果 |
| 压缩中心 | 按用途快捷配置（兼容分享/体积优先/加密分享/按大小分包）、文件/文件夹/组归档、ZIP 方法选择（Deflate/仅存储 Store/Zstandard/BZip2/PPMd/LZMA）、7Z 方法选择（LZMA2/LZMA/Bzip2/Deflate/PPMd/仅归档，LZMA2/LZMA 可选 Delta 及 x86 / ARM32 / Thumb / ARM64 / RISC-V BCJ 预处理）、ZIP/7Z/RAR 与十七种 Long 自有 AES 密码数字分卷、ZPAQ 原生密码、数字分卷及原生多部分整组另存增量更新、WIM 存储/XPRESS/LZX/LZMS 与固实 ESD、数字分卷、SWM 原生资源分包、LZ4/Brotli/LZIP 与 TAR 组合、智能体积分析、配置模板和归档转换 |
| 特殊压缩 | JPEG/PNG/WebP 图片编码与缩放、视频压缩预设、PDF 分析与结构优化 |
| LZOP 创建 | 单个普通文件生成 `.lzo`，原展开/批量配置、分析/进度/取消、校验、结果/历史与单顶层文件归档转换；固定模式，不提供密码或创建分卷 |
| 文件浏览器 | 双栏浏览本地文件，查看归档目录、搜索筛选、文本/图片预览与选择性解压（7Z Bzip2 普通/密码、非固实/固实包支持完整文本 ≤1 MiB、图片 ≤8 MiB，含内容/文件名加密和完整数字卷，共享压缩/解码各 ≤64 MiB；Bzip2 非固实/固实大文本可先看前 1 MiB，前置与开头合计 ≤64 MiB，明确未校验整文件并可提取全文；7Z Deflate64 普通/密码非固实及固实包支持完整文本 ≤1 MiB、图片 ≤8 MiB，含内容/文件名加密和完整数字分卷；固实共享压缩/解码分别 ≤64 MiB；非固实及固实大文本可先看前 1 MiB，固实前置内容与开头合计 ≤64 MiB，明确未校验整文件并可提取全文；7Z Deflate 普通/密码包支持完整图文（文本 ≤1 MiB、图片 ≤8 MiB，含固实、内容/文件名加密与数字分卷；共享压缩/解码各 ≤64 MiB）；大文本可先看前 1 MiB，含固实（前置内容与开头合计 ≤64 MiB），标明未校验整文件并可提取全文；7Z Copy 支持普通/密码、非固实/固实完整图文及大文本开头，范围沿原预览预算；7Z PPMd 支持普通/密码、非固实/小型固实的完整小文本与图片，非固实及固实大文本可读前 1 MiB（明确未校验整文件），LZMA/LZMA2 支持普通/密码、非固实/固实完整图文和大文本前 1 MiB；ALZ 支持完整小文本、大文本前缀和图片预览，EGG 支持非固实小文本/大文本前缀、图片和小型未加密固实图文预览、原连续阅读）；按目录/整包连续看图、缩略导航/缩放、异常跳过、阅读记忆、找图分页、跨页挑选与原队列取回；Store/Deflate/Bzip2/LZMA/Zstandard/PPMd ZIP、ZIPX（XZ） 普通/密码内容编辑、添加/替换/删除/改名/批量移动、复核另存与编辑草稿恢复；支持 LZOP 单文件/多成员与路径目录、旧名称代码页及完整数字卷读取，TAR.LZO 目录只读流程，以及普通压缩流、自有 AES 密码卷组、原生 SWM 多镜像及 ZPAQ 原生多部分确认读取、历史快照对比与变化提取、中断尾部诊断与完整旧快照取回、受损块和文件影响定位 |
| 密码保险箱 | 本地加密保存常用解压密码，分类、搜索、导入导出与使用统计 |
| 工具中心 · 归档资源库 | 现有可浏览归档本地建库，密码/分卷归组、跨包文件名/路径搜索、来源与类型筛选，来源收藏/标签、保存搜索、跨页多选弹窗配置与按来源批量取回，命中定位/预览；离线索引、定位和增量更新、暂停/继续及重启保留，移出保留原包 |
| 工具中心 · 工作流中心 | 保存手动顺序方案，按模板筛选、解压、压缩或转换；运行前集中选择来源与密码，实际产物接续、逐步进度/日志/结果、失败纠正、指定步骤重跑与跨次接续，核对实际内容后复用成功产物；明确启用目录规则，等待稳定/分卷到齐、按快照排队、暂停取消和跨次记录；单次/每天/每周定时、下一次时间、错过补跑/跳过与最近结果 |
| 工具中心 · 资料对比 | 文件夹与 Store/Deflate/Bzip2/LZMA/Zstandard/PPMd ZIP、ZIPX（XZ） 任意组合，按路径查看差异、实际读取内容核对，进度/停止与剩余接续；AES/ZipCrypto 密码、数字/标准卷，搜索筛选/跨页选择、弹窗取回与原草稿历史贯通；选择基础与补充，复核冲突/最终结构并另存新 ZIP，引用草稿恢复与失败继续 |
| 工具中心 · 文件完整性 | 从完成结果生成整组相对路径清单、CRC32/MD5/SHA256 校验、清单逐文件原因与待补列表/按需复查、真实进度/停止与继续、归档诊断、RAR4/RAR5 副本修复与恢复卷（需本机 Rar.exe），以及密码 ZIP 与目录可读的标准卷完整文件救援 |
| 外观与设置 | 六类设置、动态格式统计、四种新增特色主题、强调色与缩放、辅助显示、关闭到托盘与窗口记忆 |
| 历史与系统集成 | 搜索和再次执行、统一结果操作、托盘、资源管理器经典菜单（压缩/解压/校验清单）与带下载进度的应用内更新 |

详细格式能力见 [格式支持说明](docs/FORMAT_SUPPORT_LEVELS.md)；RAR 创建依赖用户安装的 WinRAR，视频暂不保证支持 Windows N。功能介绍和快速上手见 [项目首页](../README.md)。

当前开发分支已加入 Windows 默认打开与四项动作、包内外共用的双栏布局、右键操作分组及历史操作菜单，尚未正式发布。功能与界面记录见 [本轮说明](docs/WINDOWS_OPEN_AND_BROWSER_UI.md)，原 TAR.LZO 目录创建保留在 [后续格式主线](docs/ARCHIVE_TAR_LZO_NEXT.md)。

## 本地开发

Windows 构建需要受支持的 Node.js（20.19+、22.12+ 或 24）、npm、稳定版 Rust、MSVC C++ 构建工具及 WebView2。

```powershell
git clone https://github.com/Longyuyeee/long_Decompress.git
cd long_Decompress\long-compress-assistant
npm ci
npm run tauri dev
```

只调整前端界面时，可运行 `npm run dev`；需要调用文件处理和系统集成时，使用 Tauri 开发模式。

| 命令 | 用途 |
| --- | --- |
| `npm run type-check` | TypeScript 与 Vue 类型检查 |
| `npm run test:unit` | 前端单元测试 |
| `npm run build` | 前端生产构建 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 后端测试 |
| `npm run tauri build -- --bundles nsis` | 构建 Windows 安装器 |

按改动范围选择检查，复用已有结果。文档或展示资源更新不需要重新执行安装和格式全矩阵。

## 项目结构

```text
src/
  components/          页面组件、任务与结果操作
  views/               各工作区页面
  stores/              配置、任务、历史与媒体状态
  composables/         可复用交互与 Tauri 接口
  utils/               格式、路径、结果与任务辅助逻辑
src-tauri/
  src/                 Rust 引擎、数据库与系统集成
  resources/           随包引擎、原生菜单和第三方许可
  shell-extension/     Windows 原生右键扩展
  tests/               后端集成测试
scripts/               构建与功能检查脚本
e2e/                   浏览器与桌面交互场景
docs/                  开发路线、格式说明与历史记录
```

## 开发文档

- [产品路线](docs/PRODUCT_ENHANCEMENT_ROADMAP.md)与 [开发交接](docs/DEVELOPMENT_HANDOFF.md)：接续时先查看当前范围。
- [批量配置与结果操作](docs/BATCH_CONFIG_AND_RESULTS_PLAN.md)：用户交互约定与实现记录。
- [格式支持说明](docs/FORMAT_SUPPORT_LEVELS.md)：创建、读取和导入范围。
- [桌面交互说明](docs/DESKTOP_E2E.md)与 [测试指南](tests/README.md)：按需运行相关场景。
- [性能基线](docs/PERFORMANCE_BASELINE.md)与 [AES 流式容器](docs/AES_STREAM_V2.md)：对应实现与数据来源。

## 打包与发布

构建会同步准备前端、原生菜单扩展和可用的菜单身份包。正式 `v*` 标签由 [Release 工作流](../.github/workflows/release.yml)生成 NSIS 安装器、更新包、签名文件和 `latest.json`。

版本说明放在 `docs/RELEASE_NOTES_<版本>.md`，仓库首页只维护软件特点与末尾版本摘要。签名更新所需的 `TAURI_PRIVATE_KEY` 与 `TAURI_KEY_PASSWORD` 由发布环境提供。

<a id="third-party"></a>

## 许可证与第三方组件

项目代码使用 [MIT License](../LICENSE)。随包 7-Zip、ZPAQ、WIM 编码器、ALZ、EGG、视频与 PDF 引擎分别遵循各自许可；ALZ/EGG 的源码、构建与完整依赖许可位于 `vendor/unalz/`、`vendor/unegg/`、`vendor/libazo/`、`tools/alz-engine/`、`tools/egg-engine/` 和对应 resources 目录。WIM 编码器的原始源码压缩包、许可和构建说明位于 `src-tauri/resources/wim-engine/`，随安装包分发。WinRAR 编码器不随项目分发。

## 更新记录

**v1.6.62 · LZOP 单文件创建**。原压缩中心可直接生成标准 `.lzo`，多选文件分别创建；分析、进度/取消、校验、统一结果及历史接通。仅含一个顶层文件的归档可转成 LZOP，目录/多文件明确改选格式，保留原包。固定模式收起无效的强度与密码选项，保持原交互。[中文说明](docs/RELEASE_NOTES_1.6.62.md) · [记录](docs/LZOP_CREATE_FREEZE_1.6.62.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.62) · [下一版 TAR.LZO 创建](docs/ARCHIVE_ROADMAP_1.6.63.md)。

**v1.6.61 · LZOP 目录还原与旧名称编码**。带路径成员可在原双栏进入目录、搜索、预览、按目录取回并转成 ZIP/7Z；名称代码页收在原选项区，所有流程沿用同一选择。重名/路径相交与扁平取回保留独立结果，合法长名称不再任意截短。[中文说明](docs/RELEASE_NOTES_1.6.61.md) · [记录](docs/LZOP_PATHS_NAMES_FREEZE_1.6.61.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.61) · [下一版单文件创建](docs/ARCHIVE_ROADMAP_1.6.62.md)。

**v1.6.60 · LZOP 多成员资料读取**。同一个 `.lzo` 中的文件可在原双栏中分别浏览、图文预览、按需取回、完整检查并转成 ZIP/7Z，完整数字卷保留非首卷入口。重名/缺名称提供稳定成员名；整包任务遇到坏成员不发布半成品。[中文说明](docs/RELEASE_NOTES_1.6.60.md) · [记录](docs/LZOP_MEMBERS_READ_FREEZE_1.6.60.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.60) · [下一版路径与名称兼容](docs/ARCHIVE_ROADMAP_1.6.61.md)。

**v1.6.59 · LZOP / TAR.LZO 数字分卷读取**。集齐数字分卷后可从任意一卷沿原双栏打开、预览、按需取回、完整检查并转成 ZIP/7Z，保留来源与结果/历史。缺卷给出具体原因，补齐后重试；数字切片不等于原生多部分或多成员。[中文说明](docs/RELEASE_NOTES_1.6.59.md) · [记录](docs/LZOP_NUMERIC_READ_FREEZE_1.6.59.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.59) · [下一版多成员资料读取](docs/ARCHIVE_ROADMAP_1.6.60.md)。

**v1.6.58 · TAR.LZO 目录读取与转换**。`.tar.lzo` 直接在原双栏中浏览目录、搜索、图文预览和选择性取回，接通完整检查、整包提取与 ZIP/7Z 转换，保留中文长路径和已验证 GNU/PAX 内容。普通 `.lzo` 仍按单文件读取，本版只读、不新增原生密码或创建。[中文说明](docs/RELEASE_NOTES_1.6.58.md) · [记录](docs/TAR_LZO_READ_CONVERT_FREEZE_1.6.58.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.58) · [下一版完整数字切卷](docs/ARCHIVE_ROADMAP_1.6.59.md)。

**v1.6.57 · LZOP 单文件读取与转换**。普通 `.lzo` 可在原双栏中浏览、预览、检查、提取并转成 ZIP/7Z，支持三种常见方法与头/块校验，沿用原交互与结果入口。仅支持单成员只读；TAR.LZO 目录和数字卷分期推进。[中文说明](docs/RELEASE_NOTES_1.6.57.md) · [记录](docs/LZOP_READ_CONVERT_FREEZE_1.6.57.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.57) · [下一版 TAR.LZO](docs/ARCHIVE_ROADMAP_1.6.58.md)。

**v1.6.56 · 可选 7Z RISC-V BCJ 预处理**。原折叠高级设置增加 RISC-V BCJ，默认关闭；密码、固实、数字卷、图文/开头/提取、配置保存与实际分析/转换接通，保持原交互。[中文说明](docs/RELEASE_NOTES_1.6.56.md) · [记录](docs/SEVENZ_RISCV_BCJ_CREATE_FREEZE_1.6.56.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.56)。下一阶段 [LZOP 读取与转换](docs/ARCHIVE_ROADMAP_1.6.57.md)仅规划。

**v1.6.55 · 可选 7Z ARM64 BCJ 预处理**。原折叠高级设置增加 ARM64 BCJ，默认关闭；密码、固实、数字卷、图文/开头/提取、配置保存与实际分析/转换接通。[中文说明](docs/RELEASE_NOTES_1.6.55.md) · [记录](docs/SEVENZ_ARM64_BCJ_CREATE_FREEZE_1.6.55.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.55)。RISC-V BCJ 由上方 v1.6.56 接续。

**v1.6.54 · 可选 7Z ARM Thumb BCJ 预处理**。原折叠高级设置增加 Thumb BCJ，默认关闭；密码、固实、数字卷、图文/开头/提取、配置保存与实际分析/转换接通。[中文说明](docs/RELEASE_NOTES_1.6.54.md) · [记录](docs/SEVENZ_ARM_THUMB_BCJ_CREATE_FREEZE_1.6.54.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.54)。ARM64 BCJ 由上方 v1.6.55 接续。

**v1.6.53 · 可选 7Z ARM32 BCJ 预处理**。原折叠高级设置增加 ARM32 小端 BCJ，默认关闭；密码、固实、数字卷、预览/提取、配置保存和真实分析/转换接通，修复历史选项恢复。[中文说明](docs/RELEASE_NOTES_1.6.53.md) · [记录](docs/SEVENZ_ARM32_BCJ_CREATE_FREEZE_1.6.53.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.53)。ARM Thumb BCJ 由上方 v1.6.54 接续。

**v1.6.52 · 可选 7Z x86 BCJ 预处理**。原折叠高级设置增加面向可执行代码的 BCJ，默认关闭，切回 Delta 保留距离；密码、固实、分卷、预览/提取、配置保存、真实分析和转换贯通。[中文说明](docs/RELEASE_NOTES_1.6.52.md) · [记录](docs/SEVENZ_X86_BCJ_CREATE_FREEZE_1.6.52.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.52)。ARM32 BCJ 由上方 v1.6.53 接续。

**v1.6.51 · 可选 7Z Delta 预处理**。LZMA2/LZMA 可选距离 1～256 的 Delta，默认折叠并关闭；密码、固实、分卷、预览和转换贯通，配置保存与真实分析接通，修复距离 256 读取。[中文说明](docs/RELEASE_NOTES_1.6.51.md) · [记录](docs/SEVENZ_DELTA_CREATE_FREEZE_1.6.51.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.51)。7Z x86 BCJ 预处理由上方 v1.6.52 接续。

**v1.6.50 · ZIP 快速仅存储**。原方法新增 Store，密码、数字/标准分卷和转换接通；实际抽样给出可选建议，采用只改当前方法。修复转换切换方法被旧值覆盖，保留原交互和历史设置。[中文说明](docs/RELEASE_NOTES_1.6.50.md) · [记录](docs/ZIP_STORE_CREATE_FREEZE_1.6.50.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.50)。7Z Delta 预处理由上方 v1.6.51 接续。

**v1.6.49 · 7Z 仅归档与低收益建议**。原方法新增 Copy 快速打包，可加密、固实、分卷和转换；按实际样本给出可选建议，采用时只改当前方法，并修复空流/CRC 0 元数据。[中文说明](docs/RELEASE_NOTES_1.6.49.md) · [记录](docs/SEVENZ_COPY_CREATE_FREEZE_1.6.49.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.49)。ZIP 明确仅存储与低收益建议由上方 v1.6.50 接续。

**v1.6.48 · 7Z PPMd 创建**。原方法选择加入 PPMdH，接通密码、隐藏文件名、固实、数字分卷及 ZIP PPMd8 转换；草稿/模板/历史/工作流保留方法，体积分析使用实际编码。[中文说明](docs/RELEASE_NOTES_1.6.48.md) · [记录](docs/SEVENZ_PPMD_CREATE_FREEZE_1.6.48.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.48)。7Z 仅归档与低收益建议由上方 v1.6.49 接续。

**v1.6.47 · 7Z Deflate 创建**。原配置选择 Deflate，接通密码、隐藏文件名、固实、数字分卷和转换，草稿/模板/历史/工作流保留方法，体积分析使用实际编码。[中文说明](docs/RELEASE_NOTES_1.6.47.md) · [记录](docs/SEVENZ_DEFLATE_CREATE_FREEZE_1.6.47.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.47)。7Z PPMd 创建由上方 v1.6.48 接续。

**v1.6.46 · 7Z 压缩方法可选**。原配置中选择 LZMA2（默认）/LZMA/Bzip2，接通密码、隐藏文件名、固实、数字分卷和转换，配置与草稿保留选择，体积分析按实际方法抽样。[中文说明](docs/RELEASE_NOTES_1.6.46.md) · [记录](docs/SEVENZ_COMPRESSION_METHODS_FREEZE_1.6.46.md) · [发布下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.46)。7Z Deflate 创建由上方 v1.6.47 接续。

**v1.6.45 · Bzip2 大日志先看开头**。普通/密码、非固实/固实同版先看所选前 1 MiB，接通内容/文件名加密及完整数字卷/非首卷，保留编码/滚动、部分范围和提取全文，达到目标后停止推进。[中文说明](docs/RELEASE_NOTES_1.6.45.md) · [范围与记录](docs/SEVENZ_BZIP2_TEXT_PREFIX_FREEZE_1.6.45.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.45)。7Z 创建方法由上方 v1.6.46 接续。

**v1.6.44 · 7Z Bzip2 图文直接看**。普通/密码、非固实/固实包同版接回原双栏完整文本与图片，含内容/文件名加密、完整数字卷/非首卷、连续阅读及仅提取当前页。[中文说明](docs/RELEASE_NOTES_1.6.44.md) · [范围与记录](docs/SEVENZ_BZIP2_PREVIEW_FREEZE_1.6.44.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.44)。Bzip2 大文本开头由上方 v1.6.45 接续。

**v1.6.43 · 固实 Deflate64 大日志先看开头**。普通/密码固实包的后续大文本也能在原双栏先看前 1 MiB，接通内容/文件名加密与完整数字卷/非首卷；流式跳过前置内容，仅保留所选开头，明确部分范围并可提取全文。[中文说明](docs/RELEASE_NOTES_1.6.43.md) · [范围与记录](docs/SEVENZ_SOLID_DEFLATE64_PREFIX_FREEZE_1.6.43.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.43)。Bzip2 完整图文由上方 v1.6.44 接续。

**v1.6.42 · Deflate64 大日志先看开头**。普通/密码非固实包的大文本可在原双栏窗口先看前 1 MiB，接通内容/文件名加密和完整数字卷/非首卷；保留常见编码、字符边界、滚动和提取全文，明确未校验整文件。[中文说明](docs/RELEASE_NOTES_1.6.42.md) · [范围与记录](docs/SEVENZ_DEFLATE64_TEXT_PREFIX_FREEZE_1.6.42.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.42)。固实 Deflate64 大文本开头由上方 v1.6.43 接续。

**v1.6.41 · 密码固实 Deflate64 图文直接看**。AES 共享块里的完整小文本和图片接回原双栏与密码入口，含内容/文件名加密、完整数字卷/非首卷、连续阅读和仅提取当前页；保持共享完整校验及完整加密消费。[中文说明](docs/RELEASE_NOTES_1.6.41.md) · [范围与记录](docs/SEVENZ_PASSWORD_SOLID_DEFLATE64_PREVIEW_FREEZE_1.6.41.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.41)。非固实 Deflate64 大文本开头由上方 v1.6.42 接续。

**v1.6.40 · 固实 Deflate64 图文直接看**。普通共享块里的完整小文本和图片接回原双栏预览、连续阅读与仅提取当前页，支持完整数字卷/非首卷；流式处理前后内容，仅保留所选文件，检查整个共享块结束与全部可用 CRC。[中文说明](docs/RELEASE_NOTES_1.6.40.md) · [范围与记录](docs/SEVENZ_SOLID_DEFLATE64_PREVIEW_FREEZE_1.6.40.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.40)。密码固实 Deflate64 图文由上方 v1.6.41 接续。

**v1.6.39 · 密码 Deflate64 图文直接看**。非固实 AES 包也能在原双栏直接预览小文本和图片，含内容/文件名加密、完整数字卷/非首卷，沿原手动密码/保险箱、连续阅读与仅提取当前页入口。[中文说明](docs/RELEASE_NOTES_1.6.39.md) · [范围与记录](docs/SEVENZ_PASSWORD_DEFLATE64_PREVIEW_FREEZE_1.6.39.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.39)。普通固实 Deflate64 图文由上方 v1.6.40 接续。

**v1.6.38 · Deflate64 图文直接看**。普通非固实 7Z 的完整小文本和图片直接接回原双栏预览、连续阅读和仅提取当前页，支持完整数字分卷/非首卷，明确核对真实结束、压缩覆盖与 CRC。[中文说明](docs/RELEASE_NOTES_1.6.38.md) · [范围与记录](docs/SEVENZ_DEFLATE64_PREVIEW_FREEZE_1.6.38.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.38)。密码 Deflate64 图文由上方 v1.6.39 接续。

**v1.6.37 · 固实 Deflate 大日志先看开头**。普通/密码固实包的后续大文本也能在原双栏窗口先看前 1 MiB，完整数字卷/非首卷、密码入口、编码/滚动和提取全文贯通；流式跳过前置内容，仅保留所选开头，明确未校验整文件。[中文说明](docs/RELEASE_NOTES_1.6.37.md) · [范围与记录](docs/SEVENZ_SOLID_DEFLATE_PREFIX_FREEZE_1.6.37.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.37)。普通 Deflate64 完整图文由上方 v1.6.38 接续。

**v1.6.36 · Deflate 大日志先看开头**。普通/密码非固实包的大文本可在原双栏窗口先看前 1 MiB，内容/文件名加密、完整数字卷/非首卷和原密码入口贯通；保留编码/滚动、明确部分范围与提取全文。[中文说明](docs/RELEASE_NOTES_1.6.36.md) · [范围与记录](docs/SEVENZ_DEFLATE_TEXT_PREFIX_FREEZE_1.6.36.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.36)。固实大文本开头预览由上方 v1.6.37 接续。

**v1.6.35 · Deflate 固实图文直接预览**。普通/密码固实包接回原双栏文本、图片与整包阅读，内容/文件名加密、完整数字卷/非首卷、原密码入口与当前页提取贯通；共享块完整读取后只保留所选内容。[中文说明](docs/RELEASE_NOTES_1.6.35.md) · [范围与记录](docs/SEVENZ_SOLID_DEFLATE_PREVIEW_FREEZE_1.6.35.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.35)。非固实大文本开头预览由上方 v1.6.36 接续。

**v1.6.34 · 密码 Deflate 图文直接预览**。AES 内容/文件名加密与完整数字卷接回原双栏文本、图片和整包阅读，沿原手动密码/保险箱与当前页提取；完整 AES 消费和 Deflate 结束同时确认，保留编码/滚动、取消/来源与下一步入口。[中文说明](docs/RELEASE_NOTES_1.6.34.md) · [范围与记录](docs/SEVENZ_PASSWORD_DEFLATE_PREVIEW_FREEZE_1.6.34.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.34)。Deflate 固实图文由上方 v1.6.35 接续。

**v1.6.33 · 7Z Deflate 图文原位预览**。普通非固实包沿原双栏窗口完整查看小文本和图片，接通数字分卷、原整包阅读/当前页提取；确认完整流结束、大小、压缩消费和 CRC，保留编码、取消/来源与下一步入口。[中文说明](docs/RELEASE_NOTES_1.6.33.md) · [范围与记录](docs/SEVENZ_DEFLATE_PREVIEW_FREEZE_1.6.33.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.33)。密码 Deflate 图文由上方 v1.6.34 接续。

**v1.6.32 · 固实 PPMd 大日志直接读开头**。普通/密码固实包沿原窗口显示所选前 1 MiB，接通内容/文件名加密、手动密码/保险箱和完整数字卷；保留编码、滚动及提取入口。只跳过必要前置内容，明确“部分内容 / 未校验整文件”，超出范围给出下一步。[中文说明](docs/RELEASE_NOTES_1.6.32.md) · [范围与记录](docs/SEVENZ_SOLID_PPMD_TEXT_PREFIX_FREEZE_1.6.32.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.32)。普通非固实 Deflate 图文由上方 v1.6.33 接续。

**v1.6.31 · PPMd 非固实大日志直接读开头**。普通/密码包接入原文本窗口，显示前 1 MiB 并明确“部分内容 / 未校验整文件”；手动密码/保险箱、内容/文件名加密和完整数字卷贯通，保留编码、滚动、取消/来源与提取入口。[中文说明](docs/RELEASE_NOTES_1.6.31.md) · [范围与记录](docs/SEVENZ_PPMD_TEXT_PREFIX_FREEZE_1.6.31.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.31)。固实 PPMd 大日志由上方 v1.6.32 接续。

**v1.6.30 · 密码固实 PPMd 7Z 直接查看图文**。AES 内容/文件名加密的小型固实包接入原双栏完整小文本、图片与连续阅读；手动密码/保险箱、完整数字卷、取消/来源和准确当前页提取贯通，完整检查 AES 与共享 PPMd 结束。[中文说明](docs/RELEASE_NOTES_1.6.30.md) · [范围与记录](docs/SEVENZ_PASSWORD_SOLID_PPMD_PREVIEW_FREEZE_1.6.30.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.30)。非固实 PPMd 大日志由上方 v1.6.31 接续。

**v1.6.29 · 固实 PPMd 7Z 直接查看图文**。未加密小型固实包接入原双栏完整小文本、图片与连续阅读；完整检查所选共享块的前后内容与结束，只保留所选文件，沿原数字卷、取消/来源和准确当前页提取。[中文说明](docs/RELEASE_NOTES_1.6.29.md) · [范围与记录](docs/SEVENZ_SOLID_PPMD_PREVIEW_FREEZE_1.6.29.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.29)。密码固实 PPMd 图文由上方 v1.6.30 接续。

**v1.6.28 · 密码 PPMd 7Z 直接查看图文**。非固实包的 AES 内容/文件名加密接入原双栏完整小文本、图片与连续阅读；沿用手动密码/保险箱、完整数字卷与当前页提取。确认完整 AES 和 PPMd 结束，保留来源/取消及损坏下一步。[中文说明](docs/RELEASE_NOTES_1.6.28.md) · [范围与记录](docs/SEVENZ_PASSWORD_PPMD_PREVIEW_FREEZE_1.6.28.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.28)。普通固实 PPMd 图文由上方 v1.6.29 接续。

**v1.6.27 · PPMd 7Z 直接查看图文**。未加密非固实包接入原双栏完整小文本、五种图片与原连续阅读；完整数字卷、准确当前页提取、取消与来源贯通。核对 PPMd 实际结束状态，正文可读而尾部损坏仍明确报错。[中文说明](docs/RELEASE_NOTES_1.6.27.md) · [范围与记录](docs/SEVENZ_PPMD_PREVIEW_FREEZE_1.6.27.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.27)。密码 PPMd 图文由上方 v1.6.28 接续。

**v1.6.26 · 密码固实 7Z 大日志直接读开头**。AES + LZMA/LZMA2 固实大文本接入原双栏，内容/文件名加密、原密码/保险箱和完整数字卷贯通；有界跳过前置内容，准确显示所选前 1 MiB，明确部分内容/未校验整文件，保留取消与“提取后打开”。[中文说明](docs/RELEASE_NOTES_1.6.26.md) · [范围与记录](docs/SEVENZ_ENCRYPTED_SOLID_TEXT_PREFIX_FREEZE_1.6.26.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.26)。PPMd 非固实图文由上方 v1.6.27 接续。

**v1.6.25 · 固实 7Z 大日志直接读开头**。未加密 LZMA/LZMA2 固实大文本接入原双栏，准确跳过有界前置内容，只保留所选前 1 MiB；明确部分内容/未校验整文件，保留数字卷、编码边界、取消与“提取后打开”。[中文说明](docs/RELEASE_NOTES_1.6.25.md) · [范围与记录](docs/SEVENZ_SOLID_TEXT_PREFIX_FREEZE_1.6.25.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.25)。密码固实大文本由上方 v1.6.26 接续。

**v1.6.24 · 密码 7Z 大日志直接读开头**。AES + LZMA/LZMA2 非固实大文本接入原双栏，内容/文件名加密、原密码/保险箱和完整数字卷贯通；明确部分内容/未校验整文件，保留编码边界、取消与“提取后打开”。[中文说明](docs/RELEASE_NOTES_1.6.24.md) · [范围与记录](docs/SEVENZ_PASSWORD_TEXT_PREFIX_FREEZE_1.6.24.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.24)。未加密固实大文本由上方 v1.6.25 接续。

**v1.6.23 · 7Z 大日志直接读开头**。未加密 LZMA/LZMA2 非固实大文本在原双栏显示前 1 MiB，明确部分内容/未校验整文件，保留编码边界、完整数字卷与关闭取消；可沿“提取后打开”查看全文；修复 LZMA2 结束标记损坏时完整检查误报成功。[中文说明](docs/RELEASE_NOTES_1.6.23.md) · [范围与记录](docs/SEVENZ_TEXT_PREFIX_FREEZE_1.6.23.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.23)。密码大文本由上方 v1.6.24 接续。

**v1.6.22 · 带密码的 7Z 固实相册直接看图**。AES + LZMA/LZMA2 固实图片接入原双栏和阅读器，内容/文件名加密、原密码/保险箱与完整数字分卷贯通。按需翻页、缩放、跳过坏图、定位与准确提取当前页，另一栏位置保持。[中文说明](docs/RELEASE_NOTES_1.6.22.md) · [范围与记录](docs/SEVENZ_ENCRYPTED_SOLID_IMAGE_FREEZE_1.6.22.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.22)。7Z 非固实大文本开头已由上方 v1.6.23 接续。

**v1.6.21 · 带密码的 7Z 固实说明直接阅读**。在原双栏阅读 AES + LZMA/LZMA2 固实说明与日志，接通原密码/保险箱、文件名加密和完整数字卷。目录只读取文件名信息，选中内容完整检查后显示，保留取消和来源核对。[中文说明](docs/RELEASE_NOTES_1.6.21.md) · [范围与记录](docs/SEVENZ_ENCRYPTED_SOLID_TEXT_FREEZE_1.6.21.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.21)。7Z 加密固实图片已由上方 v1.6.22 接续。

**v1.6.20 · 7Z 固实相册直接看图**。未加密 LZMA/LZMA2 固实图片接入原双栏弹层和阅读器，完整数字分卷沿原入口；按需翻页、缩放、跳过坏图、定位和准确提取当前页，检查整个共享块后显示，保留取消与来源核对。[中文说明](docs/RELEASE_NOTES_1.6.20.md) · [范围与记录](docs/SEVENZ_SOLID_IMAGE_PREVIEW_FREEZE_1.6.20.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.20)。7Z 加密固实小文本已由上方 v1.6.21 接续。

**v1.6.19 · 7Z 固实小文本直接阅读**。在原双栏直接打开未加密 LZMA/LZMA2 固实包内的说明、日志与配置，完整数字分卷沿原入口；检查整个共享块后显示，保留滚动、关闭取消和来源核对。只保留选中文本，超限或加密固实仍可先提取。[中文说明](docs/RELEASE_NOTES_1.6.19.md) · [范围与记录](docs/SEVENZ_SOLID_TEXT_PREVIEW_FREEZE_1.6.19.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.19)。7Z 小型固实图片已由上方 v1.6.20 接续。

**v1.6.18 · 7Z 图片预览与连续阅读**。原双栏直接查看非固实 LZMA/LZMA2 图片，接通密码、文件名加密数字卷与原阅读器；按需翻页、缩放、跳过、定位和准确提取当前页，保持取消/来源核对与左右等宽。[中文说明](docs/RELEASE_NOTES_1.6.18.md) · [范围与记录](docs/SEVENZ_IMAGE_PREVIEW_FREEZE_1.6.18.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.18)。7Z 小型固实文本已由上方 v1.6.19 接续。

**v1.6.17 · 7Z 小文本原位预览**。在原双栏直接阅读 LZMA/LZMA2 非固实完整小文本，接通密码、保险箱与文件名加密数字卷，完整大小/CRC/解码结束后显示；保留取消、来源刷新与明确提取指引。[中文说明](docs/RELEASE_NOTES_1.6.17.md) · [范围与记录](docs/SEVENZ_TEXT_PREVIEW_FREEZE_1.6.17.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.17)。7Z 非固实图片已由上方 v1.6.18 接续。

**v1.6.16 · EGG 大文本前缀预览**。原双栏可直接阅读非固实大日志开头，最多前 1 MiB，明确“部分内容 · 未校验整文件”，保留字符边界；密码/原生卷、取消和来源刷新沿旧入口使用，完整操作保持原校验条件。[中文说明](docs/RELEASE_NOTES_1.6.16.md) · [范围与记录](docs/EGG_TEXT_PREFIX_FREEZE_1.6.16.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.16)。7Z 非固实小文本已由上方 v1.6.17 接续。

**v1.6.15 · EGG 固实图片与连续阅读**。在原双栏查看小型未加密 Deflate / LZMA 固实图片，沿原阅读器翻页、缩放、跳过和准确提取当前页；完整共享依赖检查后显示，按需读取、明确等待，保留有限缓存与取消/来源刷新。[中文说明](docs/RELEASE_NOTES_1.6.15.md) · [范围与记录](docs/EGG_SOLID_IMAGE_PREVIEW_FREEZE_1.6.15.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.15)。EGG 非固实大文本前缀已由上方 v1.6.16 接续。

**v1.6.14 · EGG 固实包内文本预览**。在原双栏直接查看未加密 Deflate / LZMA 固实包内的小文本；约束完整共享流读取，全部依赖检查通过后显示，损坏/超限/加密固实给下一步。[中文说明](docs/RELEASE_NOTES_1.6.14.md) · [范围与记录](docs/EGG_SOLID_TEXT_PREVIEW_FREEZE_1.6.14.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.14)。EGG 小型固实图片已由上方 v1.6.15 接续。

**v1.6.13 · EGG 图片预览与连续阅读**。在原双栏直接查看受支持的非固实图片，沿原阅读器翻页、缩放、跳过坏图、定位和提取当前页；接通密码保险箱、显式代码页、完整原生卷、取消与来源刷新。[中文说明](docs/RELEASE_NOTES_1.6.13.md) · [范围与记录](docs/EGG_IMAGE_PREVIEW_FREEZE_1.6.13.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.13)。EGG 小型固实文本已由上方 v1.6.14 接续。

**v1.6.12 · EGG 包内文本预览**。在原双栏直接查看非固实小文本，接通手动密码、保险箱、显式编码、完整原生卷、取消与来源刷新；完整校验所选内容，固实及超限给提取指引。[中文说明](docs/RELEASE_NOTES_1.6.12.md) · [范围与记录](docs/EGG_TEXT_PREVIEW_FREEZE_1.6.12.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.12)。EGG 图片已由上方 v1.6.13 接续。

**v1.6.11 · ALZ 大文本前缀预览**。直接在原双栏查看较大日志、说明和配置的开头，最多显示前 1 MiB 并标明未校验整文件；接通密码保险箱、显式编码与完整原生卷，关闭取消、来源变化刷新，完整提取/检查/转换保持原校验条件。[中文说明](docs/RELEASE_NOTES_1.6.11.md) · [范围与记录](docs/ALZ_TEXT_PREFIX_FREEZE_1.6.11.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.11)。EGG 小文本已由上方 v1.6.12 接续。

**v1.6.10 · ALZ 图片预览与连续阅读**。在原双栏直接看图，接通现有翻页、缩放、坏图跳过、定位和当前页提取；沿用密码保险箱、编码与完整原生卷组，关闭取消、来源变化提示刷新。[中文说明](docs/RELEASE_NOTES_1.6.10.md) · [范围与记录](docs/ALZ_IMAGE_PREVIEW_FREEZE_1.6.10.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.10)。大文本前缀已由上方 v1.6.11 接续。

**v1.6.9 · ALZ 包内文本预览**。在原双栏直接查看说明、日志与配置，接通现有密码保险箱、显式编码和完整原生卷组；只读所选完整小文件，关闭取消，来源变化提示刷新，大文件给提取指引。保留原弹层和左右等宽布局。[中文说明](docs/RELEASE_NOTES_1.6.9.md) · [范围与记录](docs/ALZ_TEXT_PREVIEW_FREEZE_1.6.9.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.9)。图片预览已由上方 v1.6.10 接续，大文本前缀另列后续。

**v1.6.8 · ALZ Bzip2 按块读取**。不再先收集整条压缩数据，移除旧 256 MiB 单条限制；保留密码状态、跨块位对齐、完整性和原任务操作。大包边界经明确标注的受控归档与独立回解核对，历史原工具大包仍待取得。[中文说明](docs/RELEASE_NOTES_1.6.8.md) · [范围与记录](docs/ALZ_BOUNDED_FREEZE_1.6.8.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.8)。小文本预览已由上方 v1.6.9 接续，图片与大文本前缀继续分阶段推进。

**v1.6.7 · 工具中心**。资料对比、工作流中心、归档资源库和文件完整性集中到四个分类页签，侧边栏统一为八个入口；`Ctrl+Shift+T` 打开工具中心，原四种快捷键继续直达。切换分类保留输入、选择、展开和结果，界面沿用原有样式。[中文说明](docs/RELEASE_NOTES_1.6.7.md) · [范围与记录](docs/TOOLS_CENTER_FREEZE_1.6.7.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.7)。

**v1.6.6 · 窗口位置与大小记忆**。调整好的窗口在重启后自动恢复，最大化保留还原尺寸，最小化/托盘不覆盖正常位置；换屏或降低分辨率时调整到可见区域，无需额外设置。回应 [#248](https://github.com/Longyuyeee/long_Decompress/issues/248)。[中文说明](docs/RELEASE_NOTES_1.6.6.md) · [范围与记录](docs/WINDOW_MEMORY_FREEZE_1.6.6.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.6)。

**v1.6.5 · ALZ Bzip2 完整性修复**。缺少结束标记或声明压缩范围内夹入尾随数据时，不再把完整解出内容当作检查、解压或转换成功；大小和 CRC 均匹配后给出明确来源损坏原因，沿原失败新任务和历史继续处理。[中文说明](docs/RELEASE_NOTES_1.6.5.md) · [范围与记录](docs/ALZ_BZIP2_INTEGRITY_FREEZE_1.6.5.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.5)。方法 3 仍待真实原包。

**v1.6.4 · 从受损 ZPAQ 取回已校验文件**。在诊断详情勾选文件，二级弹窗确认后加入原队列；按所选范围验证密码，独立目录保留原路径，结果区分已取回与未取回原因。草稿、补密码、失败新任务及历史保留范围。[中文说明](docs/RELEASE_NOTES_1.6.4.md) · [范围与记录](docs/ZPAQ_RETRIEVAL_FREEZE_1.6.4.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.4)。正式安装与更新附件已生成。已由上方 v1.6.5 接续；方法 3 [来源调查](docs/ALZ_METHOD3_SOURCE_1.6.5.md)仍保留未完成。

**v1.6.3 · ZPAQ 受损块与文件影响定位**。按所选快照检查共享数据块，分清通过、受影响和未检查；明细、筛选与关联放进二级弹窗，普通/密码及原生组入口接通。本版只诊断定位。[中文说明](docs/RELEASE_NOTES_1.6.3.md) · [范围与记录](docs/ZPAQ_BLOCK_DIAGNOSIS_FREEZE_1.6.3.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.3)。已由上方 v1.6.4 接续。

**v1.6.2 · 中断备份的完整旧快照取回**。识别 ZPAQ 未完成事务，在当前栏选择完整旧快照另存取回；普通/密码、单包/已确认多部分贯通原队列、草稿与历史，结果明确排除中断更新。[中文说明](docs/RELEASE_NOTES_1.6.2.md) · [范围与记录](docs/ZPAQ_TAIL_RECOVERY_FREEZE_1.6.2.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.2)。已由上方 v1.6.3 接续。

**v1.6.1 · 原生 ZPAQ 多部分备份继续更新**。添加新资料，另存完整独立新组，保留原件、全部旧快照和未选内容；普通包、密码包、草稿恢复、失败重试和产物清单贯通。[中文说明](docs/RELEASE_NOTES_1.6.1.md) · [范围与记录](docs/ZPAQ_NATIVE_UPDATE_FREEZE_1.6.1.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.1)。已由上方 v1.6.2 接续。

**v1.6.0 · 界面与操作体验升级**。双栏内浏览归档与常用文本预览，全部页面的信息收纳、四种特色主题、完整导航快捷键、动态格式统计、更新进度和关闭到托盘修正。[中文说明](docs/RELEASE_NOTES_1.6.0.md) · [范围与记录](docs/UI_EXPERIENCE_FREEZE_1.6.0.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.6.0)。

**v1.5.64 · ZPAQ 原生多部分与旧快照取回**。明确确认来源组，普通/密码组接通浏览预览、完整检查、精确取回与快照转换，重启及失败重试保留原选择。[中文说明](docs/RELEASE_NOTES_1.5.64.md) · [范围与记录](docs/ZPAQ_NATIVE_MULTIPART_FREEZE_1.5.64.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.64)。原后续计划已顺延为上方 v1.6.1。

**v1.5.63 · ZIPX（XZ）继续整理与对比**。方法 95 接通原编辑、内容核对、密码分卷取材及精确取回；默认另存后缀和提示更准确，XZ 尾部与大小异常不误报完成，保留原交互。[中文说明](docs/RELEASE_NOTES_1.5.63.md) · [范围与记录](docs/ZIP_XZ_COMPATIBILITY_FREEZE_1.5.63.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.63)。

**v1.5.62 · PPMd ZIP 继续整理与对比**。方法 98 接入独立普通/密码包编辑、实际内容核对、完整分卷取材与精确取回；修正提前断流判断，异常恢复参数和不完整内容明确提示，保留原交互。[中文说明](docs/RELEASE_NOTES_1.5.62.md) · [范围与记录](docs/ZIP_PPMD_COMPATIBILITY_FREEZE_1.5.62.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.62)。ZIPX（XZ）工作区接续已由上方 v1.5.63 完成。

**v1.5.61 · Zstandard ZIP 继续整理与对比**。方法 93 接入独立普通/密码包编辑、实际内容核对、完整分卷取材及精确取回；原条目保留方法，新增/替换明确选择，外部字典、窗口或截断问题有处理提示，保留原交互。[中文说明](docs/RELEASE_NOTES_1.5.61.md) · [范围与记录](docs/ZIP_ZSTD_COMPATIBILITY_FREEZE_1.5.61.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.61)。PPMd 跨流程兼容已由上方 v1.5.62 接续。

**v1.5.60 · LZMA ZIP 继续整理与对比**。方法 14 接入独立普通/密码包编辑、实际内容核对、完整分卷取材及精确取回；结束标记、真实大小与属性异常按实际读取处理，保留原交互。[中文说明](docs/RELEASE_NOTES_1.5.60.md) · [范围与记录](docs/ZIP_LZMA_COMPATIBILITY_FREEZE_1.5.60.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.60)。Zstandard 跨流程兼容已由上方 v1.5.61 接续。

**v1.5.59 · Bzip2 ZIP 继续整理与对比**。独立普通/密码包编辑另存保留原方法，内容对比、完整分卷取材、精确取回和草稿/历史贯通；原包方法与新增/替换方法说明更清楚，损坏内容不误报可用。[中文说明](docs/RELEASE_NOTES_1.5.59.md) · [范围与记录](docs/ZIP_BZIP2_COMPATIBILITY_FREEZE_1.5.59.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.59)。LZMA 跨流程兼容已由上方 v1.5.60 接续。

**v1.5.58 · 接着阅读，挑选取回**。文件浏览器可继续最近阅读，保留位置与显示方式；找图面板搜索/分页、跨页勾选、资源库命中往返、弹窗取回及原队列/草稿/历史接续贯通。[中文说明](docs/RELEASE_NOTES_1.5.58.md) · [范围与记录](docs/READING_CONTINUITY_FREEZE_1.5.58.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.58)。Bzip2 跨流程兼容已由上方 v1.5.59 接续。

**v1.5.57 · 整包图片连续阅读**。按目录或整包自然排序，连续翻页/跳页/键盘、缩略导航、原始大小/缩放；读取/停止/跳过、来源变化刷新、密码分卷和当前图精确提取贯通。[中文说明](docs/RELEASE_NOTES_1.5.57.md) · [范围与记录](docs/IMAGE_READER_FREEZE_1.5.57.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.57)。阅读记忆、长列表和所选取回由上方 v1.5.58 接续。

**v1.5.56 · 所选资料组成新 ZIP**。在对比多选弹窗选择基础与补充，沿原编辑清单确认冲突、整理最终路径并另存；密码/完整分卷取材、引用草稿恢复与失败继续贯通。[中文说明](docs/RELEASE_NOTES_1.5.56.md) · [范围与记录](docs/COMPARISON_PACKAGE_FREEZE_1.5.56.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.56)。图片连续阅读由上方 v1.5.57 接续完成。

**v1.5.55 · 资料内容核对与密码分卷 ZIP**。逐侧读取完整内容，确认相同、不同或仅时间变化，显示实际进度，可停止后继续剩余项；接通 AES/ZipCrypto、数字与标准分卷，密码弹窗/保险箱、精确取回和草稿历史贯通。[中文说明](docs/RELEASE_NOTES_1.5.55.md) · [范围与记录](docs/SOURCE_CONTENT_FREEZE_1.5.55.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.55)。所选资料组包由上方 v1.5.56 接续完成。

**v1.5.54 · 资料对比与所选取回**。两份文件夹或普通 ZIP 按路径查看类型/大小/时间差异，左右独立跨页选择，多选弹窗统一配置后进入原队列取回；来源变化、草稿和历史保留精确范围。内容相同需后续核对。[中文说明](docs/RELEASE_NOTES_1.5.54.md) · [范围与记录](docs/SOURCE_COMPARISON_FREEZE_1.5.54.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.54)。内容核对和复杂 ZIP 接续由上方 v1.5.55 完成。

**v1.5.53 · 定时工作流与错过处理**。单次、每天或每周安排固定资料处理，直观看到下一次时间与最近结果；错过后明确补跑或跳过，忙碌时排队/跳过，同来源与目录事件不重复派发。密码、来源变化提醒和原有纠正接续贯通。[中文说明](docs/RELEASE_NOTES_1.5.53.md) · [范围与记录](docs/SCHEDULED_WORKFLOW_FREEZE_1.5.53.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.53)。工作流模块在声明范围完成，对比首阶段由上方 v1.5.54 接续。

**v1.5.52 · 目录触发与运行队列**。明确启用目录规则，等待复制稳定和分卷到齐后执行已保存方案；支持按文件/项目文件夹处理、暂停与取消，密码只在本次会话使用。队列保留方案快照，失败纠正后直达最新接续结果，重启保留等待与运行关联。[中文说明](docs/RELEASE_NOTES_1.5.52.md) · [范围与记录](docs/DIRECTORY_WORKFLOW_FREEZE_1.5.52.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.52)。定时与运行管理已由上方 v1.5.53 接续。

**v1.5.51 · 工作流纠正与接续**。从失败或指定步骤重跑，修改本次来源、目录、密码和参数；开始时按实际内容核对成功步骤，只复用仍匹配的产物。新记录关联原运行，重启后可手动继续，整组分卷传递补齐。[中文说明](docs/RELEASE_NOTES_1.5.51.md) · [范围与记录](docs/WORKFLOW_RECOVERY_FREEZE_1.5.51.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.51)。目录触发与运行队列已由上方 v1.5.52 接续，当前进度以最新记录为准。

**v1.5.50 · 手动顺序工作流**。保存、复制和编辑筛选/解压/压缩/转换方案，复用原参数面板与配置组；在运行弹窗确认来源和密码，按实际产物接续，逐步查看进度、日志和结果。失败或停止不派发后续步骤，重启保留运行摘要。[中文说明](docs/RELEASE_NOTES_1.5.50.md) · [范围与记录](docs/WORKFLOW_FREEZE_1.5.50.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.50)。失败纠正与指定步骤重跑已由上方 v1.5.51 接续，当前进度以最新记录为准。

**v1.5.49 · 常用资料整理与跨包批量取回**。来源标签/收藏、保存搜索与查找位置接续，跨页勾选后在弹窗统一设置输出、目录结构和同名处理；可用部分先加入原队列，密码/离线/缺卷来源纠正后只继续剩余项，草稿与历史保留所选范围。[中文说明](docs/RELEASE_NOTES_1.5.49.md) · [范围与记录](docs/LIBRARY_RETRIEVAL_FREEZE_1.5.49.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.49)。工作流第一阶段已由上方 v1.5.50 接续，当前进度以最新记录为准。

**v1.5.48 · 资料库维护、密码包与分卷来源**。资料库接入现有可浏览格式、密码与整组分卷，复用保险箱和原预览/提取；离线资料保留上次索引并标明来源状态，支持重新定位、仅更新变化来源与批次暂停/继续，密码仅在当前会话使用。[中文说明](docs/RELEASE_NOTES_1.5.48.md) · [范围与记录](docs/LIBRARY_MAINTENANCE_FREEZE_1.5.48.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.48)。分类、收藏与跨包批量取回已由上方 v1.5.49 接续，当前进度以最新记录为准。

**v1.5.47 · 归档资源库与跨包搜索**。主动加入 ZIP/TAR 或扫描指定目录确认候选，手动建库后按文件名/包内路径跨包查找，按来源与类型筛选；命中沿原浏览器定位、预览或单项取回。可停止、重启保留索引，缺失/变化来源先处理，移出只删索引。[中文说明](docs/RELEASE_NOTES_1.5.47.md) · [范围与记录](docs/ARCHIVE_LIBRARY_FREEZE_1.5.47.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.47)。索引维护、密码包与分卷来源已由上方 v1.5.48 接续，当前进度以最新记录为准。

**v1.5.46 · 密码 ZIP 编辑与草稿恢复**。ZIP 工作区接通 AES/ZipCrypto 密码内容、保留/更换/取消输出加密与只改密码另存；未完成修改自动存为本地草稿，重启后定位缺失或变化的来源，重新填写密码再继续。已保存的草稿返回原结果，失败与原件保留。[中文说明](docs/RELEASE_NOTES_1.5.46.md) · [范围与记录](docs/ZIP_PASSWORD_DRAFTS_FREEZE_1.5.46.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.46)。编辑工作区三阶段在该版收尾，资源库已由上方 v1.5.47 接续，当前进度以最新记录为准。

**v1.5.45 · ZIP 目录整理与批量修改**。在原编辑入口按目录整理待保存内容，删除、改名、移动与批量名称规则先预览再加入清单，与添加/替换统一撤销和另存；原包保留，失败后可继续，新包沿原浏览/预览/提取使用。[中文说明](docs/RELEASE_NOTES_1.5.45.md) · [范围与记录](docs/ZIP_ORGANIZE_FREEZE_1.5.45.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.45)。密码 ZIP 和跨次编辑草稿已由上方 v1.5.46 接续，当前进度以最新记录为准。

**v1.5.44 · ZIP 内容编辑第一阶段**。在原浏览器添加文件/目录、拖入或右键替换，明确处理同名项，在修改清单复核、撤销后另存新 ZIP；原包、未改条目与空目录保留，失败/停止可调整继续，新包可直接浏览、预览和提取。首版支持独立、未加密的 Store/Deflate ZIP。[中文说明](docs/RELEASE_NOTES_1.5.44.md) · [范围与记录](docs/ZIP_EDIT_FREEZE_1.5.44.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.44)。目录整理已由上方 v1.5.45 接续，当前进度以最新记录为准。

**v1.5.43 · 批处理与结果体验**。压缩、解压与转换按状态分类找任务，原多选弹窗显示所选清单与修改摘要；成功结果直接沿原入口操作，无法打开的目录可单独重试。单条展开、多选弹窗、自然继承和等宽布局保持。[中文说明](docs/RELEASE_NOTES_1.5.43.md) · [范围与记录](docs/BATCH_RESULTS_FREEZE_1.5.43.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.43)。ZIP 编辑已由上方 v1.5.44 接续，当前进度以最新记录为准。

**v1.5.42 · 失败后调整并继续**。当前压缩/转换详情显示原因与下一步；重名时建议新名称，转换分别确认原包和新包密码。任务栏重试带入原配置创建待处理项，保留原失败、原文件和已有结果，由用户确认启动。单条展开、多选弹窗及等宽/窄窗布局保持。[中文说明](docs/RELEASE_NOTES_1.5.42.md) · [范围与记录](docs/FAILURE_RECOVERY_FREEZE_1.5.42.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.42)。批处理与结果体验已由上方 v1.5.43 接续，当前进度以最新记录为准。

**v1.5.41 · 按用途配置压缩**。兼容分享、体积优先、加密分享、按大小分包四种方案，在原配置位置显示实际参数并确认应用；密码、目录和原包凭据等未指定项保留。压缩与转换共用，单条展开、多选弹窗、自然继承和等宽/窄窗布局保持。[中文说明](docs/RELEASE_NOTES_1.5.41.md) · [范围与记录](docs/PURPOSE_PRESETS_FREEZE_1.5.41.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.41)。失败纠正已由上方 v1.5.42 接续，后续进度以最新记录为准。

**v1.5.40 · 原生 SWM 创建**。新增仅存储、XPRESS、LZX 非固实资源分包，生成 name.swm、name2.swm 等整组结果；接通密码原包转换、任意成员读取和补卷接续，保留单条展开、多选弹窗与等宽/窄窗布局。单个资源可能超过目标卷大小，无原生密码，不保证系统部署。[中文说明](docs/RELEASE_NOTES_1.5.40.md) · [范围与记录](docs/NATIVE_SWM_CREATE_FREEZE_1.5.40.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.40)。用途配置已由上方 v1.5.41 接续，后续进度以最新记录为准。

**v1.5.39 · 固实 WIM / ESD**。WIM 增加 LZMS 固实选择，ESD 固定真实固实归档；数字分卷、密码原包双向转换、浏览预览、提取与补卷接续。方法随配置组、模板、草稿、历史和多选保存，原交互与等宽/窄窗布局保持。[中文说明](docs/RELEASE_NOTES_1.5.39.md) · [范围与记录](docs/WIM_SOLID_FREEZE_1.5.39.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.39)。原生 SWM 创建已由上方 v1.5.40 接续，当前进度以最新记录为准。 [近期排期 v1.5.40～43](docs/ARCHIVE_ROADMAP_1.5.40_TO_1.5.43.md) · [五模块分阶段路线与主线回归 v1.5.44～59](docs/ARCHIVE_ROADMAP_1.5.44_TO_1.5.59.md)。

**v1.5.38 · ZIP LZMA 创建**。增加可选方法，按强度显示字典，接通密码、数字/标准卷、转换、浏览提取与补卷流程。真实编码采样、暂停停止、草稿/历史/模板和多选保留方法，原交互与等宽/窄窗布局保持。[中文说明](docs/RELEASE_NOTES_1.5.38.md) · [范围与记录](docs/ZIP_LZMA_FREEZE_1.5.38.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.38)。下一版 [v1.5.39 固实 WIM / ESD](docs/ARCHIVE_ROADMAP_1.5.39.md)已由 v1.5.39 接续，当前进度以上方最新记录为准。

**v1.5.37 · ZPAQ 快照对比与变化提取**。查看两版之间的新增、候选、元数据和移除，按需核对内容，再按版本创建所选提取任务。密码数字卷、草稿、补卷重试和历史重做保持范围与来源，原交互和布局不变。[中文说明](docs/RELEASE_NOTES_1.5.37.md) · [范围与记录](docs/ZPAQ_DIFF_FREEZE_1.5.37.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.37)。下一版 [v1.5.38 ZIP LZMA创建](docs/ARCHIVE_ROADMAP_1.5.38.md)已由 v1.5.38 接续，当前进度以上方最新记录为准。

**v1.5.36 · ZPAQ 增量更新**。往已有备份添加与更新内容，显示归档内位置，保留原件、未选内容与全部历史；普通/密码及数字卷、草稿/重做/失败纠正和新旧快照读取贯通。未变化时明确说明未新增快照，保持原交互与布局。[中文说明](docs/RELEASE_NOTES_1.5.36.md) · [范围与记录](docs/ZPAQ_INCREMENTAL_FREEZE_1.5.36.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.36)。下一版 [v1.5.37 快照对比与变化提取](docs/ARCHIVE_ROADMAP_1.5.37.md)已由 v1.5.37 接续，当前进度以上方最新记录为准。

**v1.5.35 · ZIP PPMd 创建**。为文本资料归档增加可选方法，就近显示内存预算，接通密码、数字/标准卷、转换与补卷后续流程。实际体积分析、暂停恢复、草稿/历史和多选保持方法，原交互与等宽/窄窗布局不变。[中文说明](docs/RELEASE_NOTES_1.5.35.md) · [范围与记录](docs/ZIP_PPMD_FREEZE_1.5.35.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.35)。下一版 [v1.5.36 ZPAQ增量更新](docs/ARCHIVE_ROADMAP_1.5.36.md)已由 v1.5.36 接续，当前进度以上方最新记录为准。

**v1.5.34 · ZIP BZip2 创建**。常用 ZIP 增加 BZip2，接通普通/AES-256/ZipCrypto、数字/标准卷、转换与后续提取/补卷。体积分析和强度显示跟随实际方法，转换方法与格式并排；修复较长路径下分卷结果提交失败。原交互与等宽/窄窗布局保持。[中文说明](docs/RELEASE_NOTES_1.5.34.md) · [范围与记录](docs/ZIP_BZIP2_FREEZE_1.5.34.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.34)。下一版 [v1.5.35 ZIP PPMd创建](docs/ARCHIVE_ROADMAP_1.5.35.md)已由 v1.5.35 接续，当前进度以上方最新记录为准。

**v1.5.33 · WIM LZMS 非固实创建**。现有方法选择增加 LZMS，小块与单线程控制编码资源；文件目录、密码原包转换、数字卷与后续提取/补卷/结果接通。方法跟随配置组、草稿和历史，转换时方法与目标格式并排，减少寻找与滚动。保留原交互和等宽布局。[中文说明](docs/RELEASE_NOTES_1.5.33.md) · [范围与记录](docs/WIM_LZMS_FREEZE_1.5.33.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.33)。下一版 [v1.5.34 ZIP BZip2创建](docs/ARCHIVE_ROADMAP_1.5.34.md)已由 v1.5.34 接续，当前进度以上方最新记录为准。

**v1.5.32 · WIM LZX 体积优先创建**。现有方法选择增加 LZX，接通文件/目录、密码原包转换、数字分卷与后续浏览、预览、检查、提取、补卷和统一结果；配置组/模板、草稿、历史保留方法，混合批量修改不改变无关任务的继承关系。原交互与等宽/窄窗布局保持。[中文说明](docs/RELEASE_NOTES_1.5.32.md) · [范围与记录](docs/WIM_LZX_FREEZE_1.5.32.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.32)。下一版 [v1.5.33 LZMS非固实创建](docs/ARCHIVE_ROADMAP_1.5.33.md)已由 v1.5.33 接续，当前进度以上方最新记录为准。

**v1.5.31 · WIM XPRESS 压缩创建**。在 WIM 方法中选择快速压缩，批量配置、配置组/模板、草稿、历史重做和转换保留方法；密码原包可转为 XPRESS WIM 数字卷，完成源数据与压缩比统计接通。保留原交互与等宽/窄窗布局。[中文说明](docs/RELEASE_NOTES_1.5.31.md) · [范围与记录](docs/WIM_XPRESS_FREEZE_1.5.31.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.31)。下一版 [v1.5.32 WIM LZX创建](docs/ARCHIVE_ROADMAP_1.5.32.md)已由 v1.5.32 接续，当前进度以上方最新记录为准。

**v1.5.30 · ZPAQ 历史快照**。按时间选择旧版，预览、检查和取回后来删除的文件；密码数字卷、转换、草稿和失败重试保留各自快照，来源变化明确提示。单条展开、多选弹窗与等宽/窄窗布局保持。[中文说明](docs/RELEASE_NOTES_1.5.30.md) · [范围与记录](docs/ZPAQ_SNAPSHOTS_FREEZE_1.5.30.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.30)。下一版 [v1.5.31 WIM XPRESS创建](docs/ARCHIVE_ROADMAP_1.5.31.md)已由 v1.5.31 接续，当前进度以上方最新记录为准。

**v1.5.29 · 原生 SWM 只读**。从 install.swm、install2.swm 等任意成员浏览多镜像、预览、检查和提取；Copy/XPRESS/LZX 直接跨卷读取。缺首/中/尾卷显示具体文件名，补齐后建新任务，内容可转换为密码 ZIP。保留单条展开、多选弹窗与等宽/窄窗布局，不新增 SWM 创建、WIM 密码或系统部署。[中文说明](docs/RELEASE_NOTES_1.5.29.md) · [范围与记录](docs/NATIVE_SWM_FREEZE_1.5.29.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.29)。下一版 [v1.5.30 ZPAQ 历史快照读取](docs/ARCHIVE_ROADMAP_1.5.30.md)已由 v1.5.30 接续，当前进度以上方最新记录为准。

**v1.5.28 · 资源管理器校验入口**。从经典右键带入文件生成清单，或带入 SFV/MD5/SHA256 逐份核对；确认后才读取，已有工作可保留、替换或合并。长路径优先显示清单名称，文件读取失败不再提示绿色成功。保留原有交互与窄窗滚动。[中文说明](docs/RELEASE_NOTES_1.5.28.md) · [范围与记录](docs/EXPLORER_CHECKSUMS_FREEZE_1.5.28.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.28)。本轮三版按范围收口；[v1.5.29 原生 SWM 只读计划](docs/ARCHIVE_ROADMAP_1.5.29.md)已由 v1.5.29 接续，当前进度以上方最新记录为准。

**v1.5.27 · WIM 数字分卷**。按指定大小拆卷，从任意成员浏览多镜像目录、预览文本/图片、检查和提取；支持密码 ZIP 双向转换、补卷新任务和整组结果清单。新建 WIM 明确为存储与去重，停用无效强度选项；WIM 无原生密码，数字卷不是 SWM。保留单条展开、多选弹窗和等宽/窄窗布局。[中文说明](docs/RELEASE_NOTES_1.5.27.md) · [范围与记录](docs/WIM_VOLUMES_FREEZE_1.5.27.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.27)。下一步按[既定路线](docs/ARCHIVE_ROADMAP_1.5.26_TO_1.5.28.md)推进 v1.5.28 资源管理器校验入口，已由上方版本接续。

**v1.5.26 · ZPAQ 密码数字分卷**。按指定大小拆卷，从任意成员浏览、预览、检查和提取；密码 ZIP 双向转换、补卷新任务和整组结果清单接通。修复继承分卷与多选格式配置，单条展开、多选弹窗和等宽/窄窗布局保持。[中文说明](docs/RELEASE_NOTES_1.5.26.md) · [范围与记录](docs/ZPAQ_VOLUMES_FREEZE_1.5.26.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.26)。下一步按[既定路线](docs/ARCHIVE_ROADMAP_1.5.26_TO_1.5.28.md)推进 WIM 数字卷；已由 v1.5.27 接续，当前进度以上方最新记录为准。

**v1.5.25 · 从结果生成校验清单**。完成任务、历史和转换结果可带入整组实际产物，主动计算并导出相对路径清单；同名文件保留子目录，中断项不写入，导出后可直接核对。保留单条展开、多选弹窗和等宽/窄窗布局。[中文说明](docs/RELEASE_NOTES_1.5.25.md) · [范围与记录](docs/RESULT_CHECKSUMS_FREEZE_1.5.25.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.25)。本轮三版计划按范围收口，下一阶段 [v1.5.26～v1.5.28](docs/ARCHIVE_ROADMAP_1.5.26_TO_1.5.28.md) 已规划，ZPAQ 数字卷由 v1.5.26 接续；当前进度以上方最新记录为准。

**v1.5.24 · 多文件 LZMA 与密码组合**。新增 TAR.LZMA、LZMA.AES 和 TAR.LZMA.AES，接通数字分卷、认证后浏览/预览、检查、所选/整包提取及密码 ZIP 互转；多选弹窗保留各条密码与分卷，单条展开和等宽/窄窗布局保持。[中文说明](docs/RELEASE_NOTES_1.5.24.md) · [范围与记录](docs/TAR_LZMA_AES_FREEZE_1.5.24.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.24)。结果生成相对路径校验清单已由 v1.5.25 接续；当前进度以上方最新记录为准。

**v1.5.23 · LZMA 数字分卷**。LZMA 单文件支持数字切卷创建，从任意成员浏览、有界预览、检查和提取；密码 ZIP 与单文件 LZMA 分卷转换、补卷新任务及结果接通。保留单条展开、多选弹窗、自然继承、等宽与窄窗布局。[中文说明](docs/RELEASE_NOTES_1.5.23.md) · [范围与记录](docs/LZMA_VOLUMES_FREEZE_1.5.23.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.23)。下一步按[既定路线](docs/ARCHIVE_ROADMAP_1.5.23_TO_1.5.25.md)推进 TAR.LZMA 与 Long AES 组合，已由 v1.5.24 接续；v1.5.25 结果生成校验清单尚未开发，当前进度以上方最新记录为准。

**v1.5.22 · 校验进度与继续未完成项**。计算摘要与核对清单显示真实字节和当前文件，可停止后主动继续；完成观察留在工作区，中断项不计匹配，清单变化作废旧结果。保持原有交互和窄窗显示。[中文说明](docs/RELEASE_NOTES_1.5.22.md) · [范围与记录](docs/CHECKSUM_PROGRESS_FREEZE_1.5.22.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.22)。后续 [v1.5.23～v1.5.25](docs/ARCHIVE_ROADMAP_1.5.23_TO_1.5.25.md) 已规划：LZMA 数字卷已由 v1.5.23 接续，TAR/Long AES 组合及结果生成校验清单尚未开发；当前进度以上方最新记录为准。

**v1.5.21 · 六种新 Long AES 组合**。LZ4/Brotli/LZIP 及对应 TAR 组合支持密码、数字分卷、认证后浏览/预览、检查、提取、转换与结果；批量弹窗与补卷新任务接通，普通 LZIP 完整性分派修正。保持原有交互、等宽及窄窗布局。[中文说明](docs/RELEASE_NOTES_1.5.21.md) · [范围与记录](docs/AES_CODECS_FREEZE_1.5.21.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.21)。下一步按[既定路线](docs/ARCHIVE_ROADMAP_1.5.20_TO_1.5.22.md)推进校验进度、停止与继续，已由 v1.5.22 接续，当前进度以上方最新记录为准。

**v1.5.20 · LZIP 数字分卷**。LZIP/TAR.LZIP 支持拆卷创建，从任意卷浏览、预览、检查与提取；密码 ZIP 和 TAR.LZIP 分卷转换接通。批量改格式保留分卷设置，补卷后沿原入口创建新任务；单条展开、多选弹窗、左右等宽和窄窗滚动保持。[中文说明](docs/RELEASE_NOTES_1.5.20.md) · [范围与记录](docs/LZIP_VOLUMES_FREEZE_1.5.20.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.20)。下一版按[既定路线](docs/ARCHIVE_ROADMAP_1.5.20_TO_1.5.22.md)Long AES 内层扩展已由 v1.5.21 接续，当前进度以上方最新记录为准。

**v1.5.19 · 看清哪一卷需要补回**。校验清单逐项显示匹配、缺失、内容不符和读取原因；单条展开完整路径及等宽摘要，复制待补列表，按需复查所选或问题项。清单变化会作废旧结果，未检查项不计匹配；可接续原诊断与失败新任务，保留原交互和窄窗滚动。[中文说明](docs/RELEASE_NOTES_1.5.19.md) · [范围与记录](docs/CHECKSUM_DETAILS_FREEZE_1.5.19.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.19)。原三项计划按范围收口，下一阶段[v1.5.20～v1.5.22](docs/ARCHIVE_ROADMAP_1.5.20_TO_1.5.22.md)已规划，LZIP 数字分卷已由 v1.5.20 接续，当前进度以上方最新记录为准。

**v1.5.18 · 自有 AES 密码分卷**。九种自有 AES 格式可以拆卷创建，从任意一卷进入浏览、预览和提取；密码 ZIP 与 TAR.AES 转换接通重新拆卷或合并。缺卷/后段内容异常提示更明确，补卷后通过原弹窗创建新任务，旧记录保留；修复 TAR.AES 多源目录与空目录。单条展开、多选弹窗、自然继承、左右等宽及窄窗滚动保持。[中文说明](docs/RELEASE_NOTES_1.5.18.md) · [实施与范围](docs/AES_VOLUMES_1.5.18.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.18)。已由 v1.5.19 接续，当前下一步以上方最新记录为准。

**v1.5.17 · 多段压缩流完整读取**。补齐 GZIP/BZIP2/XZ 连续流与跨段 TAR 末尾内容；后段损坏明确提示，普通压缩文件可以直接浏览和预览。从数字分卷中间卷进入也能查看末尾文件，补卷后的新任务与密码 ZIP 转换接续原流程。单条展开、多选弹窗、自然配置继承、左右等宽及窄窗滚动保持。[中文更新说明](docs/RELEASE_NOTES_1.5.17.md) · [实施与范围](docs/MULTISTREAM_1.5.17.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.17)。当时后续路线已由 v1.5.18 接续，当前下一步以上方最新记录为准。

**v1.5.16 · 读写故障与任务接续**。处理中盘满、文件占用和权限问题保留真正原因；后续卷被撤回或替换时明确提示来源变化，不发布本次不完整结果。修复提交失败后旧暂存无法接续的问题，恢复条件后通过原纠正入口创建新任务，保留旧失败历史。单条展开、多选弹窗、左右等宽与窄窗滚动保持。[中文更新说明](docs/RELEASE_NOTES_1.5.16.md) · [实施与范围](docs/IO_RECOVERY_1.5.16.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.16)。真实 VHD 盘满补测未执行，未计通过；下一阶段已写入[开发路线](docs/ARCHIVE_ROADMAP_1.5.17_TO_1.5.19.md)：多段压缩流、自有 AES 容器分卷、逐文件校验原因。

**v1.5.15 · ZIP64 大文件与密码分卷修复**。压缩/转换保留独立空文件夹，明确 ZIP64 目录损坏，救援密码问题不再盖过结构损坏原因。超过 4 GiB 的真实文件覆盖普通/AES/传统密码、单包/数字卷/标准卷及跨卷数据描述符，实际改密码转换与独立完整回解通过。保留单条展开、多选弹窗与原布局。[中文更新说明](docs/RELEASE_NOTES_1.5.15.md) · [封板快照](docs/ZIP64_VOLUMES_FREEZE_1.5.15.md) · [发布与下载](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.15)。该项已由 v1.5.16 接续；ALZ 方法 3、EGG 加密固实的未完成状态保留。

此前 **v1.5.14 已正式发布**，[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.14)。统一交付 GNU/PAX 稀疏 TAR 的真实名称、完整内容、稀疏计量和失败原因，并修正真实 ZIP/TAR 被误后缀带偏的浏览、预览、检查及提取路由。二十原件独立回解、203 Rust、170 前端、36 桌面记录、18 张实看页面和普通生产通过；正式四附件及中文更新说明已回下载核对。见[审计](docs/TAR_ROUTING_1.5.14.md)、[封板发布](docs/TAR_ROUTING_FREEZE_1.5.14.md)及[中文说明](docs/RELEASE_NOTES_1.5.14.md)。v1.5.13 已撤回未发布，原标签不移动。图标与原截图保留；Windows 为普通填零输出，不承诺 NTFS 稀疏分配或 Linux 元数据。此为 v1.5.14 交付快照，当前状态以上方最新版本和开发交接为准。

此前 **v1.5.12 已正式发布**，[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.12)。修复 ALZ Deflate 在声明压缩范围尚有尾部数据时误报完整成功；加密条目全部内容大小/CRC 匹配后明确报尾部损坏并停止试密，未匹配仍保留不确定类别。八组原厂普通/中文密码、单包/原生卷独立回解通过，八派生故障不再错误成功；105 Rust、170 前端、二十二桌面记录、十四张实看页面及普通生产通过。正式四资产、中文说明、包内引擎二十组原件/八故障/124 份许可已核对。见[审计](docs/ALZ_STREAM_INTEGRITY_1.5.12.md)、[封板记录](docs/ALZ_STREAM_INTEGRITY_FREEZE_1.5.12.md)和[更新说明](docs/RELEASE_NOTES_1.5.12.md)。原计划 Obfuscated Deflate（方法 3）未取得原厂正面包，仍不支持且保留未完成；不冒称新增该方法。图标和原截图保留；此为前版交付快照，最新状态与下一步以上方 v1.5.14 及开发交接为准。

此前 **v1.5.11 已正式发布**，[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.11)。补齐二十组原中文 ALZip 密码 EGG 的 ZipCrypto/AES/LEA、Deflate/LZMA、单包与原生卷验证，改善密码编码不匹配后的最终原因和重试指引；用户主动选择 CP936 后以新任务继续，旧失败保留。101 Rust、170 前端、二十五桌面记录和十二张实看页面通过；正式四资产、中文说明、包内引擎五十五组原包与 103 份许可已核对。见[审计](docs/EGG_PASSWORD_COMBINATIONS_1.5.11.md)、[封板记录](docs/EGG_PASSWORD_COMBINATIONS_FREEZE_1.5.11.md)和[更新说明](docs/RELEASE_NOTES_1.5.11.md)。加密固实无真实正面原包，不承诺支持；两组 LEA/LZMA 原生卷由原厂回解核对，不承诺本机 Bandizip 互通。默认编码和 helper 不变，图标与原截图保留；十版路线已交付六版、剩四版，下一版 v1.5.12 补 ALZ 方法兼容。

此前 **v1.5.10 已正式发布**，[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.10)。大 EGG LZMA 改为有界流式读取，超过旧 256 MiB 压缩输入的普通、固实和传统密码原生四卷已验证；完整 CRC、缺卷最终原因和事务输出保持。99 项 Rust、152 项前端、三组大原包、七条桌面流程和十六张实看页面通过，实际写出后暂停/恢复/取消及历史最终原因已核对。见[审计](docs/EGG_LARGE_LZMA_1.5.10.md)、[封板记录](docs/EGG_LARGE_LZMA_FREEZE_1.5.10.md)和[更新说明](docs/RELEASE_NOTES_1.5.10.md)。正式四资产、中文说明、包内引擎的 35 组原包和 103 份许可已核对。EGG 不创建，LZMA 字典仍限 256 MiB，未验证密码组合不外推。图标和原截图保留；十版路线已交付五版、剩五版，下一版 v1.5.11 补特殊 EGG 密码组合。

此前 **v1.5.9 已正式发布**，[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.9)。加密 RAR 缺卷保留实际卷名和补回位置；前段密码通过后的后续数据异常不再一概当错密，诊断也不再给出不适用的修复工具提示。十二组真原包、96 项 Rust、152 项前端、39 桌面记录及十四张实看页面通过，补回原卷新任务、旧失败保留与转换独立回解已核对。见[审计](docs/RAR_FINAL_CAUSES_1.5.9.md)、[封板记录](docs/RAR_FINAL_CAUSES_FREEZE_1.5.9.md)与[更新说明](docs/RELEASE_NOTES_1.5.9.md)。正式四资产、中文更新说明和包内主程序/四份说明已回下载核对。前段密码通过不等于整包完整；不新增 RAR 创建或修复，不执行输入 EXE。图标和原截图保留。

此前 **v1.5.7 已正式发布**，[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.7)。补齐 7Z 中 PPMdH、Deflate、Deflate64、Bzip2 的原生读取，贯通密码、固实、数字分卷、所选提取及转换；改进损坏、方法不支持和输出错误的最终原因。61 项 Rust、150 项前端、25 个真实桌面场景与 8 张实看页面通过；正式四资产、中文说明及包内主程序/四份说明已回下载核对。见[审计](docs/SEVENZ_METHODS_1.5.7.md)、[封板记录](docs/SEVENZ_METHODS_FREEZE_1.5.7.md)和[更新说明](docs/RELEASE_NOTES_1.5.7.md)。不改变默认压缩算法，软件图标和原截图保留。

此前 **v1.5.6 已正式发布**，[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.6)。RAR 进度改用实际解码字节，修正跨卷总量、暂停计时、同目录产出统计和停止误报失败。49 项 Rust、133 项前端、12 个真实桌面场景与 8 张实看页面通过；正式四资产、中文说明及包内主程序已回下载核对。见[本版审计](docs/RAR_PROGRESS_1.5.6.md)、[封板发布记录](docs/RAR_PROGRESS_FREEZE_1.5.6.md)与[更新说明](docs/RELEASE_NOTES_1.5.6.md)。后续十版路线已交付一版，下一版补齐真实 7Z 方法兼容；原图标和截图保留。

此前 **v1.5.5** 已正式发布，[下载安装包](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.5.5)。补齐真实 RAR4/5 外置卷 SFX 与固定 7-Zip 24.09 GUI 加载器读取，修复 RAR 密码判断、块内暂停取消及取消后的文件锁；文件管理器中卷浏览、补卷新任务和历史重做完成。94 项 Rust、214 项前端、十五组原包、24 个桌面场景和 18 张实看页面通过。正式四资产、中文说明及包内资源已回下载核对；见[审计](docs/SFX_VOLUMES_1.5.5.md)、[封板记录](docs/SFX_VOLUMES_FREEZE_1.5.5.md)与[更新说明](docs/RELEASE_NOTES_1.5.5.md)。SFX 只读、不执行输入 EXE，不创建/修复或处理恢复卷，未验证模块不外推。原十二版路线已全部交付，接[后续十版目标](docs/ARCHIVE_ROADMAP_1.5.6_TO_1.5.15.md)，先改善 RAR 实际字节进度。软件图标和既有截图保留。

此前 **v1.5.3** 完成 EGG 基础方法、传统 ZipCrypto、原生卷与明确密码编码的只读闭环，见[当时审计](docs/EGG_BASIC_1.5.3.md)与[更新说明](docs/RELEASE_NOTES_1.5.3.md)；其复杂方法限制是当时状态，现以 v1.5.4 的实际组合和资源边界为准。

此前 **v1.5.2** 新增 ALZ 只读引擎，贯通普通/传统密码/原生分卷的浏览、所选与整包提取、检查、转换及失败接续。默认 CP949，可明确选择原工具代码页；历史与重试保留编码，不保存密码或局部选择。73 项不同 Rust、235 项前端、真实桌面与独立载荷核对通过，22 张页面已查看；正式四资产、中文说明及包内引擎已核对。见[实施审计](docs/ALZ_READ_1.5.2.md)、[封板记录](docs/ALZ_READ_FREEZE_1.5.2.md)及[更新说明](docs/RELEASE_NOTES_1.5.2.md)。ALZ 不提供创建、损坏修复或缺卷部分救援。

**v1.5.2**：ALZ 只读、传统密码和原生分卷、编码选择及失败接续，见[更新说明](docs/RELEASE_NOTES_1.5.2.md)与[封板记录](docs/ALZ_READ_FREEZE_1.5.2.md)。

此前 **v1.4.20**：扩展真实好压 ZipCrypto 论坛 ZIP 的缺卷部分抢救，逐文件区分已恢复、涉及缺卷、校验失败与无法确认；保留原任务设置继续解压。40 项不同 Rust、29 项前端和真实桌面流程通过，12 张常用/窄窗截图已查看。PR #179 已受保护合入，正式四资产、安装器版本及中文更新说明已回下载核对。见[实施审计](docs/ZIP_PARTIAL_RESCUE_1.4.20.md)、[封板记录](docs/ZIP_PARTIAL_RESCUE_FREEZE_1.4.20.md)、[更新说明](docs/RELEASE_NOTES_1.4.20.md)及[正式 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.20)。软件图标及原截图保留。

**v1.4.18** 的 ZIPX（XZ）标准多卷、密码创建、改密/重新分卷/合卷及缺卷补回继续保留。产品输出已用 WinRAR 7.23 独立回解，Bandizip Beta 解码异常及 7-Zip 卷名限制明确保留，原数字卷默认不变。见[ZIPX 审计](docs/STANDARD_ZIPX_1.4.18.md)和[更新说明](docs/RELEASE_NOTES_1.4.18.md)。

**v1.4.17**：好压论坛 7Z 密码分卷、缺卷与损坏准确分类、补卷继续和输出名称修正。见[更新说明](docs/RELEASE_NOTES_1.4.17.md)、[封板记录](docs/SPECIAL_VOLUME_FREEZE_1.4.17.md)及 [Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.17)。

**v1.4.15**：减少 ZIP 重复预检，TAR.LZ4 / TAR.BR 流式创建，改善 RAR 预检查暂停/取消和矮窗口归档列表。保留完整校验，不承诺整条流程固定百分比提速；见 [实施审计](docs/ARCHIVE_PERFORMANCE_1.4.15.md)、[封板记录](docs/ARCHIVE_PERFORMANCE_FREEZE_1.4.15.md)、[中文更新说明](docs/RELEASE_NOTES_1.4.15.md)及 [正式 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.15)。

**原 v1.4.14 候选随 v1.4.15 交付**：ZIP Zstandard 和 ZIPX（XZ）创建接通密码、数字分卷和转换，保留批量、配置组、模板、预设、草稿及历史的方法。ZIP Zstandard 另支持标准分卷，ZIPX 标准分卷暂不提供。原始证据见 [实施记录](docs/ZIP_OUTPUT_METHODS_1.4.14.md)和 [封板记录](docs/ZIP_OUTPUT_METHODS_FREEZE_1.4.14.md)。

**v1.4.13** 增加标准 ZIP 缺卷部分抢救、密码包救援与恢复清单，打通失败任务救援后继续解压，见 [更新说明](docs/RELEASE_NOTES_1.4.13.md)和 [正式 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.13)。缺卷救援要求目录仍完整可读，不补造缺失数据。

**v1.4.12** 新增 LZIP / TAR.LZIP 的完整用户流程及错误提示改善，见 [更新说明](docs/RELEASE_NOTES_1.4.12.md)和 [正式 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.4.12)。该版不提供密码或分卷创建；数字分卷已在 v1.5.20 补齐。

v1.4.11 可以直接处理 ZIP/7Z/RAR 自解压包，增加好压论坛 ZIP 的明文与已核对的密码分卷读取，修正部分跨卷正确密码误判，并完善缺卷提示及补回重试；不创建 SFX，不新增好压 7Z 论坛卷支持。

v1.4.10 统一 ZIP 选择性解压与密码范围，补上旧 ZIP 方法、Zstandard 密码卷型和真实超过 4 GiB 的 ZIP64 密码数字卷证据。

v1.4.9 保留批量检查修复的切页状态，重启后可手动恢复工作区和补填密码继续处理；修正 RAR 管道密码输入编码，并改善大文件完整性校验的取消响应。没有新增格式或扩大任意缺卷救援的承诺。

v1.4.8 扩展完整标准 ZIP 分卷与普通流式 Deflate 丢目录救援；RAR 分包可附带或补充创建 `.rev`。多包检查自动整理卷组，多选配置在弹窗修改，恢复结果可统一加入解压队列。

v1.4.7 已正式发布，接通本机 RAR 工具的恢复记录修复与 `.rev` 补卷，恢复结果保留密码并可继续解压；恢复保护配置覆盖多选、转换、模板、历史和草稿。Windows 右键可直接进入检查页面，保持单条展开、多选弹窗。

v1.4.6 已正式发布，将失败任务的补卷、换来源和密码纠正放在单条展开区，多选仍使用弹窗；支持边界明确的密码 ZIP 丢目录救援，新包可保留输出设置加入解压队列。压缩/解压详情统一等宽、紧凑摘要和可滚动日志，并修复小窗口文字重叠。

v1.4.5 接通目录可读的密码 ZIP 救援，新包保留填写的密码并使用 AES-256 / UTF-8；新增 ARJ、Unix `.Z`、UDF 读取及双栏/右键入口。新增结果加密提示、定位与复制，保留单条展开和多选弹窗。范围与实际检查见 [本版记录](docs/PASSWORD_ZIP_RESCUE_AND_FORMATS_1.4.5.md)和 [封板记录](docs/PASSWORD_ZIP_RESCUE_FREEZE_1.4.5.md)。

v1.4.4 新增真正标准 ZIP 分卷创建及配置组/历史/模板卷型保存，补齐 PPMd 与加密 ZIPX、直接选择 RAR 中间卷的兼容，修正大文件按小终卷误判容量比率；ZIP 目录丢失时恢复可信的完整条目，部分恢复明确提示。真实桌面、原始失败和范围边界见 [本版审计](docs/ARCHIVE_RELIABILITY_1.4.4.md)及 [封板记录](docs/ARCHIVE_RELIABILITY_FREEZE_1.4.4.md)。数字切卷和密码编码默认值保持不变；密码 ZIP、缺卷和未知边界不承诺修复。

v1.4.2 支持一次转换多个压缩包、关闭后恢复尚未开始的归档任务，并分别提示补填原包和新包密码。所选任务批量配置使用弹窗，单条任务继续行内展开；设置中心按设计稿统一布局，保留全部设置和即时生效行为。详情见 [实施记录](docs/BATCH_CONVERSION_AND_DRAFT_RECOVERY_PLAN.md)与 [设置页记录](docs/UI_VISUAL_REFRESH_PLAN.md)。

<details>
<summary><strong>展开历史版本开发记录</strong></summary>

以下为各版本当时的开发记录，不覆盖当前功能介绍和最新开发交接。

### v1.3.2 失败原因与损坏 RAR 识别

- RAR 列表必须完整成功后才能信任其中的加密条目标记；截断文件即使暴露 `Encrypted = +`，也会直接报告损坏或不完整。
- 明确的纯密码错误继续进入保险箱和字典流程，不影响正常加密归档。
- 压缩、解压与特殊压缩的失败终态统一保存错误摘要，并保证最后一条日志写明最终失败原因。
- 历史持久化层对遗漏原因提供明确兜底，不再显示只有“失败”却没有原因的记录。
- 详见 [v1.3.2 Release notes](docs/RELEASE_NOTES_1.3.2.md)、[失败原因审计](docs/FAILED_TASK_FINAL_REASON_AUDIT_2026-09-05.md)和 [v1.3.2 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.3.2)。

### v1.3.1 RAR 密码判定修复

- RAR 自动密码候选必须由真实加密条目验证；未加密条目不能再证明候选正确。
- 密码一旦被加密头或加密内容证明，后续数据错误会明确归类为归档损坏、不完整或条目密码不一致，不再回退到“输入密码重试”。
- 手动密码先完成真实内容测试再提示验证成功；保险箱使用次数只在完整解压成功后增加。
- 解压前后锁定源文件大小和修改时间，拒绝 0 字节及仍在下载或复制中的归档。
- 详见 [v1.3.1 Release notes](docs/RELEASE_NOTES_1.3.1.md)、[本机失败记录审计](docs/RAR_PASSWORD_POST_VALIDATION_FIX_AUDIT_2026-09-05.md)和 [v1.3.1 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.3.1)。

### v1.3.0 配置、密码与工作区收口

- 压缩与解压批次配置切换统一放在底部；压缩设置始终展示分卷能力或不可用原因，密码框可按名称选择保险箱条目。
- 密码保险箱默认显示正文，解压任务显示实际使用密码；密码生成器改为固定标题/底栏和内部滚动的紧凑弹窗。
- 双栏文件浏览器支持粘贴路径跳转；文件完整性页改为紧凑双栏工具区；特殊压缩操作尺寸统一。
- “添加压缩包”只显示受支持格式，并保留常见首卷入口；56 个测试文件、314 项测试与生产构建通过。
- 详见 [v1.3.0 Release notes](docs/RELEASE_NOTES_1.3.0.md) 和 [v1.3.0 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.3.0)。

### v1.2.9 发布后总审计与维护收口

- 对 v1.2.8 的队列顺序、并发与同目标串行、全局/单独配置、暂停/恢复/停止及清理进行完整复验，没有发现新的产品缺陷。
- 真实 Windows Tauri 门禁重新执行暂停控制与 archive-flow；前者验证单项/全部控制和停止清理，后者验证并发、真实字节遥测、同目标串行、加密 7Z 往返及密码格式回退。
- 生产依赖通过 npm 官方安全源审计，结果为 0 已知漏洞；前端 325 项、Chromium 13 项、集成 6 项、性能 17 项、Rust Release 全量、壳扩展及严格 Clippy 均通过。
- 详见 [v1.2.9 Release notes](docs/RELEASE_NOTES_1.2.9.md)、[发布审计](docs/RELEASE_AUDIT_1.2.9.md)和 [v1.2.9 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.9)。

### v1.2.8 归档队列、配置来源与真实暂停

- 解压列表与真实执行队列共用中文数字自然排序；不同输出资源按设置并行，同一最终目录继续安全串行，避免事务提交和同名冲突竞态。
- 压缩和解压任务可明确选择“跟随全局”或“单独配置”；设置中心可持久保存默认压缩格式、等级和解压同名目录策略。
- 行内、工具栏和左下任务监控提供单项/全部暂停与恢复。原生归档读写、密码保险箱/词表、ZIP/7Z 密码验证、完整性校验和发布前检查都在安全点等待。
- Windows 外部 7-Zip、WinRAR/RAR 和分卷进程使用系统级挂起与恢复；停止会唤醒暂停任务、清理暂存并让队列接续。
- 真实 Windows 子进程、Tauri/WebView2 控件和完整 archive-flow 已验证；特殊压缩不会显示不具备后端协议的伪暂停。
- 详见 [v1.2.8 Release notes](docs/RELEASE_NOTES_1.2.8.md)、[归档控制审计](docs/ARCHIVE_QUEUE_CONFIGURATION_CONTROL_AUDIT_2026-09-04.md)和[发布审计](docs/RELEASE_AUDIT_1.2.8.md)。[v1.2.8 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.8) 提供 Windows 安装包和签名自动更新资产。

### v1.2.7 任务布局、完整提示与固定双栏

- 解压任务名称列获得更合理的可用宽度；省略名称可悬停查看全文，状态、阶段、进度与操作使用稳定网格对齐，终态内容保持居中。
- 多文件按中文和数字自然顺序排列；存储预检恢复位置、介质、剩余空间、预计占用、文件系统和预留空间，760×520 下仍完整可见；侧栏低高度模式继续展示版本徽标。
- 所有真实发生省略的单行文字统一在悬停或键盘聚焦时显示完整内容，已有业务标题不会被覆盖，动态内容再次悬停时会刷新。
- 解压中心、压缩中心和特殊压缩的标题与上下留白向文件浏览器的紧凑密度对齐，把更多垂直空间留给任务区。
- 双栏文件浏览器在 920、760、390 px 下始终使用左右两栏；窄窗口只收缩低优先级列、工具文字和间距，不再切换成上下布局。
- 详见 [v1.2.7 Release notes](docs/RELEASE_NOTES_1.2.7.md)、[解压任务布局审计](docs/DECOMPRESSION_TASK_LAYOUT_AUDIT_2026-09-03.md)、[工作区与双栏审计](docs/WORKSPACE_DENSITY_AND_DUAL_PANE_AUDIT_2026-09-03.md)和[发布审计](docs/RELEASE_AUDIT_1.2.7.md)。[v1.2.7 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.7) 提供 Windows 安装包和自动更新资产。

### v1.2.6 解压运行态、卷根事务与原生窗口交互

- 低高度侧栏、左下任务摘要、折叠任务行和展开详情统一为紧凑信息层级；长路径、状态和日志保持单行省略，详情两栏在 920×620 与 760×520 下无横向溢出。
- 速度只在稳定非终态样本后发布，完成、失败或取消时清空速度与 ETA；百分比使用显示层缓动，任务存储和后端真实进度不被改写。
- Windows 卷根目录的事务暂存固定落在目标卷，避免根目录无父级时回退工作盘并触发跨卷 `os error 17`；隔离逻辑卷根通过真实产品 IPC、字节校验和暂存清理验证。
- 标题栏拖动和八方向缩放统一交给 Windows `WM_NCLBUTTONDOWN` 原生循环，避免前端手工坐标、DPI 和松键状态漂移；真实系统输入门禁验证窗口移动 `48×28`、缩放 `40×30`，释放后尺寸保持。
- 内置密码字典继续逐项真实验证，但持久日志只保留边界和周期节点，不记录密码明文，也不再用高频日志挤压界面。
- 详见 [v1.2.6 Release notes](docs/RELEASE_NOTES_1.2.6.md)、[解压运行态审计](docs/DECOMPRESSION_RUNTIME_UX_AUDIT_2026-09-03.md)与[发布审计](docs/RELEASE_AUDIT_1.2.6.md)。[v1.2.6 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.6) 提供 Windows 安装包和自动更新资产。

### v1.2.5 窗口拖动、缩放与侧栏修复

- 补齐 Tauri 标题栏原生拖动的编译能力和运行权限，空白标题栏可用于移动窗口，三个窗口控制按钮保持独立。
- 重写窗口缩放会话的结束逻辑，修复松开左键后鼠标再次移入会继续缩放的问题。
- 高 DPI 环境下将物理指针位移换算为逻辑窗口尺寸，缩放幅度不再随系统缩放比例偏移。
- 920×520 最小窗口下完整显示 8 个侧栏入口，不再出现多余的导航滚动条。
- 详见 [v1.2.5 Release notes](docs/RELEASE_NOTES_1.2.5.md) 与 [实现审计](docs/WINDOW_INTERACTION_AND_SIDEBAR_AUDIT_2026-09-02.md)。[v1.2.5 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.5) 提供 Windows 安装包和自动更新资产。

### v1.2.4 特殊压缩信息层级与窗口交互

- 图片、视频、PDF 的设置弹窗限制在可读尺寸内；PDF 输出目录移入批量设置，主工作区不再被配置挤压。
- 压缩中心全局设置改为两列紧凑布局；配置组管理限制尺寸并使用固定标题、独立主题化滚动区，浅色和深色主题均保持文字可见。
- 视频任务默认折叠，只保留尺寸、时长、质量和预计大小；输入事实、流信息及执行前变化在用户展开后显示。
- 空白标题栏可拖动窗口，四边和四角均可缩放；顶端不再重复显示产品名。
- 左侧“本地处理 · 隐私优先”已移除，原位置用于固定任务监控，任务出现不会改变导航布局。
- 详见 [v1.2.4 Release notes](docs/RELEASE_NOTES_1.2.4.md) 与 [实现/测试审计](docs/SPECIAL_COMPRESSION_SHELL_AUDIT_1.2.4.md)。[v1.2.4 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.4) 提供 Windows 安装包、签名自动更新资产和完整发布说明。

### v1.2.3 特殊压缩运行时与交互收尾

- 图片设置使用主题化、键盘可操作的下拉菜单；视频质量与输出分辨率分开控制，默认不主动降分辨率。
- 视频单项配置保留真实探测事实并合并连续设置变更；媒体外部进程统一隐藏窗口启动，不再反复弹控制台。
- 图片 Flex 卡片和视频/PDF Grid 行按内容高度排布，中间任务区超出后纵向滚动；三类任务在左侧全局任务栏显示准确类型。
- 删除窗口壳层残留的 1 px 透明缓冲和边框；实际 Release 软件已逐屏检查窗口边缘、三个特殊压缩 Tab、设置弹窗和 PDF 空态。
- 详见 [v1.2.3 Release notes](docs/RELEASE_NOTES_1.2.3.md)、[实现/测试审计](docs/SPECIAL_COMPRESSION_RUNTIME_UX_AUDIT_1.2.3.md)与[发布审计](docs/RELEASE_AUDIT_1.2.3.md)。
- [v1.2.3 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.3) 提供 Windows 安装包、签名自动更新资产和完整发布说明。

### v1.2.2 特殊压缩与文件浏览器收尾

- 图片、视频设置使用固定于视口的 Modal，打开前后主工作区尺寸和页面滚动高度保持；三类工作区移除已由 Tab 表达的重复标题和说明。
- 特殊压缩 Tab、主卡片和操作按钮对齐解压/压缩中心的即时反馈与入场动效，并保留减少动态效果设置。
- 双栏右键菜单为文件夹、文件和压缩包统一提供“在 Windows 文件管理器中打开”；目录精确打开，文件精确选中，不再错误回落桌面。
- 系统保护、junction 等特殊目录无法读取属性时展示友好提示；之后读取普通项目会清除旧错误状态。
- 详见 [v1.2.2 Release notes](docs/RELEASE_NOTES_1.2.2.md)、[发布审计](docs/RELEASE_AUDIT_1.2.2.md)与[实现审计](docs/SPECIAL_COMPRESSION_FILE_MANAGER_POLISH_AUDIT.md)。
- [v1.2.2 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.2) 提供 Windows 安装包、签名自动更新资产和完整发布说明。

### v1.2.1 更新后菜单稳定性修复

- 公开 `v1.2.0` 更新首次实测中，应用、版本、安装位置和重启均成功，但旧 updater 尾部清理在新进程首次注册之后删除了目录类菜单；测试明确失败，没有记为全绿。
- 新进程现在于启动后 2 秒和 6 秒再次复核已有菜单，覆盖 updater 的尾部清理窗口；普通启动只在用户原本启用菜单时修复，不会擅自开启功能。
- 公开更新测试在首次完整后额外观察 8 秒，再退出应用并复核菜单，避免瞬时完整冒充稳定完成。
- 真实公开 `v1.2.0 → v1.2.1` 应用内更新 25/25、失败 0；当前机器最终安装公开 `1.2.1`，菜单、用户数据、安装位置与自启动保持，相关应用进程为 0。
- 详见 [v1.2.1 Release notes](docs/RELEASE_NOTES_1.2.1.md) 与 [发布审计](docs/RELEASE_AUDIT_1.2.1.md)。
- [v1.2.1 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.2.1) 提供 Windows 安装包、签名自动更新资产和完整发布说明。

### v1.2.0 特殊压缩与双栏便利功能

- “特殊压缩”左侧导航显示 `Ctrl+Shift+S`；图片、视频、PDF 三个模式 Tab 与标题同行，使用原空白区域。
- 图片和视频批量设置默认折叠；三种空工作区均使用主卡片内部布局，未添加文件时不产生页面纵向滚动。
- 双栏各自提供进入/退出多选按钮；`Ctrl+A` 全选、`Esc` 退出多选、`F5` 刷新活动栏。
- 文件路径按盘符和目录逐级可点；目录和空白区右键可让另一栏打开相同位置，空白区还能交换两栏。
- 左栏跨栏操作明确向右，右栏明确向左；原有不覆盖、重解析点阻断、暂存校验和跨卷先校验后删源边界保持不变。
- [Release notes](docs/RELEASE_NOTES_1.2.0.md)、[发布审计](docs/RELEASE_AUDIT_1.2.0.md)与[实现审计](docs/UX_CONVENIENCE_1_2_0_AUDIT.md)记录预期/实际差异和验证证据。

### v1.1.19 双栏文件浏览器

- 页面打开后直接展示左右两个真实文件栏，不再以拖入压缩包作为默认浏览方式。
- 跨栏复制使用目标暂存与逐文件 BLAKE3 校验；移动在跨卷时先校验目标再删除源，所有操作默认拒绝同名覆盖。
- 右键支持复制/移动/压缩/解压到另一栏、重命名、移到系统回收站和属性；压缩与解压复用原任务、历史和安全事务。
- 双击压缩包或点击“打开压缩包”进入原有只读安全工作区，归档预览、嵌套浏览和选择性解压能力保持不变。
- 公开四资产已经回下载核验；真实 `v1.1.18 → v1.1.19` 应用内更新 24/24，安装目录、两套用户数据、经典菜单、自启动和唯一版本化 Shell DLL 均符合预期。
- [v1.1.19 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.19)、[Release notes](docs/RELEASE_NOTES_1.1.19.md)、[发布审计](docs/RELEASE_AUDIT_1.1.19.md)与[实现审计](docs/DUAL_PANE_FILE_BROWSER_AUDIT.md)记录公开资产、真实预期/实际差异和测试证据。

### v1.1.18 特殊压缩导航

- 左侧新增唯一“特殊压缩”入口，内部提供图片、视频和 PDF 三种模式；不把三种能力拆成三个零散主入口。
- 压缩中心移除媒体模式切换，只保留归档文件压缩、智能分析、配置组和归档任务队列。
- 图片、视频、PDF 的执行引擎、任务/历史 schema、取消、容量预检、暂存验证与原子发布保持不变。
- 单元 270/270、媒体架构契约、生产构建及三类隔离 Windows Release 真实桌面门禁通过；精确候选六组正式安装生命周期 64/64，公开四资产回下载与 `v1.1.17 → v1.1.18` 应用内更新 24/24，详见 [v1.1.18 发布审计](docs/RELEASE_AUDIT_1.1.18.md)。
- [v1.1.18 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.18) 提供 Windows 安装包、签名自动更新资产和完整发布说明。

### v1.1.17 PDF 安全优化

- 提供“无损整理”和“兼容图片优化”两种本地 PDF 处理模式；执行前展示真实页数、大小、加密、签名、表单和附件事实，并明确兼容模式可能有损且不保证缩小。
- qpdf 12.4.0 以固定五文件运行时随包分发，启动和执行时核对大小、SHA-256、版本、加密后端及 JSON/图片优化能力；密码通过标准输入传递，不进入参数、日志或历史。
- 签名 PDF 默认只分析；加密 PDF 即使密码正确也不会在无法安全保持加密时输出降级副本。源文件永不覆盖，结果经暂存、独立结构复核、容量/竞态/取消检查后原子发布。
- PDF 任务复用统一队列、取消和历史；完整矩阵覆盖文本、扫描、图文混合、中文字体、透明图片、表单、附件、300 页和 6000×4000 图片，并验证批量失败隔离、默认阅读器和完整重启。
- 精确候选正式 NSIS 已通过无测试桥生产 IPC 生命周期 54/54、PDF 工作区 23/23；公开四资产、回下载校验和 `v1.1.16 → v1.1.17` 应用内更新 24/24 均已通过。
- [v1.1.17 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.17) 提供 Windows 安装包、签名自动更新资产和完整发布说明。

### v1.1.16 视频压缩

- 支持 MP4/H.264、MOV、AVI、WMV、WebM 等常见输入，公开输出固定为 MP4/H.264/AAC；H.265 是已验证输入，不是输出格式承诺。
- 提供清晰、均衡、小体积三档与最大分辨率设置，执行前展示后端探测事实、估算区间以及字幕、多音轨等流变化风险。
- 编码使用安全参数数组、机器进度、Windows Job Object 进程树取消、隔离暂存、完整 ffprobe/帧验证和原子发布；源文件默认保持不变。
- 视频任务复用统一任务、日志、取消和 SQLite 历史；真实取消、默认应用播放、完整应用重启和正式 NSIS 生命周期已有审计证据。
- 本版本仅承诺带 Media Foundation 的非 N Windows x64；Windows N 尚无实机证据，暂不支持。硬件编码不在首版范围内。
- 公开发布资产、签名清单、安装包内容与 `v1.1.15 → v1.1.16` 应用内更新均已复核；安装态覆盖、卸载、恢复和开机自启动状态保持门禁最终 49/49 通过。详见 [发布审计](docs/RELEASE_AUDIT_1.1.16.md)。
- [v1.1.16 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.16) 提供 Windows 安装包、签名自动更新资产和完整发布说明。

### v1.1.15 图片压缩

- 图片工作区支持 JPEG、PNG、WebP 的保持/转换格式、质量、缩放、元数据和冲突策略，并展示真实输入/输出预览与指标。
- 真实生产编码发布前重新解码，核对格式、尺寸、方向、Alpha 和元数据；安全事务覆盖容量预检、目标竞争、磁盘满、取消与清理。
- Windows Release 实测 100 张混合批量 100/100，96 MP 可处理、100.01 MP 按边界拒绝；正式安装态图片闭环 17/17。
- 公开 `v1.1.14 → v1.1.15` 应用内更新 24/24，通过签名校验、自动重启、用户数据、自动启动、经典菜单和唯一 Shell Extension 验证。
- [v1.1.15 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.15) 提供 Windows 安装包、签名自动更新资产和完整发布说明。

### v1.1.14 压缩包浏览工作区 2.0

- 像文件管理器一样浏览压缩包目录，支持历史导航、键盘操作、搜索筛选、精确多选和选择性解压。
- 文件与目录右键操作统一到对象化菜单；默认应用打开使用隔离缓存，可执行内容必须二次确认。
- ZIP/TAR 图片与文本预览受严格内存、扫描、类型和尺寸预算限制；7Z/RAR 无法证明有界时明确禁用内部预览。
- 支持最多三层只读嵌套归档、逐层密码隔离、循环与深度保护，并可恢复外层浏览状态。
- 正式安装态验证覆盖 ZIP、加密 7Z/RAR、TAR/TAR.GZ、中文八层路径、混合内容、损坏包、危险脚本、150% 缩放和 18 万条目取消。
- [v1.1.14 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.14) 提供 Windows 安装包、自动更新资产和完整发布说明。

### v1.1.13 解压主流程可信度修复

- 密码候选阶段固定显示 0，真实解压开始后才按实际产出字节更新进度。
- 修复正确 RAR 密码被固定短超时误判；独立桌面门禁同时验证错误密码无输出、正确密码解压和内容 SHA-256。
- `.long-extract-*` 事务目录立即隐藏并在清理后通知 Explorer；冲突弹窗的逐项决策直接提交现有暂存，不重新读取归档。
- Win32 容量预检覆盖不存在的目标子目录；新历史保存真实格式，长字典日志保留阶段与结果但不记录密码明文。
- [v1.1.13 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.13) 提供 Windows 安装包、自动更新资产和完整发布说明。

### v1.1.12 显式开机自启动收口

- 设置中心恢复 Windows 开机自动启动开关，状态加载严格只读，只有用户点击才会注册或删除启动项。
- 启用命令固定为带引号的当前程序绝对路径与 `--autostart`，重复启用不改写；登录启动隐藏主窗口并驻留托盘。
- 更新保留用户选择，卸载清理当前和历史品牌值；注册失败时界面回读真实状态，不保存虚假成功。
- 真实 Windows Release Tauri 门禁点击实际设置开关，验证注册表精确值、幂等启用、关闭清理及隐藏启动；托管 CI 构建测试二进制并校验脚本，GUI 闭环保留在交互式 Windows 环境执行。
- [v1.1.12 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.12) 提供 Windows 安装包、自动更新资产和完整发布说明。

### v1.1.11 修复收口

- 修复大型未加密分卷被误判为加密包的问题：加密检测只读取结构化目录元数据，探测超时会明确失败，不再启动密码保险箱或内置字典。
- 分卷解压进入真实执行后持续显示两位小数进度、当前处理文件、已产出内容、预计展开大小和展开速度；完成后使用实际输出目录大小校正统计。
- 密码保险箱、导入词表和内置字典会明确显示当前阶段、候选来源、序号、结果与耗时，日志不记录密码明文。
- 新增“解压后移入系统回收站”，只在完整解压事务成功后处理整组源分卷；失败、取消和部分解压均保留源文件。
- 压缩详情新增真实已读取量、当前压缩包大小、实时压缩比与空间变化；分卷压缩按全部卷文件汇总。
- 使用用户提供的五卷真实 ZIP 样本验证：未进入密码流程，并在受控执行窗口内持续产生解压进度和当前文件事件。
- [v1.1.11 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.11) 提供 Windows 安装包、自动更新资产和完整发布说明。

### v1.1.10 修复收口

- 分卷归档导入支持用户一次拖入任意分卷：标准 ZIP 分卷、数字分卷和旧式 `.z01/.zip` 会归一到可执行的起始卷，重复选择的同组分卷只创建一个任务。
- 缺少首卷、卷号断档或混入不同分卷组时会在进入队列前给出明确诊断，不再只显示笼统的“不支持格式”。
- 解压和压缩详情在 760px 最小窗口到常见桌面尺寸下保持左右结构，长路径限制展示高度，日志拥有独立纵向滚动区且不会横向撑开页面。
- 解压左栏的资源预检卡禁止被 Flex 压缩；压缩中心将同类预检信息统一放到左侧配置区，目标位置、介质、剩余可用与预计占用保持可读。
- 新增真实尺寸响应式门禁，覆盖 760×520、920×620、1024×720 与 1440×900 的长路径、长日志、资源预检和滚动边界。
- [v1.1.10 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.10) 提供 Windows 安装包、自动更新资产和完整发布说明。

### v1.1.9 修复收口

- 暂停未签名 Windows 构建中的开机自启动入口，移除主程序的 `Run` 注册表读写命令。
- 旧配置中的 `autoStart` 会迁移为关闭，设置加载不会再触发持久化写入。
- 安装器与自动更新器一次性清理 `Long解压`、`LongDecompress` 和旧品牌启动值；用户数据库、密码保险箱和任务历史保持不变。
- 解压服务改为读取密码保险箱实际使用的文件库，真实加密归档命中后会原子更新调用次数、最近使用时间和当天趋势。
- 历史任务详情改为不透明表面，状态与完成时间保持单行；侧栏新增原生版本角标。
- 密码统计改为解压密码调用与归档线索语义；压缩和解压共享的目标存储预检在窄窗口中保持最多两列且不横向溢出。
- [v1.1.9 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.9) 提供 Windows 安装包、自动更新资产和完整发布说明。

### v1.1.7 更新

- 新增“历史任务”中心：压缩与解压的完成、失败、取消结果会持久保留，支持趋势、筛选、详情、脱敏日志和独立清理；真实 ZIP 往返在应用重启后仍可恢复。
- 密码保险箱数据全景升级为 35 天活跃热力、更新年龄、风险洞察、常用与最近活跃排行，统计只使用本机真实数据。
- 普通 ZIP、AES ZIP 以及 TAR、TAR.GZ、TAR.BZ2、TAR.XZ、TAR.ZST 接入真实输入字节、速度和 ETA，不再按文件数伪造吞吐。
- TAR 系列补齐大文件读取中取消、长路径和元数据回归；五种格式分别通过 64 MiB Windows Release 可见门禁、独立 7-Zip 校验、应用解压和 SHA-256 回环。
- 修正短压缩任务来不及形成速率基线，以及历史任务 schema v6 后旧迁移测试仍期待 v5 的发布阻断。
- [v1.1.7 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.7) 提供 Windows 安装包、自动更新资产和完整发布说明。

### v1.1.6 更新

- 解压密码候选统一由后端状态机管理，未加密归档不会触发密码破解或记录虚假密码成功。
- 批量压缩与解压接通有界并发，同一输出目录始终串行；真实字节回调用于速度和 ETA 展示。
- 原生 7Z 支持真实固实压缩及 AES 密码往返；分卷创建声明与当前实际能力保持一致。
- 非原生密码格式转 7Z 时显示最终扩展名并二次确认，拒绝后不创建任务或文件。
- Windows Release 桌面及安装态回归覆盖上述主流程、经典右键菜单、覆盖安装、卸载和数据恢复。
- [v1.1.6 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.6) 提供 Windows 安装包、自动更新资产和完整发布说明。

### v1.1.5 更新

- 解压中心与压缩中心的展开详情在所有窗口宽度下固定保持左右分栏，内部内容按空间换行并只做纵向滚动。
- 详情面板统一使用强调色细滚动条，隐藏 Windows 原生箭头，修复压缩配置区滚动条风格不一致的问题。
- 项目主页采用完整的真实运行截图，集中展示解压、压缩、归档浏览、密码保险箱和生命周期分析。
- [v1.1.5 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.5) 提供 Windows 安装包、自动更新资产和完整发布说明。

### v1.1.4 更新

- 重构压缩包浏览中心的信息层级：目录与文件成为主工作区，压缩包、密码、统计和输出目录改为紧凑布局。
- 左侧完整路径平铺改为可折叠文件夹树，同时保留当前目录、搜索、类型筛选和精确选择性解压。
- 支持直接拖入压缩包浏览；资源管理器经典菜单与 Windows 11 原生 Long解压 子菜单均接入“浏览压缩包内容”，并直接进入浏览中心。
- [v1.1.4 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.4)提供 Windows 安装包、自动更新资产和完整发布说明。

### v1.1.3 更新

- 统一 ZIP 浏览与解压使用的 Unicode 路径语义，修复中文深层路径在精确选择解压时被过滤为空的问题；非空选择零输出会明确失败。
- Windows Release 桌面门禁覆盖中文长路径 ZIP、加密 7Z/RAR、智能压缩、Mark-of-the-Web、压缩后校验和校验成功后的源文件删除保护。
- 完整桌面回归继续验证诊断、修复、图片预览、资源预检、任务模板、监控目录、托盘隐藏和应用重启恢复。
- 间接依赖 `nanoid` 更新到 3.3.18，生产依赖安全审计为 0 个已知漏洞。
- [v1.1.3 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.3)记录安装包、更新资产和完整发布说明。

### v1.1.0 更新

- 修复经典资源管理器菜单中 Long解压 父项没有二级菜单、点击后触发文件关联错误的问题。
- 使用空 `SubCommands` 与内联 `shell` 子项恢复四类级联菜单；Long解压、一键解压和一键打包统一设置为顶部同组。
- Vite 8、Vitest 4、Vue 插件 6 与 vue-tsc 3 完成迁移，完整 npm 审计和生产依赖审计均为 0。
- 覆盖率、浏览器 E2E、Rust、Shell Extension、桌面构建和 Windows 安装包继续作为发布门禁。
- [v1.1.0 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.1.0)记录本轮测试与发布资产。

### v1.0.23 更新

- 在“显示更多选项”的经典菜单外层增加“一键解压到同名文件夹”和“一键打包为 ZIP”，常用操作不再需要展开 Long解压 二级菜单。
- 经典菜单使用空 `SubCommands` 标记和内联 `shell` 子项注册四类菜单根与 17 条命令，避免 Long解压 父项失去二级菜单后被错误点击；注册状态、覆盖安装和公开更新验收同时核对独立快捷项及同组顶部排序。
- 资源管理器格式表与解压中心统一：过滤 DOCX、XLSX、PPTX、ODT、ODS、EPUB 和未完成公开验证的格式，补齐 QCOW、VDI、VMDK、APFS、EXT、HFSX 等已验证格式。
- v1.0.19 → v1.0.23 → 卸载 → v1.0.19 的本地安装生命周期 41/41 项通过，原安装路径、用户数据和菜单状态均已恢复。
- 无商业代码签名的公开安装包继续只交付经典菜单，不把 Windows 11 第一层菜单写成已支持能力。
- [v1.0.23 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.0.23)记录本轮测试与发布资产。

### v1.0.22 更新

- 公开更新自动化现在严格区分旧进程退出与新版独立重启，并等待启动阶段完成右键菜单结构迁移。
- 数据保持检查会精确停止当前安装路径的应用后再读取 SQLite 文件，避免文件占用被误判为升级失败。
- 发布验收继续覆盖签名更新清单、安装目录、两套用户数据、唯一版本化 Shell DLL，以及四类菜单根/17 条命令。
- README、开发审计、路线图和 Release 追溯已同步到当前状态。
- 不配置 Windows 商业代码签名证书；公开安装器继续使用已验证的传统菜单，应用内更新仍使用独立的更新包校验签名。
- [v1.0.22 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.0.22) 与[正式验收报告](docs/RELEASE_VALIDATION_1.0.22.md)已发布；真实公开更新 22/22 项通过。

### v1.0.21 更新

- 修复 Windows“显示更多选项”中只有顶层入口、二级命令消失的问题；四类菜单根使用空 `SubCommands` 标记和内联 `shell` 子项注册 17 条按用户命令。
- 状态检测会拒绝旧 `SubCommands` 布局，并逐项核对命令目标；应用覆盖升级后会在启动阶段自动迁移旧菜单。
- 已从官方 v1.0.20 通过软件内可见更新界面升级到公开 v1.0.21，签名交接、独立重启、安装目录、两套用户数据、唯一 Shell DLL 和 4 根/17 命令共 21 项检查全部通过。
- 新增固定机器性能趋势工具，覆盖大文件 ZIP、小文件 ZIP、原生 7Z 和 AES v2 真实往返；同机同配置且双方至少 10 次样本后才应用回归阈值。
- 完成 Vue、Playwright、Vue Test Utils、GSAP、PostCSS 和 Autoprefixer 的同主版本更新，生产依赖审计保持 0 个已知漏洞。
- [v1.0.21 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.0.21) 与[正式验收报告](docs/RELEASE_VALIDATION_1.0.21.md)均已发布，最终结果为 `PASS`。

### v1.0.20 更新

- ZIP、7Z、TAR 系列、单文件流和应用自有 AES 压缩写入已拆分为独立原生模块，归档能力、执行路由与事务发布规则保持统一。
- 7Z 解压补齐 CRC 与密码错误分类、时间戳恢复、文件筛选、取消、磁盘写满和暂存回滚的真实归档回归。
- 压缩失败、磁盘写满或目标竞争时不会覆盖既有输出，并会清理临时文件和未发布半成品。
- 修复已完成的同源任务阻止切换格式再次压缩的问题；终态行会被新任务原位替换，活动任务重复请求会明确提示。
- 桌面 E2E 每次使用独立应用身份、IPC、数据和 WebView2 目录，异常残留不会污染后续验收。
- `1.0.17` → `1.0.20` 候选覆盖安装、卸载、数据保持和基线恢复 42 项通过；ZIP、7Z 与快速解压完成真实安装版抽查。

### v1.0.19 更新

- 修复应用内更新完成后没有自动重启的问题：Tauri 被动更新现在默认重新启动应用，维护型安装可用 `/NR` 明确禁止重启。
- 无签名身份包时不再把遗留 Windows 11 native 菜单误判为有效，启动同步会可靠降级到传统右键菜单。
- 新增公开版本应用内更新自动验收，覆盖真实 UI 检查、签名交接、版本切换、自动重启、安装目录、用户数据、Shell DLL 和右键菜单。
- 正式公开 v1.0.18 → v1.0.19 应用内更新 18 项通过：签名交接、覆盖安装、自动重启、
  原安装目录、用户数据、单一 Shell DLL、传统右键菜单和无 MSIX 残留均已验证。
- 候选覆盖安装、卸载、数据保持和基线恢复 41 项通过；严格全格式桌面矩阵通过。

### v1.0.18 更新

- 修复无代码签名证书时 Windows 11 仍注册不完整原生菜单的问题；公开版会明确降级到“显示更多选项”中的传统菜单。
- 覆盖安装会先注销旧身份包并清理历史 Shell DLL/MSIX，不再在安装目录累计多个扩展版本。
- 新增真实已安装版本验收：从公开 v1.0.17 覆盖到 v1.0.18，验证版本、安装目录、用户数据指纹、右键菜单、卸载清理并恢复基线。
- 版本一致性门禁统一检查前端、Tauri、主程序、Shell Extension、Cargo lockfile、版本化 DLL 和 Git 标签。
- 全格式严格桌面矩阵覆盖 25 种可创建格式、虚拟磁盘、文件系统、安装包、固件及固定上游只读样本；缺少生成器时不再静默跳过。

### v1.0.17 更新

- 压缩中心任务行固定为压缩包名称、源文件路径、压缩状态与进度三部分，展开后继续使用左侧配置、右侧阶段/进度/实时日志的结构。
- 密码保险箱保存真实密码强度，编辑密码时保留创建时间、使用次数、最后使用时间和完整使用记录。
- 新增保险箱数据全景与单条密码生命周期分析，覆盖安全指数、强度分布、风险项目、分类分布和常用密码排行。
- 使用趋势支持 7 天、30 天、90 天和全部历史；长期画像提供使用时长、最早创建、历史月均使用及历史峰值月份。
- 统计图表加入折线绘制、渐变面积、节点弹出、环图轨道、分类流光和卡片错峰入场动效，并兼容“减少动态效果”。
- 前端 145 项测试、类型检查、生产构建与实际页面视觉验收通过；Rust release 全套测试和真实归档矩阵通过，生产依赖审计为 0 个已知漏洞。

### v1.0.16 更新

- 压缩条目在等待、执行、完成、失败和取消阶段保持原位展示，不再拆分为配置列表与独立任务列表。
- 展开条目后固定为左侧配置、右侧阶段/进度/速度/当前文件/实时日志，交互与解压中心保持一致。
- 配置提交后与任务 ID 稳定绑定并冻结，阻止重复提交；取消竞态不会再把任务错误改回“已完成”。
- 磁吸成组入口移动到待分组文件旁，顶部全局设置和开始压缩按钮不再因选择文件发生位移。
- 单文件、分组和批量任务统一支持取消、批量取消、逐项清理及一键清理已完成任务。
- 压缩中心对齐解压中心的紧凑任务行、抽屉展开动效与详情卡层级，并优化详细设置的两列密度。
- 密码保险箱扩大密码正文列，收窄名称和备注列，同时固定“使用频率”表头单行显示。
- 前端 141 项单元与组件测试、类型检查、生产构建以及 Chromium/移动端 Chrome 10 项交互测试通过。

### v1.0.15 更新

- 对规范化输出路径实施前后端双重占用，阻止重复启动同一压缩目标。
- 压缩和解压任务在列表、统计、启动、取消与清理流程中完全隔离。
- 压缩中心提供与解压中心一致的可展开任务详情、实时进度、日志、取消和已完成清理。
- 7Z 使用真实字节进度和可取消读写；加密判断直接读取元数据，不再执行无反馈的完整归档预检。
- LZMA 改用内置流式编码器，修复随包 7-Zip 不支持 `-tlzma` 导致的压缩失败。
- 真实 Windows Tauri E2E 已验证常用格式、加密 ZIP/7Z、任务取消及压缩解压逐字节一致性。

### v1.0.14 更新

- 新增 `AESENC02/TARAES02` 分块加密格式，大文件加密和解密保持有界内存，并保留旧格式受限只读兼容。
- 加密流程补齐错误密码、数据篡改、截断、任务取消和磁盘写满清理，失败时不会留下已提交输出。
- 真实 Windows Tauri E2E 覆盖第二实例任务转发、ZIP 压缩解压逐字节闭环、长任务取消和残留清理。
- 桌面生命周期门禁覆盖活动任务退出确认、更新安装阻断以及托盘隐藏和二次启动恢复。
- 前端覆盖率达到 75.93% 行、72.68% 分支和 56.07% 函数，关键 Tauri 桥接层具备直接回归。
- 发布治理新增 `master` 分支保护、必需 CI、版本化验收清单和 GitHub Issue 记录入口。

### v1.0.13 更新

- 归档流式复制统一采用可复用的 256 KiB 缓冲，覆盖 ZIP、7Z、GZ、BZ2、XZ、Zstandard 和密码 ZIP 等主要路径。
- 原生 ZIP 解压进度事件按 4 MiB 节流，缓冲池保留扩容容量，减少 IPC、内存分配和小块 I/O。
- 通用引擎减少递归资源扫描；暂存布局整理与资源计数合并，降低大量小文件的目录遍历成本。
- 文件筛选规则在任务开始时预编译，不再为每个归档条目重复创建正则表达式。
- 大文件压缩、解压及密码 ZIP/7Z 流程补齐块级取消响应。
- 新增 100 MiB、1 GiB 和 10,000 个 4 KiB 文件的真实基准，解压结果使用 CRC32 校验并监控峰值工作集。

### v1.0.12 更新

- 放大品牌图标的有效内容区域并减少透明留白，同步更新应用、安装器和各平台图标资源；移除侧栏品牌图标的额外描边与阴影容器。
- 修复部分主题强制覆盖用户强调色的问题，增强强调色选择状态和设置开关开启/关闭状态的视觉差异。
- 密码保险箱首次加载显示稳定骨架屏，不再短暂显示空数据或零统计。
- 密码名称、密码正文和备注三列统一宽度，每条密码新增独立显示/隐藏按钮，并保留复制快捷操作。
- 创建表单改用“创建新密码”“密码名称”“密码正文”等直观文案；移除手动锁定、主密码输入和闲置自动锁定入口，同时继续使用安装实例密钥保护本机数据。

### v1.0.11 更新

- 托盘悬停提示统一显示“Long解压”，右键菜单新增打开主窗口、解压中心、压缩中心、显示/隐藏和退出快捷入口。
- 解压中心收紧归档过滤：DOCX、XLSX、PPTX、ODT、ODS、EPUB 等文档容器不再作为压缩包导入，并对选择、拖放和右键任务入口统一校验。
- 简化左下角全局进度区，只保留清晰的“X 个进行中”状态，移除与其重叠的完成数显示。
- 压缩中心新增“使用同名压缩包”选项，适用于单文件、文件组和全局压缩设置。
- 优化批量文件元数据读取和任务统计，采用受控并行读取并减少重复遍历。

### v1.0.10 更新

- 软件名称统一升级为“Long解压”，覆盖窗口标题、托盘、安装器、开始菜单、资源管理器右键菜单和 GitHub Release。
- 使用全新的 L 形拉链品牌图标，直观结合字母 L 与文件解压语义。
- 重新生成 PNG、ICO、ICNS、Windows Appx 全尺寸图标，并同步网页 favicon 与应用侧栏标识。
- 保留 `com.longcompress.assistant`、用户配置目录和 Tauri 更新密钥，维持现有安装及应用内更新兼容性。
- 正式安装包在没有 Windows 代码签名证书时继续跳过 Windows 11 新式菜单身份包，传统右键菜单正常可用。

### v1.0.9 更新

- 完成 Windows 11 新式右键菜单扩展基础设施和原生 `IExplorerCommand` 实现；由于项目未配置 Windows 代码签名证书，v1.0.9 公开安装包暂不携带新式菜单身份包，仍使用“显示更多选项”中的传统菜单。
- 新增“一键解压到同名文件夹”，从资源管理器直接把所选归档解压到同名目录。
- 新增“一键打包为 ZIP”，支持把单个或多个所选文件、文件夹打包到当前目录。
- 已保留新式菜单身份包的安装、卸载和启动修复基础；项目当前不做 Windows 商业代码签名，因此正式版继续使用传统菜单。

### v1.0.8 更新

- 新增签名应用内更新：启动后每天最多检查一次，设置中心也可手动检查。
- 自定义更新交互支持版本说明、稍后提醒、跳过版本、失败重试和中英文界面。
- 压缩或解压任务运行时禁止安装，更新失败不会影响当前版本继续使用。
- Tauri 公钥内置于应用；私钥仅保存在 GitHub Actions Secrets，更新包必须通过签名验证。
- 发布流水线自动生成 NSIS 安装包、`.nsis.zip`、`.sig` 和 `latest.json`。
- 1.0.7 用户需要手动安装一次 1.0.8；从 1.0.8 开始支持后续应用内更新。

### v1.0.7 更新

- 修复所有格式共用的加密状态误判：未加密归档不会再触发字典攻击或显示“密码破解成功”。
- 解压流程加入事务式暂存、原子提交与失败回滚，并统一冲突策略、文件筛选、扁平化及“仅解压较新文件”语义。
- 增加磁盘空间、文件数量、展开体积、压缩比、符号链接和 Windows 重解析点安全限制。
- 标准分卷 ZIP 使用 `.zip.001` 结构；压缩输出、TAR AES 与旧式 AES 路径补齐原子写入和资源限制。
- 外部引擎与密码字典支持可靠取消；RAR 密码创建增加进程参数风险确认，后台命令继续保持无控制台窗口。
- 真实归档矩阵、后端全目标测试、前端单元与端到端测试、类型检查、生产构建及 Clippy 全部通过。

### v1.0.6 更新

- 内置完整 7-Zip 26.02 引擎，并根据运行时能力动态开放格式。
- 新增 WIM 创建，以及 QCOW2、VDI、VMDK、APFS、EXT、GPT、AR、PPKG 等首批扩展格式读取。
- 创建 RAR 缺少编码器时，可选择改用 7Z、通过 winget 安装 WinRAR、打开官方下载页或取消；已安装 WinRAR 时支持密码 RAR 创建。
- 设置中心新增引擎版本、读取/创建格式数量、RAR 编码器状态和重新检测入口。
- 新增真实 7Z、ZIP、TAR、WIM、密码 7Z 与 AR 样本回归矩阵。

</details>
