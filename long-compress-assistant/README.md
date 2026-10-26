<div align="center">

<img src="../icon.png" width="88" height="88" alt="Long解压图标">

# Long解压 · 开发说明

Vue 3 · TypeScript · Pinia · Tauri 1.8 · Rust · SQLite

[软件介绍](../README.md) · [下载安装](https://github.com/Longyuyeee/long_Decompress/releases/latest) · [产品路线](docs/PRODUCT_ENHANCEMENT_ROADMAP.md) · [开发交接](docs/DEVELOPMENT_HANDOFF.md)

</div>

## 开始开发

在 Windows 准备受支持的 Node.js（20.19+、22.12+ 或 24）、npm、稳定版 Rust、MSVC C++ 构建工具及 WebView2。

```powershell
git clone https://github.com/Longyuyeee/long_Decompress.git
cd long_Decompress\long-compress-assistant
npm ci
npm run tauri dev
```

只调整前端可使用 `npm run dev`；本地文件处理、原生对话框与系统集成需要 Tauri。使用演示 IPC 的界面截图不能替代真实 Windows 默认应用确认。

| 命令 | 用途 |
| --- | --- |
| `npm run build` | Vue / TypeScript 检查与前端生产构建 |
| `npx vitest run <相关测试文件>` | 受影响功能的前端检查 |
| `cargo test --manifest-path src-tauri/Cargo.toml <测试名>` | 后端定向检查 |
| `npm run build:shell-extension` | 准备当前版本的 Explorer 扩展 |
| `npm run test:release-identity` | 核对应用、锁文件与右键扩展版本身份 |
| `npm run tauri build -- --bundles nsis` | 构建 Windows 安装器 |

按改动范围运行检查，复用已有结果；不为文档或展示资源改动重跑完整格式矩阵。

## 结构与交互约定

```text
src/
  components/       共享组件、文件列表、任务和结果操作
  views/            解压、压缩、浏览、工具与设置页面
  stores/           配置、任务、草稿、历史与媒体状态
  composables/      复用交互与 Tauri 接口
  utils/            格式能力、路径与辅助逻辑
src-tauri/
  src/              Rust 引擎、数据库、系统集成
  shell-extension/  Windows 原生右键扩展
  resources/        随包引擎、菜单资源和许可
scripts/            构建与功能检查
e2e/                浏览器与桌面场景
docs/               产品路线、格式说明和发布记录
```

- 单条任务行内展开，多选任务使用配置弹窗；编辑某条自然形成该条配置。
- 包内外共用导航与列表，左右等宽。返回时保留位置、选择和另一栏状态。
- 常用操作就近展示，次要操作放菜单，复杂配置放二级界面，主区域留给内容。
- Windows 默认应用由用户在系统设置确认。设置打开行为不会替用户更改关联；一次性提醒记录独立保存，升级和恢复偏好不重置。

## 继续工作前先看

| 文档 | 内容 |
| --- | --- |
| [开发交接](docs/DEVELOPMENT_HANDOFF.md) / [产品路线](docs/PRODUCT_ENHANCEMENT_ROADMAP.md) | 当前状态、下一步及已有验证 |
| [v1.7.0 范围](docs/ARCHIVE_ROADMAP_1.7.0.md) / [实现与检查](docs/WINDOWS_OPEN_AND_BROWSER_UI.md) | Windows 打开、双栏统一与历史菜单 |
| [v1.7.2 创建记录](docs/LZOP_MULTI_CREATE_FREEZE_1.7.2.md) / [下一步](docs/ARCHIVE_ROADMAP_1.7.3.md) | LZOP 多成员创建与转换；后续数字分卷创建仅规划 |
| [格式支持说明](docs/FORMAT_SUPPORT_LEVELS.md) | 创建、读取、密码、分卷和限制 |
| [桌面测试说明](docs/DESKTOP_E2E.md) / [测试指南](tests/README.md) | 本地和隔离场景 |

## 打包与发布

应用、npm 锁文件、Tauri 与右键扩展的版本必须一致。构建会准备前端、原生菜单扩展和可用的菜单身份包。发布说明保存为 `docs/RELEASE_NOTES_<版本>.md`；正式 `v*` 标签由 [Release 工作流](../.github/workflows/release.yml)生成 NSIS 安装器、更新包、签名与 `latest.json`。

发布环境提供 `TAURI_PRIVATE_KEY` 和 `TAURI_KEY_PASSWORD`。用户端升级保留已配置的关联入口；真正卸载只移除本软件拥有的关联注册，不修改 Windows UserChoice。

产品能力与最近五个版本统一在 [项目首页](../README.md#updates)维护，完整历史见 [Releases](https://github.com/Longyuyeee/long_Decompress/releases)。

<a id="third-party"></a>

## 许可证与第三方组件

项目代码使用 [MIT License](../LICENSE)。随包 7-Zip、ZPAQ、WIM、ALZ、EGG、视频与 PDF 引擎分别遵循各自许可。ALZ / EGG 源码、构建和依赖许可位于 `vendor/unalz/`、`vendor/unegg/`、`vendor/libazo/`、`tools/alz-engine/`、`tools/egg-engine/` 及对应资源目录。WIM 编码器原始源码包、许可与构建说明位于 `src-tauri/resources/wim-engine/`，随安装包分发。WinRAR 编码器不随项目分发。
