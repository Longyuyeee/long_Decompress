# #366 · 减少动画导致窗口偏移

2026-11-06，修复 [#366](https://github.com/Longyuyeee/long_Decompress/issues/366)。本次为界面缺陷修复，未提升版本或发布安装包；当前正式版仍为 v1.7.40，XZ 多流/填充计划保持不变。

## 原因与处理

在真实 Vue 页面、编译后 CSS 和 1920×1040 无头 Chromium 中复现：开启减少动画后，应用向右偏移 595.1875 像素，开关被裁出窗口。

`UpdateProgress.vue` 的 scoped 样式写成 `:global(html.reduce-motion) .update-progress-track > .indeterminate`。Vue 编译后只保留了全局部分，导致用于居中更新指示条的 `margin-left:31%` 落在 `html.reduce-motion` 上；即使没有下载更新也会触发，保存后重启仍受影响。

改用普通祖先选择器 `html.reduce-motion .update-progress-track > .indeterminate`，让 Vue 把 scope 标识加到真正的指示条上。保留减少动画功能、开关位移和原有布局；不以清空用户设置或强制取消偏好规避问题。已保存开启状态的用户使用修复后的构建即可恢复正常。

## 同类检查与防回归

- 排查 `src/` 中所有 `:global(...)` 样式。另一处失败重试/Toast 层级规则把完整选择器放在括号内，没有本次的后续目标丢失问题，无需改动。
- 新增编译后 CSS 检查：实际调用 Vue 样式编译器，再将生成样式作用到文档和更新指示条，确认根元素不偏移、只有指示条居中，关闭后恢复。
- 新增四个无头 Chromium 场景：1920×1040 浅色、1024×720 深色、920×620 黑金主题的开启/关闭、保存重启、开关可点击、恢复默认弹窗及压缩/解压中心切换；另覆盖应用减少动画和系统减少动画下的未知长度更新指示条，以及系统偏好关闭后恢复动画。
- 断言检查根坐标、侧栏坐标、窗口边界、开关完整可见、弹窗边界和指示条真实位置，避免只查元素存在而漏掉屏幕外控件。

## 检查结果

23 项相关单元测试通过：辅助显示 3 项、更新状态/弹窗/字节回归 19 项、编译样式 1 项。四个 Chromium 场景通过（52.7 秒）；四张截图实看通过。`npm run build`（Vue/TypeScript 与生产构建）通过。

修复前专项已明确失败，根坐标为 595.1875；修复后四个场景根坐标均为 0。首次测试脚本修正了图标参与按钮名称、现有导航路径和主题名称的定位问题后通过，没有扩大产品改动。

界面测试仅替代原生 IPC，不下载/安装更新，不操作桌面或修改 Windows 设置。本次未改动原生逻辑、引擎或系统集成，没有重复运行无关格式与算法矩阵。

见[检查记录](evidence/reduced-motion-366/verified.json)、[浅色](evidence/reduced-motion-366/settings-wide-light.png)、[深色](evidence/reduced-motion-366/settings-compact-dark.png)、[小窗口黑金](evidence/reduced-motion-366/settings-small-obsidian.png)、[更新弹窗](evidence/reduced-motion-366/update-dialog.png)。
