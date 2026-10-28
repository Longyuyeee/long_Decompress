# v1.7.13 · 旧 GNU 稀疏文件部分取回封板记录

日期：2026-10-28。

## 已完成

恢复读取接受旧 GNU `S`，使用 `entry_size` 读取包内存储量，GNU realsize 表示逻辑大小。完整读取头内/扩展块映射，确认范围、顺序、存储总量、扩展标记和填充；数据在外层已校验前缀内完整到达后，才提交条目。GNU 长名称沿既有规则应用一次。有效 PAX size 与旧 GNU 稀疏布局叠加时明确停止，不猜物理/逻辑大小。

新有界布局最多每文件 65,536 段、4,096 扩展块，报告总映射不超过 1,000,000 段；逻辑总量沿用 512 GiB 上限。映射计入来源 revision，即使源大小/时间和逻辑 SHA 相同也不能复用旧映射身份。扫描与复制均流式计算包含零洞的 SHA，逐块检查取消，不分配整份逻辑文件到内存。

Windows 先通过 FSCTL_SET_SPARSE 标记独立新文件，设置逻辑大小，再只向映射位置写入包内数据。失败目标明确报错，原件保留且暂存清理；逻辑进度按块合并，目录时间仍在内容完成后处理。普通文件路径、时间提示和历史沿既有流程，新增 storedBytes 可选，内部稀疏图不发送为用户清单。

原列表用一行小字提示稀疏文件/包内数据，右侧大小仍为完整大小；原折叠结果显示两种大小。没有新页面或常驻配置区，单条展开、多选弹窗、等宽双栏与主题保留。完整 TAR 读取器未修改。

格式依据：[GNU 旧格式](https://www.gnu.org/software/tar/manual/html_node/Old-GNU-Format.html)；Windows 写入依据：[FSCTL_SET_SPARSE](https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-fsctl_set_sparse)、[DeviceIoControl](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-deviceiocontrol)。只增加 windows-sys 对应 IO/Ioctl feature，没有升级依赖版本。

## 原件和检查

`tests/fixtures/archives/tar-sparse-1713/` 是自编 Linux 稀疏源文件的真实 GNU tar 1.35 `--format=gnu --sparse --hole-detection=raw` 生产者原件，官方固定 SHA lzop 压缩并完整回解核对。源文件、GNU 和 Python 读取的逻辑 SHA 一致；manifest 保存物理头/数据/扩展块位置、映射与逻辑清单。不是应用写入器生成，也不冒称第三方用户原包。固定 SHA 原件之后可直接复用。

- 新原生 7 项最终全部通过：完整原件浏览/完整性/健康诊断；坏尾、坏校验和缺结束标记的实际稀疏输出与后续普通文件、所选子集；映射头/扩展/数据边界、异常范围和 PAX size 冲突；空洞扫描/复制主动取消与真实目标写入失败；同源大小/时间/逻辑内容但不同图的 revision 重查；缺中卷与补回；非整块存储数据的填充截断/异常和实际复制。首轮编译的共享取消函数可见性、测试访问内部 offset 已修正，测试定位改用独立 manifest；没有重跑已成功的完整旧矩阵。
- 相关旧原生 1 项通过：链接、非 GNU 的 `S`、未知类型、PAX sparse 元数据仍停止，普通复制取消清理保持。资源 DLL 在首批编译中更新后，这一接续项触发重新编译，未重复新原生 7 项。
- 前端 4 项通过：0 包内字节显示、普通行不增加稀疏标签、折叠结果/逻辑大小/部分历史，以及旧目录时间与历史兼容。
- 独立 Python 核对 4 份真实输出的路径、21 个文件逻辑 SHA、明确文件/目录修改时间及 Windows 稀疏标记和分配量。64 MiB 样本包内数据 512 B、实际分配 64 KiB；1 MiB 全零文件包内及实际分配均为 0。少段/多段文件也保留稀疏分配，值见 verified.json。所选结果只有指定文件/空目录，原件 SHA 未变。
- 后台真实 Vue/CSS 回放原生报告和结果，3 组通过：零数据筛选/选择，折叠结果及两种大小，长名称的宽窄布局与原部分历史。1100×700、760×520 无横向溢出，底部按钮可见，大小详情可滚动查看。首次脚本把 GNU raw 检测样本的包内量误设为 4 KiB，按独立生产者 manifest 的实际 512 B 修正后通过；未改变产品显示。六张最终截图逐张查看。IPC 明确为回放，没有接管桌面。
- v1.7.13 类型/Vite、Shell DLL 和 8 个版本来源/标签身份通过；正式 Windows 安装包与更新清单以对应提交 CI/Release 为准。

证据见 [本版目录](evidence/tar-sparse-1.7.13/)。PAX sparse、链接救援、缺口后扫描、原生密码/块分段及全部权限/所有者恢复未扩展。下一阶段 [v1.7.14](ARCHIVE_ROADMAP_1.7.14.md)仅规划；旧 GNU 完整矩阵、文件/目录时间、默认应用、更新和其他格式结果继续复用。
