# PDF 空态与无原生边框修复审计

> 后续 `v1.2.3` 进一步删除了布局自身的 1 px 透明缓冲、圆角外框与焦点边框；本文件关于“仅关闭系统装饰”的阶段性结论由 [v1.2.3 特殊压缩运行时与交互审计](SPECIAL_COMPRESSION_RUNTIME_UX_AUDIT_1.2.3.md) 补充。

日期：2026-09-02  
基线：公开 `v1.2.2`，`master@a2553c4`  
开发分支：`codex/pdf-frameless-shell-polish`

## 本轮需求与结论

| 需求 | 修复前实际 | 修复后实际 | 状态 |
| --- | --- | --- | --- |
| 删除 PDF 工作区重复安全说明 | 空态仍显示“默认输出为新文件，禁止覆盖源文件；文件是否变小取决于原始结构，不保证压缩率。” | 文案节点与专用样式均删除，单元和真实 WebView2 均断言不存在 | 完成 |
| 去掉软件最外层 Windows 白色原生框 | `src-tauri/tauri.conf.json` 的主窗口为 `decorations: true`，主布局没有应用内标题栏 | 恢复 `decorations: false`，重新挂载已有 `WindowTitleBar`，保留拖动、最小化、最大化和关闭入口 | 完成 |
| 防止同类回归 | 发布门禁没有约束窗口装饰，主布局测试虽 mock 标题栏但不验证其存在 | 发布身份门禁固定要求 `decorations: false`；主布局测试固定要求标题栏存在 | 完成 |

## 历史根因

- 这不是 `v1.2.2` 当天新引入的回归。`git show ac28143 -- src-tauri/tauri.conf.json src/components/layouts/MainLayout.vue` 显示，2026-07-15 的 `ac28143` 在一个名为“修复解压语法/Clippy”的提交中，同时把 `decorations` 从 `false` 改为 `true`，并删除了 `MainLayout` 中的 `WindowTitleBar`。
- `WindowTitleBar.vue` 以及它的原生窗口动作测试一直保留，`MainLayout.test.ts` 也一直保留标题栏 mock；因此当前状态是一次未被门禁发现的不完整回退，而不是完整的新产品方向。
- 本轮只恢复经过历史代码证明的成对设计：禁用原生装饰 + 应用内标题栏。没有恢复旧提交中八个只有鼠标样式、没有实际 resize 调用的伪缩放热区。

## 真实验收与预期—实际差异

新增长期可复跑命令：

```powershell
$env:EDGE_DRIVER_PATH='<与 WebView2 主版本一致的 msedgedriver.exe>'
npm.cmd run test:e2e:desktop:shell-polish
```

该门禁重新编译并启动真实 Windows Tauri Release/WebView2 二进制，证据写入被忽略的 `test-results/desktop-e2e/shell-polish-result.json` 与 `shell-polish-pdf.png`。

| 指标 | 预期 | 实际 | 差异 |
| --- | ---: | ---: | ---: |
| Tauri 原生装饰 | `false` | `false` | 0 |
| 应用内标题栏控制按钮 | 3 | 3 | 0 |
| PDF 重复说明存在 | `false` | `false` | 0 |
| PDF 空态纵向溢出 | 0 px | 0 px | 0 |
| PDF 空态横向溢出 | 0 px | 0 px | 0 |

其他门禁：

- 类型检查：通过。
- 定向单元：36/36；完整前端单元：48 文件、280/280。
- 生产前端构建：通过。
- 发布身份：`v1.2.2` 一致，唯一 Shell DLL 正确，新无边框约束通过。
- 浏览器矩阵：核心改动相关用例通过；Firefox 首轮仅在 context teardown 超时，单独复跑 1/1 通过。
- Mobile Chrome 首轮稳定发现双栏文件浏览器窄屏规则把横向滚动重新打开；已将 `overflow:auto` 修正为 `overflow-y:auto; overflow-x:hidden`，同一真实布局用例复跑 1/1 通过。

## 已知独立问题与接续点

- 现有 `test:e2e:desktop:responsive-layout` 在 920×620 的真实 Tauri 解压详情中记录到 2 px 横向溢出：详情外框 577 px、client 575 px。它发生在进入本轮 PDF/标题栏页面之前，且不由本轮横向尺寸改动产生；本轮没有扩大范围修改解压详情。
- 后续若继续开发，先为该 2 px 差异建立独立问题与修复前失败证据，再检查详情边框/box sizing；不要放宽 `overflow <= 1 px` 门禁来掩盖。
- 本轮保持版本 `1.2.2`，未打包或发布新 Release。若决定以 `v1.2.3` 发布，应从本分支合并后的精确提交重新跑完整 Rust/Shell、NSIS、安装生命周期、公开更新和资产回下载流程，不能把本地桌面测试二进制当作发布候选。
