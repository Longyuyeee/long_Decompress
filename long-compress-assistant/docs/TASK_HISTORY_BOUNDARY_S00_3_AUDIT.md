# S-00.3 运行队列与历史任务边界审计（2026-08-27）

## 1. 开发目标

运行队列只负责当前会话中的任务状态，历史任务只来自后端 SQLite 持久化。删除 `task.ts` 中没有后端实现、没有调用方、也不会返回数据的 `fetchTasks` 占位接口，避免后续开发者误把当前队列当作历史数据源或重新制造第二套历史状态。

## 2. 实现与数据边界

- 当前任务终止时，`task.ts` 继续通过 `save_task_history` 写入后端；
- 历史页面继续只通过 `history.ts` 调用 `list_task_history`、`delete_task_history` 和 `clear_task_history`；
- 删除 `task.ts` 的空 `fetchTasks` 函数及公开导出；
- 增加边界测试，明确运行队列 store 不暴露历史读取接口，也不会调用 `list_task_history`；
- 不修改历史 schema、500 条保留策略、日志脱敏、界面布局或当前任务清理行为。

## 3. 预期—实际—修正

| 门禁 | 预期 | 首次实际 | 修正 | 最终实际 |
| --- | --- | --- | --- | --- |
| 调用链审计 | 占位接口没有调用方，真实历史只有一个读取源 | `fetchTasks` 仅在定义和 return 中出现；历史页只依赖 `history.ts` | 删除占位接口并增加边界测试 | 全仓不再存在 `fetchTasks`，历史读取源唯一 |
| 静态与单元回归 | 删除接口不影响当前任务、历史页面或构建 | 首次通过，无产品差异 | 无 | 类型检查通过；40/40 文件、235/235 测试通过 |
| 真实写入与重启 | ZIP 压缩和解压各形成一条数据库历史，重启后仍可读取 | 首次通过，无产品差异 | 无 | 两种任务均为已完成，来源、输出、耗时和记录 ID 重启后保持 |
| 真实界面 | 历史页在正常与 760×520 窗口无横向溢出，状态不换行，详情背景不透明 | 首次通过，无产品差异 | 无 | WebView2 实测全部满足 |

## 4. 真实验证证据

- `npm.cmd run type-check`：通过；
- `npm.cmd run test:unit -- --run`：40/40 测试文件、235/235 测试通过；
- `npm.cmd run build`（`VITE_DESKTOP_E2E=1`）：桌面门禁前端构建通过；
- `cargo build --release --features custom-protocol,desktop-e2e`：Windows Release 门禁后端构建通过；
- `npm.cmd run test:e2e:desktop:history`：使用 2 MiB 随机真实文件完成 ZIP 压缩—解压、SQLite 历史持久化、应用完全退出重启、历史页与紧凑窗口视觉检查，全部通过。
- 恢复无 E2E 桥接的 `npm.cmd run build`，并执行 `cargo build --release --features custom-protocol`：正式生产构建通过。

## 5. 需求对齐与下一步

本节点只清理重复数据源风险，没有增加媒体功能，也没有削弱压缩、解压或历史任务能力。S-00 下一步进入 S-00.4：统一 README、交接文档和格式支持等级，审计并整理未参与 Tauri 构建的旧工程。完成 S-00.4 后再执行 S-00 总体验收，决定是否允许进入媒体压缩 B-00。
