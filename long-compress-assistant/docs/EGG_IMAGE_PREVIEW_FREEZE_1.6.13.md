# v1.6.13 EGG 图片预览封板记录

## 用户交付与范围

EGG 非固实图片接入原双栏、单图弹层与连续阅读器，继续使用双击/预览按钮、翻页、缩放、坏图跳过、定位和当前页提取。另一栏状态、单条展开、多选弹窗、50/50 双栏、工具中心、快捷键与原主题保持，没有新增浏览中心或常驻配置区域。说明收在原问号内，失败提示隐藏内部分类标记，并保留提取或跳过的下一步。

helper 0.5.0 新增 `preview-image`，与小文本共用精确所选非固实条目的完整解码；图片输出不超过 8 MiB，实际所选压缩载荷读取最多 64 MiB（包括 AES/LEA 认证重读），所有所选块大小、CRC、既有终止条件和条目总大小通过后才返回。主应用复用 PNG/JPEG/GIF/WebP/BMP 与单边 8192、1600 万像素范围。方法解码器未修改，固实条目、超限及不支持的内容给提取指引；内容预算不是整个进程总内存承诺。

手动密码、保险箱、显式代码页与完整原生卷沿旧入口接通，单图和连续阅读共用所选图片的密码尝试，不先完整验证无关大文件，不进入暴力尝试，不把所选预览记为整包密码验证。helper 与包括保险箱候选的整体读取均限时 30 秒；原请求取消登记传到子进程，关闭/换项停止，读取前后核对来源版本。原当前页提取继续走队列和完整校验，仅提取准确的包内路径。

同时补齐 EGG 文本读取后的来源版本核对，修正旧路径只为 ALZ 捕获后置核对版本的问题；EGG 单图、文本和阅读三个命令入口均明确拒绝 ZPAQ 快照条件，不能忽略传入的读取条件。

## 来源与针对性检查

新增图片来自[自有像素与归档生成器](../scripts/prepare-egg-image-1613.py)，只复用既有自有 PNG/BMP 像素函数，不执行旧生成器。Store/Deflate/Bzip2/LZMA 使用 Python 标准压缩器构造 EGG，中文 CP936 ZipCrypto 与 AES-256/认证分别构造，原生三成员在实际加密载荷中间切开并保留双向 ID 链。测试生成器使用本机已有 Python cryptography；它不属于产品运行依赖。样本和哈希见[清单](../tests/fixtures/archives/egg-image-1613/manifest.json)。

新增图片/密码/卷型明确为受控回归，不是新原 ALZip 图片原件；未重新下载、执行安装器或重复失败取材。既有原 EGG 文本、算法与密码/卷型依据沿用，不把受控图片外推为全部历史互通；AZO 图片与 LEA 图片未新增正向原件，不声称取得新的互通证明。

- 四项不同 Rust 检查通过：Store/Deflate/Bzip2/LZMA、中文 ZipCrypto/AES 与原生中间卷的精确像素字节；8 MiB 精确边界、旧小文本；CRC、缺结束、尾随、非图片、尺寸、固实、输入/输出超限和错密；提前取消及读取后来源变化；实际临时保险箱所选图片与使用计数保持。见[日志](evidence/egg-image-preview-1.6.13/rust.log)。1093 项其余测试被筛出，不冒称全量运行。
- 55 项不同前端检查通过：54 项原视图/能力与新 EGG 图片、阅读接线、关闭取消和旧图文路径；只额外运行一项新增阅读错误提示检查。见[汇总](evidence/egg-image-preview-1.6.13/frontend-checks.json)。
- 11 项新图片操作的完整字节与自有像素哈希一致，覆盖两种图片和四种压缩方法、ZipCrypto/AES 与完整原生卷；见[helper 收据](evidence/egg-image-preview-1.6.13/helper.json)和[脚本](../scripts/audit-egg-image-1613.py)。实际超过 64 MiB 的受控输入文件在忽略目录生成，产品按声明范围于解码前拒绝，没有声称完整解码该大载荷。
- 五组实际桌面通过：原双栏单图、原阅读器翻页/缩放/跳过、当前页准确提取及哈希、保险箱与中间卷的单图/阅读双入口、超限/来源变化/两入口提前取消/旧文本/缺卷和三个命令的快照混用拒绝。见[桌面收据](evidence/egg-image-preview-1.6.13/desktop.json)。首次脚本在进入提取场景前漏关原阅读器，导致遮挡导航；只修脚本关闭场景残留弹层，保留前两组并接续剩余项，产品候选不变，见[首次日志](evidence/egg-image-preview-1.6.13/desktop-run.log)与[接续日志](evidence/egg-image-preview-1.6.13/desktop-resume.log)。
- 三图已实看：[1100×650 单图](evidence/egg-image-preview-1.6.13/image-wide.png)、[760×520 单图](evidence/egg-image-preview-1.6.13/image-narrow.png)、[760×520 阅读器](evidence/egg-image-preview-1.6.13/reader-narrow.png)。图片实际解码，关闭、底部信息和主要操作可见；双栏等宽、无页面横向溢出，布局和主题未重构。

桌面候选 SHA-256：`172be1ee62e81b3e4585d5717c00decbb444a898f2f6586c6b47e3d5b8857d81`；helper SHA-256：`4a1b7740bc66863856891ad3bb4e7ecbaa1a3ded6d085e9177f88c612410ba5e`。helper、类型/Vite、Shell、最终原生编译和八处版本身份/引擎清单通过，见[构建记录](evidence/egg-image-preview-1.6.13/checks.json)。本地桌面使用 `custom-protocol,desktop-e2e`、opt-level=1/LTO=false，正式发布配置保持；helper 源码与资源按锁定的原始字节规范保存。

## 发布与下一阶段

已达到可发版程度。README、中文 Release、支持口径、开发交接和下一阶段路线已同步，按受保护 PR / Windows package / 正常合并 / v1.6.13 标签发布；源码、合并、标签与正式运行身份在本版 PR 接续。旧算法矩阵、原件整包回解、安装与真实盘满检查沿用既有依据，不重复铺开。

前版 v1.6.12 [正式运行 39844889171](https://github.com/Longyuyeee/long_Decompress/actions/runs/39844889171)已成功，安装包、更新包/签名和清单四附件齐备，同树 master CI 39844831684 已确认取消。[v1.6.14 EGG 小型固实文本](ARCHIVE_ROADMAP_1.6.14.md)仅规划、未开发；先有原件的未加密小型固实文本，再按实际增量接固实图片及大文本前缀。ALZ 方法 3、原工具图片/历史 Bzip2 大包和 EGG 加密固实继续作为来源条件，不空发取材版本。
