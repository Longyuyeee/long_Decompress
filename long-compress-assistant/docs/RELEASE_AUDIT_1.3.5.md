# 1.3.5 发布审计

范围为失败后的可见指引与现有恢复入口衔接，具体代码和测试纠偏见 [开发审计](FAILURE_RECOVERY_2026-09-23.md)。不新增自动修复引擎。

## 验证

- 前端 59 文件、400 项全部通过；类型检查与生产构建通过。
- 新增 9 个分类/证据用例先红后绿，4 个页面场景验证指引、诊断导出、现有草稿入口和零自动后端调用。
- `node scripts/test-tauri-desktop.mjs --history-only` 通过真实 Windows WebView2 / 7-Zip / SQLite 场景：损坏 ZIP 不请求密码，完整 ZIP 同名保留两者；重启查看损坏指引，实际 `.002` 缺卷阻止草稿；换完整来源、创建 pending 新任务、手动启动成功，旧失败记录不变，再重启仍一致。
- 本机可复建证据 `test-results/desktop-e2e/download-failure-user-result.json`；测试二进制带隔离桥，不作为公开安装包。
- Rust 业务代码未修改，不将上一版的 405 项结果冒充本轮重跑；本轮编译了桌面测试程序和版本化 Shell 扩展，版本身份 8 处一致。
- 没有重测真实 ACL 权限、全部媒体格式、原生选择器、默认应用或卸载生命周期；输出冲突真实场景为保留两者路径，结构化提交失败指导为组件覆盖。

## 发布与接续

README、版本身份和发布说明同步到 1.3.5。公开包由现有 GitHub Release 工作流构建，正式附件下载核对及本机覆盖升级事实最终维护在 [v1.3.5 Release](https://github.com/Longyuyeee/long_Decompress/releases/tag/v1.3.5)，未生成资产前不得据此文档宣称发布成功。用户备份不上传仓库。

后续见 [下一版计划](NEXT_VERSION_AFTER_1.3.5.md)。已有 workflow 单测门禁本地提交 `02f3791` 仍未获推送权限，不算云端已启用。
