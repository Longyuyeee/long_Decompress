# Long解压测试入口

本文只记录当前仓库中实际可执行的测试入口。测试结论必须同时写明预期、实际结果和差异；模拟 DOM 测试不能替代真实 Windows/Tauri、真实压缩包与真实媒体文件验证。

## 测试分层

| 层级 | 真实位置 | 命令 | 用途 |
| --- | --- | --- | --- |
| 前端单元/组件 | `src/**/__tests__`、`tests/fixtures` | `npm run test:unit` | 组件状态、工具函数、契约与固定夹具 |
| 前端集成 | `tests/integration` | `npm run test:integration` | 跨模块数据流和性能集成 |
| 性能回归 | `tests/performance` | `npm run test:performance` | 当前实现的可重复基准与边界 |
| 浏览器 E2E | `e2e/app.spec.ts` | `npm run test:e2e` | 响应式布局和浏览器可验证交互 |
| Rust 单元/集成 | `src-tauri/src`、`src-tauri/tests` | `cargo test --manifest-path src-tauri/Cargo.toml` | 后端事务、安全边界和真实引擎 |
| Windows 桌面门禁 | `scripts/test-tauri-desktop.mjs` | `npm run test:e2e:desktop:<场景>` | Release Tauri、生产 IPC、真实文件系统和原生窗口 |

Playwright 的唯一配置测试目录是根目录 `e2e/`；不要在 `tests/e2e/` 新增规范文件，因为它们不会被 `playwright.config.ts` 收集。

## 常用审计顺序

```powershell
npm run test:unit
npm run test:integration
npm run test:performance
npm run test:e2e -- --project=chromium
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

涉及桌面原生行为时，再运行对应的 `test:e2e:desktop:*` 场景。发布前还必须执行版本身份、正式安装、升级与资产回下载门禁；具体命令以当期 `docs/RELEASE_AUDIT_*.md` 和 `package.json` 为准。

## 测试数据规则

- 小型、可提交的固定夹具位于 `tests/fixtures`。
- 媒体矩阵由 `npm run test:fixtures:media` 或对应 `*:real` 脚本准备并校验清单。
- 大文件、加密归档、跨卷和低容量场景应在隔离临时目录或专用测试卷中创建，测试后清理。
- 不得把不存在的示例文件、CSS 选择器或脚本写成可执行说明；新增命令前先在 `package.json` 中建立真实入口。

## 结果记录

每轮阶段审计至少记录：

1. 需求或风险的预期结果；
2. 使用的真实输入、环境和命令；
3. 实际结果及可复核证据；
4. 与预期的差异、修正和剩余阻断；
5. 是否允许升版、打包和发布。

当前开发接续点见 [`docs/DEVELOPMENT_HANDOFF.md`](../docs/DEVELOPMENT_HANDOFF.md)，当前解压运行态审计见 [`docs/DECOMPRESSION_RUNTIME_UX_AUDIT_2026-09-03.md`](../docs/DECOMPRESSION_RUNTIME_UX_AUDIT_2026-09-03.md)。
