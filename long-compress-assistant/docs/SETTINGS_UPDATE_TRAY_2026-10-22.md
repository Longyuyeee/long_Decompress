# 设置、更新进度与托盘修正

本轮沿用 UI PR #261 和 v1.5.64，接续设置与实际使用问题；没有执行合并、打标签或发布。

## 用户可见变化

- 格式页删除写死的 `37+ / 16 / 3`。解压后缀合并应用能力和本机引擎后缀并去重；创建数量对应可用输出选项，RAR 仅在编码器可用时计入；密码解压数量来自同页的实际格式列表。重新检测同步更新，长明细默认收起，统计口径放在标题问号内。
- 更新安装弹窗显示实际下载大小、总大小及百分比。未返回 Content-Length 时显示不定进度；下载完成后明确进入验证/安装，最后显示重启阶段。出错停止，重试清空上次进度；安装期间不能隐藏进度弹窗。
- 新增极地冰川、雨林秘境、星云紫境、曜石鎏金。分别使用冰蓝瓷白、墨绿苔金、深紫柔粉、炭黑香槟金的背景与面板，包含不同的渐变或细纹；保留已有六个选项，提供色板预览、推荐强调色与独立调整。浅强调色按钮自动使用深色文字。外观同时写入后端配置，更新后只剩后端配置时也能恢复；跟随系统响应系统颜色变化，恢复默认真正恢复外观。
- 标题栏 X 改为请求原生关闭策略。勾选关闭到托盘时隐藏窗口，取消勾选时沿用运行中任务确认与草稿保存退出；系统关闭请求使用相同处理函数。

根因是 Tauri 1.8.3 的 `Window::close` 直接销毁窗口，不触发项目监听的 `CloseRequested`。仅同步设置开关无法修复，故将自绘 X 接入显式原生命令，而非继续调用该接口。托盘打开/退出入口保留。

## 本轮检查

五个相关前端文件共 **38 项不同检查通过**，覆盖分块累计、未知总大小、下载后阶段、失败重试重置、监听初始化/释放、进度条语义、标题栏命令、外观恢复与格式数量/明细一致。最终格式明细收纳只接续对应的 9 项设置检查并通过，不另计为新增用例。类型与 Vite、原生构建通过；未重复格式、盘满或安装矩阵。

独立配置的真实桌面首轮候选 `0808298a2b22ceb488e1f0007358d9db3b5d84c53c1085526401431e0a4d5163` 完成四个新主题、宽窄显示、下载事件通道、未知总大小和安装阶段、错误提示、真实标题栏 X 隐藏及第二实例唤回。测试机检测到 **121 个解压后缀 / 41 个可创建格式 / 24 个密码解压格式**；未安装 RAR 编码器，故未把 RAR 创建计入。

更新显示通过测试桥发送模拟下载块，走真实 Tauri 事件传输并由产品监听器处理；没有启动安装器，不能把该记录当成真实升级下载/安装测试。关闭检查直接点击产品 X，隐藏后 WebView 仍可响应并能唤回。

首轮最后的“取消关闭确认”使用了不存在的 `footer` 元素选择器而超时；此前已完成的下载与托盘检查和截图保留。最终只接续受影响的外观/明细显示及取消关闭确认，不重跑已通过的功能。

桌面入口：`node scripts/test-tauri-desktop.mjs --settings-update-themes-only`；收尾加 `--settings-update-themes-final`。分构建记录与代表图位于 `docs/evidence/settings-update-themes/`。

最终候选 `1ed24d752cb28501598a671f68aace5dba48b02541a9e6ed135614254c716f44` 接续四条宽窄显示，格式明细实际展开可达、统计标签及主题图标清晰；取消托盘后点击真实 X 显示原有任务确认，点击取消正常返回。最终记录通过并完成。

- [首轮功能与 12 条布局](evidence/settings-update-themes/result.json)
- [最终显示及关闭确认](evidence/settings-update-themes/final-result.json)
- [曜石鎏金与主题色板](evidence/settings-update-themes/final-theme-obsidian-1100x700.png)
- [冰川](evidence/settings-update-themes/theme-glacier-1100x700.png) · [雨林](evidence/settings-update-themes/theme-forest-1100x700.png) · [星云](evidence/settings-update-themes/theme-nebula-1100x700.png)
- [实际格式统计](evidence/settings-update-themes/final-format-counts-1100x700.png)
- [更新下载进度](evidence/settings-update-themes/update-downloading-narrow-760x520.png)
