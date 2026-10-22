# v1.6.6 窗口记忆封板记录

## 用户问题与交付范围

[#248](https://github.com/Longyuyeee/long_Decompress/issues/248) 反馈 5K 屏上每次启动都要重新调整窗口。旧版本只有固定 920×620 配置，没有持久化窗口状态。本版自动记忆正常位置/大小和最大化，不新增开关、说明区或页面结构。

窗口状态独立保存于应用数据目录 `window-state.json`（开发/桌面测试沿各自隔离目录）。按显示器名称、逻辑尺寸与工作区偏移恢复；用 Windows `WINDOWPLACEMENT` 的正常矩形保留最大化后的还原尺寸，避免将任务栏工作区坐标交给屏幕坐标 API 造成漂移。移动/缩放合并写入，关闭与退出前同步保存，临时文件替换防止半份记录。最小化保留此前状态，不保存隐藏/最小化作为下次普通启动方式。缺失、无效或未知 schema 使用默认窗口。

原屏不可用则选当前主屏；按实际缩放/工作区收回越界和过大窗口。窗口先在隐藏状态恢复，页面加载后普通启动显示、`--autostart` 保持隐藏；沿用 X 关闭到托盘、第二实例唤醒和任务退出确认。没有修改工作区、单条展开、多选弹窗或左右等宽。

## 本轮检查

- 两项新增 Rust：5K/200% 与其他缩放、负坐标显示器、断屏后低分辨率、越界与最小尺寸；损坏记录、有效记录替换、非法值/schema。只运行 `--lib window_memory`，未重跑其余 1076 项。
- 普通生产类型/Vite、Shell DLL 与八处版本身份通过。桌面候选使用 `VITE_DESKTOP_E2E=1` 和 `custom-protocol,desktop-e2e`；本地 opt-level=1、LTO=false，正式流水线沿原发布配置。
- 真实桌面只覆盖窗口生命周期：正常位置/尺寸重启、最大化和还原、最大化后最小化退出、实际 X 隐藏与第二实例唤醒、最大化的开机隐藏启动、断屏/越界/过大状态恢复、损坏状态回退。最终记录、候选 SHA 和截图在 [证据目录](evidence/window-memory-1.6.6/desktop.json)。
- 初次 WebView2 `setRect` 命令未改变原生宿主，比较只覆盖默认矩形，不能充当用户缩放依据；只补移动/缩放及依赖其尺寸的最大化还原。补查用 DPI 感知的真实 HWND 设置/读取，在本机 200% 缩放下确认 2200×1200 物理尺寸、(120,80) 位置重启不变，并补 760×520 逻辑小窗。早期补查的非 DPI 感知读取曾混用逻辑与物理单位，超时未计通过；修正工具后测量，不放宽产品断言。
- 未重跑归档算法/安装/盘满矩阵，未修改既有用户数据。实机只有当前显示器组合；5K、负坐标与其他 DPI/工作区变换另由纯状态计算测试覆盖，不宣称物理换屏实测。

## 状态与接续

七项真实桌面流程完成，三张实际截图均已查看，现已封板，按正常 PR/版本标签发布；正式安装包、更新包/签名及清单由现有 Release 流水线生成。桌面候选 SHA-256 `f7c1fe3734d7bf4abf0d5395b5eae041a25188fc21bac279da107fd9b8a8eee6`。源码与发布提交身份在 PR 回执中补充，不为流水线状态追加重复打包提交。

README、[中文 Release](RELEASE_NOTES_1.6.6.md)、[本版路线](ARCHIVE_ROADMAP_1.6.6.md) 与开发交接同步。[v1.6.7 候选](ARCHIVE_ROADMAP_1.6.7.md) 回到 ALZ Bzip2 大包真实来源与有界读取，目前未完成，方法 3 / EGG 加密固实限制保持。

原生坐标依据：[WINDOWPLACEMENT](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-windowplacement)、[GetWindowPlacement](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowplacement)、[SetWindowPlacement](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowplacement)。
