# v1.6.12 EGG 小文本预览封板记录

## 用户交付与实现范围

在原双栏直接查看 EGG 的非固实说明、日志与配置，继续使用双击、预览按钮、右键和 Ctrl+Enter、原只读弹层及其滚动。另一栏的位置与选择保留，单条展开、多选弹窗、50/50 双栏、工具中心和快捷键保持，没有新增浏览中心或常驻配置区。预览帮助收在原问号内，EGG 图片入口本版不开放。

helper 0.4.0 新增 `preview-text`，必须精确指定一个非固实普通文件，完整输出不超过 1 MiB，所选压缩数据声明范围不超过 64 MiB；实际载荷读取也限制为 64 MiB，包括 AES/LEA 认证重读。认证与解码共享同一计数 reader，失败不返回部分内容。所有所选块的实际输出、CRC、现有流终止条件与条目总大小都通过后，才返回内存内容；不解码无关文件，不落地临时提取。解码方法本身未修改，原完整提取、检查和转换路径共享同样的完整解码函数。

复用原手动密码、保险箱排序、显式代码页与完整原生卷。密码候选仅完整读取所选小文本，不先验证无关大文件，不进入暴力尝试，不把所选预览记为整包密码验证。helper 和包括保险箱候选的整体预览限时 30 秒，取消通过原请求登记传到子进程；来源版本在格式探测前、内容读取前后核对。固实、超限、损坏、缺卷和来源变化保持不同原因；界面隐藏内部分类标记，二进制与编码不完整给清楚指引，不能显示为完整内容。

预览预算约束内容读取与输出，不是整个桌面进程总内存承诺。Store/Deflate/Bzip2/LZMA/AZO 复用既有解码，字典及解码器资源范围保持；不增加格式数量、EGG 图片/固实预览、创建/修复或加密固实支持。

## 来源与针对性检查

没有重新调查或下载原工具。复用已固定的 [egg-153](../tests/fixtures/archives/egg-153/manifest.json)、[egg-154](../tests/fixtures/archives/egg-154/manifest.json) 与 [egg-1511](../tests/fixtures/archives/egg-1511/manifest.json) 原件和独立载荷哈希，只新增预览操作的读取核对。[审计脚本](../scripts/audit-egg-text-1612.py)的八项涵盖 Store/Deflate/Bzip2/LZMA/AZO、ZipCrypto 与原 ALZip 中文 AES/LEA 中间卷，实际方法和密码类别逐条记录在[原件收据](evidence/egg-text-preview-1.6.12/helper-originals.json)。原厂 `m3` 选项实际为 LZMA，不能按选项名冒称 AZO；AZO `.sys` 示例是 `MZAZO sample payload` 可打印测试载荷，文本入口仍依据支持的扩展名，不冒称新增原工具文本来源或全部历史组合。

[受控样本生成器](../scripts/prepare-egg-text-1612.py)仅创建自有文本、无关未知方法、精确 1 MiB/超限、CRC/终止错误、二进制、空文本和奇数 UTF-16 边界，见[身份清单](../tests/fixtures/archives/egg-text-1612/manifest.json)。实际超过 64 MiB 的压缩数据派生文件仅在忽略目录生成，本版验证其在解码前按范围拒绝，不声称已完整读取或校验大载荷。

- 四项不同 Rust 检查通过：原文本与密码/卷组、完整性和输入/输出范围、取消/替换/缺卷、实际临时保险箱与所选无关内容隔离。见[日志](evidence/egg-text-preview-1.6.12/rust.log)。在版本身份统一前执行，生产逻辑未再改变；最终 1.6.12 原生桌面覆盖真实 IPC。1089 项其余测试被筛出，没有宣称全量运行。
- 52 项相关前端检查通过，涵盖新 EGG 文本/图片能力分离、来源版本与请求取消、迟到内容和错误提示。见[汇总](evidence/egg-text-preview-1.6.12/frontend-checks.json)。
- 八项新 helper 预览结果匹配已有独立载荷哈希。初始脚本误把 Deflate 样本条目写成 `hello.txt`，按原件修正为 `text.txt` 后只续跑余项，已过 Store 保留，未复跑旧整包提取矩阵。
- 五组实际桌面完成：原双栏文本/滚动与能力、原 LEA 中文密码/中间卷与错密、固实小窗口提示、CRC/真实输入范围/来源变化/提前取消、AZO 样本内容与实际二进制/新请求恢复。首次脚本把 `.sys` 文件名当作二进制断言，与真实可打印载荷不符；只修正脚本并保留已完成组与失败前的通过断言，候选未改，见[桌面收据](evidence/egg-text-preview-1.6.12/desktop.json)、[首次日志](evidence/egg-text-preview-1.6.12/desktop-run.log)与[接续日志](evidence/egg-text-preview-1.6.12/desktop-resume.log)。
- 三张截图已实看：[1100×650 文本](evidence/egg-text-preview-1.6.12/text-wide.png)、[760×520 文本](evidence/egg-text-preview-1.6.12/text-narrow.png)、[760×520 固实提示](evidence/egg-text-preview-1.6.12/solid-narrow.png)。文本和底部元信息可读，内容独立滚动，关闭可见，双栏等宽、无页面横向溢出，原主题和布局保持。

桌面候选 SHA-256：`9dbfd2479718f18bf8c13edda74613a4374016f40bbdc8748483bfdf596b56cf`；helper SHA-256：`6c0c1c26b0d4eda62a421a1f844675e3a45166013d0a0cf9def09e8c7b2720b3`。helper、类型/Vite、Shell、最终原生编译与八处版本身份/引擎资源清单通过，见[构建记录](evidence/egg-text-preview-1.6.12/checks.json)。本地桌面使用 `custom-protocol,desktop-e2e`、opt-level=1/LTO=false，正式发布配置保持；按锁定的原始字节规范保存 helper 源码、清单和二进制。

## 发布接续与下一版

已达到可发版程度。README、Release、支持口径和开发交接同步，按受保护 PR / Windows package / 正常合并 / v1.6.12 标签发布；源码、合并、标签和正式运行身份在本版 PR 接续。既有旧算法矩阵、独立整包回解、安装及真实盘满依据沿用，不重复铺开。

前版 v1.6.11 [正式运行 39840578768](https://github.com/Longyuyeee/long_Decompress/actions/runs/39840578768)已成功，安装包、更新包/签名和清单齐备；同树 master CI 39840521134 已确认取消。[v1.6.13 EGG 非固实图片](ARCHIVE_ROADMAP_1.6.13.md)仅规划未开发；之后再拆固实有界选读及大文本。ALZ 方法 3、历史 Bzip2 大包/原工具图片和 EGG 加密固实继续作为来源条件，不空发取材版本。
