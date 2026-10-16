# v1.4.16 旧 ZIP 密码编码兼容实施审计

## 当前状态

2026-10-17 从已发布 v1.4.15 的主线 `480d2d0` 接续，分支 `codex/v1.4.16-legacy-zip-passwords`。本版按[十二版本路线](ARCHIVE_ROADMAP_1.4.15_TO_1.5.5.md)补旧工具密码编码，不增加新格式或特殊卷；公开稳定版仍为 v1.4.15，尚未封板、升版或发布 v1.4.16。

阶段一至四分别已推送 `48f68d3`、`a10147b`、`090f9bd`、`12143fc`。阶段五已接浏览/预览、临时打开/嵌套、检查、恢复和转换源的后端参数入口；最终相关 120 项 Rust 用例通过，5 项 ignored 不计。新增 9 个恢复/跨编码重建产物经独立回解，前端参数与配置、真实用户页面还未贯通，不能将后端成功写成整个版本已完成。

## 范围和顺序

1. 取得独立工具生成的 CP932（日文）、CP936（简体）、CP950（繁体）真实密码包，核对原件、完整载荷及错误编码行为。不能用修改头部或本应用自压自解替代。
2. 底层按用户明确选择编码；默认仍为现有 UTF-8 / 本机 Windows 兼容，不扩成盲试全部编码。拒绝不可表示字符及有损映射，保留完整内容校验、错误原因和所选范围。
3. 输入密码编码贯通浏览/预览、所选/整包解压、检查、转换及重试；目标密码编码与输入分开。创建接 ZIP/ZIPX 既有方法、密码及卷型，不扩大原来未支持的 ZIPX 标准卷。
4. 批量配置、配置组、模板、草稿、历史再次执行保留选择；非 ZIP 不携带无效选项，不保存密码明文。默认设置及旧数据兼容保持。
5. 新界面沿用现有下拉与就近问号，单条展开、多选弹窗和等宽详情不改版。真实执行错密码后纠正、跨编码转换、切页/恢复与所选内容，并实看常用/窄窗口。
6. 只补受影响检查与独立回解，完成本版闭环后才统一版本、候选构建及发版；不重跑无关媒体、Windows N 或已完成安装矩阵。

## 阶段一 原始样本和基线

### 实际代码缺口

`services/zip_password.rs` 只有 `utf8` / `windows_local` 两种输出编码；默认读取优先 UTF-8，再试当前机器 ACP，并保留短加密头双匹配的完整内容判别。`DecompressOptions` 没有输入编码参数；模板、历史和前端类型还有两值白名单。不能只加下拉而让后续阶段继续丢失选择，也不能把文件名代码页当密码编码。

### 真实独立原包

- 从 [Info-ZIP 官方项目](https://infozip.sourceforge.net/Zip.html)取得 Zip 3.0 源码，原始 ZIP 的 362 个文件逐一与解包源码比对一致；源码 SHA256、编译器/生成器身份、三个卷和载荷哈希保存在[样本目录](../tests/fixtures/archives/legacy-zip-password-1416/README.md)。
- 使用上游 makefile 与 MSVC 19.44 x64 编译，压缩和加密源码未改；给测试副本嵌入 [Windows 11 进程代码页清单](https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests#activecodepage)。对照 ACP/OEMCP 为 936，两个副本分别为 932、950，参数字节经独立探针核对。未改系统语言、注册表、本应用清单或用户安装。
- 三个原包均为真实 Deflate + ZipCrypto，载荷 359 字节，含中/繁/日文本。Python 3.13 标准库 `zipfile` 独立完整读取、校验 CRC 并逐字节核对；UTF-8、其他可表示编码及错误密码均不能通过，不可表示的字符不替换。复跑 `python scripts/verify-legacy-zip-password-fixtures.py`，不重新生成原卷。
- 文件名特意使用 ASCII，隔离密码编码问题；没有将这些原包夸大为 AES、分卷、文件名编码或所有旧软件的证据。Git `-text` 固定原始载荷与辅助清单，避免跨机器 CRLF 转换。

### 生成工具中的真实问题

最初误用不存在的官方二进制下载路径得到 404，改用官方原始源码。旧 C 源码的 `CR` 宏与新版 SDK 头字段冲突，使用编译参数 `-FIwindows.h` 调整头加载顺序后成功，不修改归档算法。首次命令行 shell 引号调用失败后改用 VS Developer PowerShell。尺寸转换/弃用警告保留，不宣传工具无警告。

最初默认输出含 NTFS 安全描述符；该两份自建原件保留在忽略工具目录，再以 `-X -j` 创建不含机器属性的固定样本，原始文件哈希分别记账。没有为了通过读取而改写密文或降低内容校验。

### 应用基线

新增 Rust 基线用例已通过（1 项）：先用固定已知密码字节完整读取三个原卷，再核对旧默认入口。当前 ACP 936 下，简体原包能读取，而日文/繁体原包不能通过完整读取，真实复现“密码正确但跨系统打不开”的缺口。其余筛选出的 0 项不计测试数量；基线阶段产品编码实现与 UI 未改。

## 阶段二 显式无损读取与后端编码保存

- `ZipPasswordEncoding` 补充 `cp932` / `cp936` / `cp950`。固定代码页调用 Win32 明确编码，禁止替换/最佳近似，并要求编码后解码与原 Unicode 文本相同；CP932 的日元/反斜杠、上划线/波浪号别名不会静默替换。UTF-8 及原 Windows 本机模式的行为、默认值和原错误保留。
- `open_entry_with_encoding` / `open_named_entry_with_encoding` 使用任务明确选择；`None` 仍为旧的 UTF-8/本机 ACP 自动路径，不盲试新编码。明文条目不因密码无法编码而失败，文件不存在的原因优先保留，完整 CRC/内容校验没有删减。
- 不可编码返回带所选编码和原始原因的 `ZipPasswordEncodingError`，经 ZIP I/O 类型保留，不伪装成 `InvalidPassword` 或文件损坏。下一阶段仍需让候选密码循环与最终错误处理识别该原因，不能简单将所有 I/O 错误当密码不匹配。
- 后端模板导出/导入和压缩/转换历史白名单保存五种明确模式，旧快照仍兼容、不保存密码。转换快照中存在但非字符串的编码值不再静默变为 UTF-8；未知值使该配置快照无效，不影响原任务最终原因和日志的保存规则。

### 定向结果

最终选择集 **49 项不同 Rust 用例通过、4 项 ignored**：密码入口 12、原生 ZIP 解压 12、原生 ZIP 创建 11、通用引擎 9、历史配置 4、模板 1。基线与早先重复运行包含在这 49 项中，不重复相加；ignored 的一个性能入口及三个外部环境用例不计成功。编译仅有已有 sevenz-rust 的 SeedRead/生命周期警告。

新显式入口对三个固定 Info-ZIP 原卷完整读取并逐字节一致；显式 UTF-8/错误密码不能回退放行，不可表示及别名字符拒绝，原卷的密文损坏和明文 CRC 损坏仍失败。默认旧入口的本机边界与原短加密头碰撞回归保持。Python 独立回解另核对全部原卷 SHA256，未改原卷或预期。

本阶段未改 Vue/TypeScript 或提供新下拉选项，没有用代码文本替代页面验收，也没有执行新页面成功流程或安装器。当前编译身份仍为 1.4.15，仅代表开发分支定向用例，不是 v1.4.16 正式产品。

## 阶段三 创建后的校验与发布

`CompressionService::compress` 在消耗创建参数前保留目标密码编码，校验单包、ZIP 数字卷和标准卷时明确传入相同编码。`verify_native_with_zip_encoding` 与 `verify_numeric_zip_with_encoding` 使用共享显式读取入口，完整读完条目以校验 CRC/AES 认证；旧检查入口仍保持 UTF-8/本机兼容。转换输出也使用同一创建服务，但转换输入的编码仍待贯通。

没有关闭 `verify_after`、强制删源前校验或转换的必需校验。校验失败不发布暂存产物、不清理源文件；编码错误保留类型及原始原因，取消仍为取消，不包装成校验失败。非 ZIP 路由和原来的输出事务没有改变。

### 本轮结果

- **34 项不同 Rust 用例通过，无 ignored**。包含新增的两个服务用例及相关密码、原生创建、完整校验和输出保护回归；与阶段二重叠的用例不累计相加。初次编译 53.99 秒，最终选择集执行 3.39 秒，只有已有 sevenz-rust 警告。
- 新创建用例覆盖 **66 个有效组合**：CP932/936/950 × AES-256/ZipCrypto × ZIP Store/Deflate/Zstandard 或 ZIPX XZ × 单包/数字卷/标准卷，仍排除 ZIPX 标准卷。6 个 ZIPX 标准卷请求继续明确拒绝，不制造成功；140,000 字节跨卷载荷、中文目录、空文件及空目录均核对。
- 每个有效组合分别拒绝显式 UTF-8 的错编码和错误密码，在暂存校验后才发布，再完整核对内容；另有三种代码页各自的单包/数字卷密文损坏、不可表示字符及取消检查，源文件和已有目标保留，只清理本任务暂存输出。
- 开启独立验证环境后，66 个有效产物全部由 **7-Zip 26.04 x64** 实际回解并核对内容/空条目，不依赖本应用读取器或应用自己的回写结果。未改原始 Info-ZIP 样本、系统语言、正式 7-Zip 或本应用清单；本轮没有 UI 修改或桌面用户入口验收，不能替代后续页面流程。

### 独立工具接续

本机忽略目录 `test-results/zip-encoding-1416/independent-7zip/{cp932,cp936,cp950}` 各有 `7z.exe` 和原始 `7z.dll`。副本来自仓库 `src-tauri/resources/archive-engine/`，只给 CP932/950 的 EXE 嵌入样本目录内 `generator/ja-JP.manifest` / `zh-TW.manifest`，与阶段一同为进程代码页设置；CP936 使用本机原始 ACP936。7-Zip 的 [ZIP 读取源码](https://github.com/ip7z/7zip/blob/main/CPP/7zip/Archive/Zip/ZipHandler.cpp) 使用进程 ACP 转换密码，`-mcp` 文件名参数不能替代这一设置。三个副本都先以已知密码通过原始 Info-ZIP 包的 `t` 检查。

本机工具 SHA256：原始/CP936 EXE `95fff19b73eff77c87c0d7bd32822e43fb90dd2932459f962eb532d8dfb6ba53`；CP932 EXE `19cef0d94c0dd2ae15aa35cd2773b2ab3a3501dc43b6686fd82f114724bee151`；CP950 EXE `e54b53efbe49f05f1c3ee5dffdbf89fc8fa616ae15c287bbd29704e3f7498c62`；三个 DLL 均为 `899CD73D7FC60E1C1C35C7DE0F5BED3DC69BFE2CECBD48014D868A347970A0B1`。

换机器复用仓库工具及清单创建测试副本，不修改上游解码源码、不覆盖正式工具。可用 Windows SDK `mt.exe -manifest <上述清单> "-outputresource:<测试副本>/7z.exe;#1"` 嵌入；执行前分别用原始包确认实际代码页，不能仅根据目录名判断。工具和测试临时产物无需加入安装包。

从应用目录运行 PowerShell，设置仅影响该次测试：

```powershell
$env:LONG_ZIP_1416_INTEROP_TOOLS = Join-Path (Get-Location) 'test-results/zip-encoding-1416/independent-7zip'
try {
    cargo test --manifest-path src-tauri/Cargo.toml --release --lib -- fixed_zip_encoding compression_verification::tests zip_password::tests native_compression::zip::tests verification_failure split_archive_verification_failure encrypted_zip_split_creation_publication_and_extraction_roundtrip
    if ($LASTEXITCODE -ne 0) { throw 'ZIP verification selection failed' }
} finally {
    Remove-Item Env:LONG_ZIP_1416_INTEROP_TOOLS
}
```

未设置环境变量时只执行应用侧验证，不可声称独立回解通过。测试密码来自公开自有样本，临时目录由测试创建和回收，不操作用户历史任务或安装数据。

## 阶段四 任务输入与候选密码分类

`DecompressOptions.zip_password_encoding` 是任务内的可选输入编码，与创建输出的同名强类型字段独立。省略或 `null` 保持旧自动 UTF-8/本机模式；明确选择只使用这一种编码，未知值拒绝。没有增加全局状态或修改默认设置。

整包检测原来绕过 `DecompressOptions`、仅所选内容走任务选项，已统一为 ZIP 的实际任务范围。密码预检和实际逐文件解压都使用该选择，后续不回退默认读取。原有所选/过滤、完整内容检查、取消及暂存事务保持；密码包开启并行设置时仍按原限制回退顺序路径，未放宽并行适用范围。

保险箱、导入词表及内置字典均区分匹配、未匹配、字符无法表示三个结果。只有带类型的不可表示/有损映射可以跳过并继续；编码转换系统错误、源文件 I/O、归档损坏和取消继续向上返回，不因外层都是 ZIP I/O 就吞掉。手动输入无法编码时返回 `[archive-inspection:password-encoding]`、代码页及原始原因，不显示成密码错误或确认损坏。候选日志只说明跳过，不能据此判断包的完整性。

### 本轮场景与结果

- 三份 Info-ZIP 原始包和仅切片的数字卷，分别走整包与所选 `payload.txt` 的检测、错误编码、错误密码、不可表示字符及完整解压。共 12 条正确输入路径还原原载荷，`skip_corrupted=true` 不会吞掉编码错误或创建误导性结果，原卷逐字节保持。
- 隔离保险箱优先放不可编码候选，再放正确密码；导入词表按不可编码、错误、正确、未使用的后续候选排序，三种代码页均能继续找到正确密码。独立损坏副本、中止和源文件缺失仍失败，没有改原始样本来通过。
- 混合 AES 包中不同条目分别使用日文/繁体代码页。各任务只读自己的选择，不能把该密码当作解锁整包；只选明文条目时，即使多传不可编码密码也不读取其他加密条目。
- 阶段三的 **66 个有效创建组合**进一步走原生输入解压，完整核对载荷、空文件/空目录，再用相同独立 7-Zip 副本回解。没有扩大 ZIPX 标准卷或其他未支持组合。
- 最终选择集 **70 项不同 Rust 用例通过，5 项 ignored**；ignored 为一个性能入口、三个外部 RAR 环境入口及一个旧样本导出入口，不计成功。前端现有错误判定新增编码标记回归，**24 项通过**，编码失败即使原始详情含密码关键词也不弹成密码错误重试。与阶段二/三重叠的用例不相加。

最初新增四项输入用例有两项通过、两项因测试直接从异步线程调用阻塞解压而触发嵌套运行时失败。生产本就在 `spawn_blocking` 中运行；测试按相同路径修正，原断言、真实 I/O 和完整内容核对保留。随后五项新增输入/错误类型用例通过，最后并入上述定向选择集。清理一个本轮未使用导入后仅剩原有 sevenz-rust 警告，未修改产品实现来避开线程异常。

当前没有改 Vue 页面、前端配置类型或公开稳定版身份；后端方法测试不等于真实桌面入口通过，前端仍不能选择这些新输入模式。

复跑阶段四时沿用阶段三独立工具环境，将 Cargo 筛选参数换为：

```text
fixed_zip zip_input_encoding_serialization zip_password::tests native_extraction::zip::tests universal_engine::tests zip_selection::tests native_compression::zip::tests compression_verification::tests verification_failure split_archive_verification_failure encrypted_zip_split_creation_publication_and_extraction_roundtrip automatic_password_candidates selected_zip_service_preflight
```

前端只运行 `node_modules/.bin/vitest.cmd run src/utils/__tests__/passwordErrors.test.ts`；原始包继续用 `scripts/verify-legacy-zip-password-fixtures.py` 核对，不重新生成。

## 阶段五 预览检查恢复与转换源参数

命令层新增可省略的 `zipPasswordEncoding`，传至预览、密码验证、诊断、完整性测试、临时打开/嵌套及单条/批量 ZIP 恢复。转换单独接 `sourceZipPasswordEncoding`；原包输入不会继承 `options.zip_password_encoding` 的新包设置。旧请求仍走 `None` 自动兼容，旧服务入口也保留。ZIP 目录浏览继续只读元数据，不因密码未匹配阻止查看文件列表，不扩大为整包密码检测。

文字和图片预览保留原有大小、像素及前缀边界，只读取所选条目；命令返回完整错误链，不只显示外层“无法读取”。直接完整性检查的旧 ZIP 密码路径原本会落入不支持密码的外部命令入口，现改为原生完整 CRC/AES 校验，非 ZIP 原有限制不变。

恢复读取器在中央目录、本地头恢复及缺卷视图中使用明确编码。编码失败单列 `password_encoding`，不称为密码错误、损坏或可修复。明确选择编码的加密恢复副本保留该编码和原密码，并完整校验；省略输入编码时仍沿用旧的 UTF-8 恢复输出约定。只恢复完整且校验通过的文件，不改原件、不把公开文件作为加密密码成功的证据。

### 本轮结果

- 原有相关 43 项先通过；新增及相邻选择集 50 项通过，最终合并受影响回归集为 **120 项不同 Rust 用例通过、5 项 ignored**，执行 4.49 秒。重叠运行不累计；只有已有 sevenz-rust 警告。ignored 范围与阶段四一致。
- 三份 Info-ZIP 原包及数字切片均完成明确编码文字预览、完整检查、错误编码/不可表示拒绝；AES 图片与大文字前缀保持所选范围和边界。原包 SHA256、载荷及 Python 独立验证结果不变。
- 从真实原包的私有副本只移除中央目录，保留上游本地头、描述符和密文，再按三种代码页诊断及恢复；另用三组 AES 标准 ZIP 暂时移开中卷，只恢复后续完整加密文件，缺失部分不进入结果，补回原卷后逐字节一致。这是故障恢复检查，不是伪造外部编码样本。
- 三组跨编码重建分别读取日文→生成简体、读取简体→生成繁体、读取繁体→生成日文；源密码、目标密码和编码分别核对，原包保留。该用例验证实际读取/创建/校验/发布和命令参数构造，尚不代表完整桌面转换入口通过。
- 上述 3 个目录恢复、3 个缺卷恢复和 3 个跨编码新包，均由阶段三同一组独立 7-Zip 测试副本完整回解。既有 66 个创建组合也在最终选择集中复核，未把重复输出相加作为新覆盖。

复跑阶段四命令时追加 `archive_preview::tests archive_diagnostics::tests zip_recovery_reader conversion_request_tests legacy_zip_cross_encoding`，保留 `LONG_ZIP_1416_INTEROP_TOOLS` 环境设置即可。前端尚未传新字段，不能宣称用户现已可选；本阶段没有 Vue 页面修改或新的桌面成功证据。

## 接续点

1. 接前端统一编码类型和命令包装，配置组转换器、批量、草稿、历史恢复及输入/目标分开保存。现有前端仍是两值，不能跳过它们宣称配置链完成；旧 `test_archive_password` 和无选项引擎接口仍保持默认兼容，不作为新配置入口。
2. 以现有 ThemedSelect/问号接界面、`password-encoding` 失败标签和操作指引。恢复产物再浏览/解压时继承明确编码；嵌套归档子工作区单独选择，不强制继承父包密码编码。
3. 按真实顺序验证错密码纠正、所选/整包解压、跨编码转换、检查/恢复及结果再用、切页/再次执行和常用/窄窗口页面。复用本版后端和独立工具证据，不用方法测试替代实际 UI。
4. 完成本版范围后再封板升版发布；公开 README、安装器身份和 Release 仍维持 v1.4.15。
