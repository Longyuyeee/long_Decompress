# 文档导航与维护规则

`docs/` 同时保存当前接续文档和历史发布证据。历史审计不是当前需求清单，但它们用于追溯真实测试、版本决策和公开资产，因此不得仅因日期较早而删除。

## 当前接续入口

- [`DEVELOPMENT_HANDOFF.md`](DEVELOPMENT_HANDOFF.md)：跨电脑开发接续总入口，最新记录在文件顶部。
- [`RELEASE_NOTES_1.2.7.md`](RELEASE_NOTES_1.2.7.md) / [`RELEASE_AUDIT_1.2.7.md`](RELEASE_AUDIT_1.2.7.md)：当前 v1.2.7 候选范围与发布门禁状态。
- [`WORKSPACE_DENSITY_AND_DUAL_PANE_AUDIT_2026-09-03.md`](WORKSPACE_DENSITY_AND_DUAL_PANE_AUDIT_2026-09-03.md)：当前省略文本完整提示、三个核心工作区密度和窄窗口双栏固定布局审计。
- [`DECOMPRESSION_TASK_LAYOUT_AUDIT_2026-09-03.md`](DECOMPRESSION_TASK_LAYOUT_AUDIT_2026-09-03.md)：v1.2.6 后续任务名称、状态对齐、自然排序、存储预检和低高度版本徽标修复审计。
- [`DECOMPRESSION_RUNTIME_UX_AUDIT_2026-09-03.md`](DECOMPRESSION_RUNTIME_UX_AUDIT_2026-09-03.md)：当前解压运行态界面、速度、进度、跨盘事务和密码流程审计。
- [`../tests/README.md`](../tests/README.md)：当前真实测试目录、命令与证据规则。

## 历史证据

- `RELEASE_NOTES_<版本>.md`：公开版本用户可见变更。
- `RELEASE_AUDIT_<版本>.md`：打包、安装、升级、资产与回下载证据。
- `*_AUDIT*.md`、`*_BASELINE*.md`：已完成阶段的实现与真实验证记录。
- `CURRENT_DEVELOPMENT_*`、`*_HANDOFF*`：过去机器或阶段的交接快照；只用于追溯，当前工作以本页“当前接续入口”为准。

## 清理准则

可以删除：不在任何配置中收集的测试、引用不存在脚本/夹具/界面的示例、被当前权威文档完全替代且没有审计引用的草稿。

必须保留：公开 Release 说明、发布审计、真实测试证据、仍被接续链引用的阶段审计。修改当前行为后，应更新当期审计和 `DEVELOPMENT_HANDOFF.md`，而不是回写历史版本结论。
