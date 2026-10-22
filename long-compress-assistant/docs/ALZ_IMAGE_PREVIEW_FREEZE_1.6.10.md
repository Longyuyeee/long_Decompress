# v1.6.10 ALZ 图片预览封板记录

## 用户交付与范围

ALZ Store/Deflate/Bzip2 中的受支持图片接入原双栏、单图弹层和连续阅读器，沿原双击/预览按钮、翻页、缩放、坏图跳过、定位和当前页提取使用。另一栏状态、单条展开、多选弹窗、50/50 布局、工具中心和快捷键保持，没有新增浏览中心或常驻配置区。

helper 0.6.0 新增 `preview-image`，与小文本共用精确条目的完整解码和有界写入，解码算法未改。图片单条输出不超过 8 MiB、压缩输入不超过 64 MiB，完整大小/CRC/流终止通过后才返回；主应用复用 PNG/JPEG/GIF/WebP/BMP 识别与单边 8192、16,000,000 像素范围，实际图片显示失败可跳过或提取。内容预算不是整个桌面进程总内存承诺。

密码尝试前先约束所选图片大小，避免保险箱为一张图完整验证无关大条目。复用手动密码、保险箱、显式名称/密码编码与完整原生卷；预览关闭/换项取消，来源版本在格式探测前及读取后核对，旧结果不覆盖新预览。单图和阅读器均复用现有取消登记；预览 helper 的读取期限为 30 秒。文本仍保持原 1 MiB 完整文件限制。

## 来源与实际检查

[生成器](../scripts/prepare-alz-image-1610.py)从自有像素生成 PNG/BMP，再显式构造 Store/Deflate/Bzip2、UTF-8 密码及跨实际压缩数据的原生卷；清单见[样本身份](../tests/fixtures/archives/alz-image-1610/manifest.json)。Bzip2 从 Python/libbzip2 的真实单块载荷重封，不是历史原工具图片原件。无关方法 3 条目不会被解码，但不代表支持方法 3。

曾从[官方页面](https://altools.co.kr/product/ALZIP)取得同一 ALZip 12.37 安装器，哈希与此前原件记录相同；本地 innoextract 1.9 不支持其 Inno 6.4.2，未运行安装器、未安装或更改关联，亦未取得新的原工具图片原件。停止同一来源调查，准确记录见[来源尝试](evidence/alz-image-preview-1.6.10/source-attempt.json)。本版图片密码/卷型为受控回归；此前原 ALZip Deflate 密码/原生卷、原 Bzip2 小文本等底层依据沿用，不外推所有历史图片组合。

- 三项不同 Rust 检查通过：所选两种图片的十次精确字节/哈希、8 MiB 边界，CRC/缺结束/非图片/超限/尺寸及错误密码，阅读取消和来源变化。见[日志](evidence/alz-image-preview-1.6.10/rust.log)。
- 50 项相关前端检查通过：46 项视图、1 项能力、3 项现有阅读器接线，含 ALZ 图文入口、预览关闭取消和迟到结果，见[汇总](evidence/alz-image-preview-1.6.10/frontend-checks.json)。
- 新图片操作直接调用 helper 的输出/输入上限均拒绝，输入负样复用 v1.6.9 实际承载 64 MiB + 1 字节的受控文件。首次脚本的默认 Windows 解码和所选条目名修正后，只接续未完成项，见[边界记录](evidence/alz-image-preview-1.6.10/helper-boundaries.json)。
- 五组实际桌面完成：单图与原双栏；连续阅读/缩放/跳过；只提取当前页并比对哈希；保险箱与原生中间卷及错密；超限/来源变动/提前取消与文本范围保持。见[桌面收据](evidence/alz-image-preview-1.6.10/desktop.json)。首次目录选择模拟值误传文件列表，导致提取取消；只改脚本并接续提取和剩余路径，产品候选不变，已通过显示与翻页保留。
- 三张实际截图已实看：[1100×650 单图](evidence/alz-image-preview-1.6.10/image-wide.png)、[760×520 单图](evidence/alz-image-preview-1.6.10/image-narrow.png)、[760×520 阅读器](evidence/alz-image-preview-1.6.10/reader-narrow.png)。双栏等宽、无横向页面溢出、图片实际解码、关闭可见；原布局和主题样式未重构。

桌面候选 SHA-256：`1303f4c7ae827e7a31d943a24aab38e3e66d51e973292442a92a72668d04ad9d`；helper SHA-256：`558393d2e45d7fc6f8e966934115de438c20e57f851440fc9b41d8f49e57db7b`。普通类型/Vite、最终桌面前端/原生编译、Shell 与八处版本身份通过；本地桌面为 `custom-protocol,desktop-e2e`、opt-level=1/LTO=false，正式发布配置保持。helper 的精确字节源码与许可清单按提交换行规范锁定，换行整理未改逻辑或二进制。

## 发布接续

已达到可发版程度。README、中文 Release、能力说明、开发交接与下一版路线同步；按受保护 PR / Windows package / 合并 / v1.6.10 标签发布，源码、合并、标签和正式流水线身份在本版 PR 接续。旧算法、独立大包完整回解、安装与真实盘满依据沿用，不重复铺开。前版 v1.6.9 [正式构建](https://github.com/Longyuyeee/long_Decompress/actions/runs/39829431411)成功，安装器、更新包、签名及清单四项齐备。

[v1.6.11 ALZ 大文本前缀](ARCHIVE_ROADMAP_1.6.11.md)仅规划。新增原工具图片、历史 Bzip2 大包、方法 3、EGG 加密固实继续作为来源条件任务，不空发取材版本，也不把部分内容记为完整成功。
