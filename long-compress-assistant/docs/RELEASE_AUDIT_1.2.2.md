# Long解压 v1.2.2 发布审计

> 审计日期：2026-09-02
> 状态：正式发布关闭

## 目标对齐

本版本只收口已冻结的六项需求：特殊压缩设置不挤压布局、移除重复文案、动效对齐压缩/解压中心、文件与文件夹统一 Explorer 入口、精确定位不回落桌面、特殊目录属性失败友好提示。未把后续标签页、书签或冲突策略扩展混入本版本。

## 真实差异与修正

- 属性拒绝后查看普通文件时曾残留旧提示；现每次查询前清空旧属性和旧提示，合同从首轮失败修正到 10/10。
- Explorer 路径曾先执行 `trim()` 再构造 `PathBuf`；现仅用空白判断拒绝空值，合法扩展路径保留原始输入，Rust 合同 6/6。
- WebView2 的 CSS 像素出现 `651.988→652` 子像素取整；门禁按 `≤0.1 px` 校验外壳，页面可视高度与滚动高度仍精确相等。
- Chromium 首轮在 200 ms 入场动画未结束时采样，出现约 4.95 px 临时差；门禁改为等待真实动画完成后测量，未延长产品动画或放宽最终稳定阈值。

## 发布门禁

| 门禁 | 结果 |
| --- | --- |
| 类型检查 / 前端单元 | 通过；48 文件、280/280 |
| Chromium | 11/11 |
| Rust 主工程 | 全目标通过；主库 383 通过、10 条条件忽略，主程序 1/1；严格 Clippy 通过 |
| Shell 扩展 | 5/5；严格 Clippy 通过 |
| Windows Tauri/WebView2 | 真实右键 UI 精确打开目录、选中文件；真实 junction 转译为友好属性提示；普通属性恢复；设置 Modal 几何通过 |
| 八处版本身份与唯一 Shell DLL | `1.2.2` 一致；唯一 `long_compress_shell_extension_1_2_2.dll`，246,784 B，SHA-256 `59FD5946F65EA7029C6D205C75A239B3B39D45D1F699F4B371C7D3F39AE7F5AD` |
| 无测试桥 NSIS / 安装生命周期 | `Long解压_1.2.2_x64-setup.exe` 已生成；真实 `v1.2.1 → v1.2.2 → 卸载 → v1.2.1` 49/49、失败 0 |
| PR 五项 CI / 标签 / Release / 公开更新 | PR #114 五项 CI 全绿；annotated `v1.2.2`、四项公开资产及真实公开更新 25/25 全部通过 |

本地候选 NSIS 为 19,390,508 B / SHA-256 `4467A5BC7C01B82F49E8DA88E8E0638ADE595217C975730927CBCC03462EA6B0`；主程序为 29,675,520 B / SHA-256 `A9EFF9304CB6AF62B841C1224834A5536D16F8CEE5150928FC1A9376685C027C`，ProductVersion `1.2.2`。安装生命周期原始证据位于被忽略的 `test-results/installed-release-validation/20260902-095128/result.json`；结束后已恢复公开 `v1.2.1`、原菜单模式、用户数据、自启动和安装路径，相关应用进程为 0。

## 正式发布关闭

- PR [#114](https://github.com/Longyuyeee/long_Decompress/pull/114) 五项 CI 全绿后合入 `master@6d9012b908e7b1bf24d0fbb32af669021c22b2f4`；annotated `v1.2.2` 标签精确指向该提交。
- PR CI run [33581191005](https://github.com/Longyuyeee/long_Decompress/actions/runs/33581191005) 与 Release run [33582167082](https://github.com/Longyuyeee/long_Decompress/actions/runs/33582167082) 均成功。
- 公开 `latest.json` 为 965 B / SHA-256 `67E744768CDEFAECE0FA4AB22BCFAE05FA0214B4E2EE548BAD46FB2C28DDBAAB`；NSIS 为 19,352,130 B / SHA-256 `9822423AB0F2D0E0D6AC3924C96599A7E500922E80FE6FDEBEF4DD2718EFDF79`；updater ZIP 为 19,352,288 B / SHA-256 `62F2AB6E9805487EA9E2A748BBCC1EE24F5409DE9AC218031A57853CF6C3E4E4`；签名为 428 B / SHA-256 `B486294515F7742C737868D4BB85290223697A8853FE4204099CAC3570277636`。manifest 版本为 `1.2.2`，下载 URL 和签名与公开资产一致。
- 真实公开更新证据位于被忽略的 `test-results/public-update-validation/20260902-103133/result.json`：`v1.2.1 → v1.2.2` 共 25/25、失败 0；菜单在 updater 清理窗口后及应用退出后保持 4 根/17 命令/4 快捷动作，安装路径、两套用户数据、自启动和唯一 `long_compress_shell_extension_1_2_2.dll` 均符合预期。
- 当前机器最终安装公开 `1.2.2` 于 `E:\Long\Long解压`，相关应用进程为 0。下一次开发必须从最新 `master`/公开 `v1.2.2` 开始，不再从 PR #114、候选分支或本机被忽略的证据目录接续。
