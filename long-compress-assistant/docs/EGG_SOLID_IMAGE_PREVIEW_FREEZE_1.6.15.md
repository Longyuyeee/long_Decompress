# v1.6.15 EGG 小型固实图片封板记录

## 用户交付与范围

未加密 Deflate / LZMA 固实图片接原双栏单图弹层与原阅读器，双击、眼睛、右键、Ctrl+Enter、翻页/缩放/跳过/定位和当前页提取沿原入口使用。单条展开、多选弹窗、50/50 双栏、工具中心和原主题保持；范围说明在问号中，等待时才显示共享内容提示。

helper 0.7.0 将已有固实文本内存收集参数化：文本保持 1 MiB，图片不超过 8 MiB；完整共享压缩载荷、解码总量和普通文件声明总量分别不超过 64 MiB。全部依赖的文件/块大小、CRC、既有终止与实际载荷计数通过后，才返回完整所选字节。无关内容丢弃，不创建临时提取目录；完整解压/检查/转换与解码器成功条件不变。主应用继续 PNG/JPEG/GIF/WebP/BMP、单边 8192 和 1600 万像素条件；预算不是整个进程总内存承诺。

主应用固实图片分支按完整依赖预检，未初始化/锁定保险箱也可读普通固实，加密固实在候选前给提取指引。复用单图/阅读 30 秒、请求取消、读取前后来源核对。固实 EGG 禁止下一张自动预读，保留既有至多 3 项 / 16 MiB / 2400 万像素缓存，复用前检查来源，换页/关闭取消未完成请求；没有全包预取、无界缓存或新的设置入口。

## 来源与增量检查

[生成器](../scripts/prepare-egg-solid-image-1615.py)只复用自有 PNG/BMP 像素函数，未执行旧生成器；共享 Deflate / LZMA 流分成两个载荷段，各有输出区域 CRC，图像后面仍有共享依赖。相同像素为 2 MiB BMP 附加合法填充，并构造精确 8 MiB 与超限、后段 CRC/截断/尾随、完整依赖超限、未知方法、其他文件持有加密头等受控样本。三成员原生卷切在实际共享载荷中，全部属于受控，不冒称原工具固实图像或新原生卷互通。清单见[样本哈希](../tests/fixtures/archives/egg-solid-image-1615/manifest.json)。当前工作区未取得新原工具图片，未重复下载或执行旧取材。

真实超过 64 MiB 压缩输入只在忽略目录生成，按完整依赖范围在解码前拒绝，不跟踪大文件。旧原件算法、非固实图文/密码/卷组与安装/真实盘满依据沿用，不重新铺开矩阵。

- 4 项新增 Rust 通过：Deflate/LZMA 与受控原生中间卷的 PNG / 2 MiB BMP 精确字节；8 MiB 边界及文本 1 MiB 不扩大；共享后段 CRC/结束/尾随/依赖超限/加密/未知方法/非图片/尺寸；共享读取完成时取消与来源替换；锁定保险箱普通固实、加密固实拒绝、中间卷准确提取与缺卷。见[日志](evidence/egg-solid-image-preview-1.6.15/rust.log)。1101 项其余测试筛出，未声称全量。
- 6 项相关前端通过：改动的阅读器 3 项原有操作/进度/停止和 2 项新固实按需/缓存来源/等待/晚到丢弃，以及 1 项原单图弹层共享等待和悬停范围。见[汇总](evidence/egg-solid-image-preview-1.6.15/frontend-checks.json)，未计入筛出项。
- 15 项新 helper 操作通过：两方法的两张图、受控中间卷与精确 8 MiB 的 6 份完整字节核对，另 9 项共享依赖/结束/加密/方法/输入输出边界与文本预算拒绝。见[收据](evidence/egg-solid-image-preview-1.6.15/helper.json)和[脚本](../scripts/audit-egg-solid-image-1615.py)。
- 5 组实际桌面通过：原双栏单图；原阅读器翻页/缩放/坏图跳过；当前页准确提取与 2 MiB 哈希；受控中间卷单图/阅读双正式入口；共享后段 CRC 实际弹层、加密/超限、取消与来源变化。见[桌面收据](evidence/egg-solid-image-preview-1.6.15/desktop.json)。
- 3 张已实看：[1100×650 单图](evidence/egg-solid-image-preview-1.6.15/image-wide.png)、[760×520 单图](evidence/egg-solid-image-preview-1.6.15/image-narrow.png)、[760×520 阅读器](evidence/egg-solid-image-preview-1.6.15/reader-narrow.png)。图片实际显示，关闭、工具栏和底部信息/提取操作可见，双栏等宽，无页面横向溢出。

桌面候选 SHA-256：f23ed8108a6daa597e77bff7fe03eaae565668e0dd144a4f78e47e5d71db9de5；helper SHA-256：62d4c997d42f0e29530c8aca68d25b4e126a8fb0b53371779335a2d84e816fa1。helper、类型/Vite、Shell、最终桌面编译与八处版本身份/引擎清单通过，见[构建记录](evidence/egg-solid-image-preview-1.6.15/checks.json)。本地使用 custom-protocol,desktop-e2e、opt-level=1/LTO=false，正式发布配置保持；本版检查均首次通过，未重跑已过旧矩阵、安装或真实盘满。

## 发布与后续

已达到可发版程度。README、Release、支持口径和版本身份同步，按正常受保护 PR / Windows package / 正常合并 / v1.6.15 标签发布；正式身份在本版 PR 接续。[v1.6.16 非固实大文本前缀](ARCHIVE_ROADMAP_1.6.16.md)已接续开发。EGG 加密固实、创建/修复，ALZ 方法 3、原工具图片/历史 Bzip2 大包各自保留来源条件。

前版 v1.6.14 已由 [PR #275](https://github.com/Longyuyeee/long_Decompress/pull/275) 正常合并并推送标签；封板时[正式运行 39853009550](https://github.com/Longyuyeee/long_Decompress/actions/runs/39853009550)仍在打包，附件未确认生成。同树 master CI 39852938781 已确认取消，正式最新状态在发布回执接续。
