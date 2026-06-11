# 剩余工作清单

> 最后更新: 2026-06-10 | 合并提交: 55d9974

---

## P0 — 阻塞发布

### 1. 安装器配置
`tauri.conf.json` 没有 `bundle` 段，无法生成可分发的 .exe/.dmg/.deb 包。
- 文件: `long-compress-assistant/src-tauri/tauri.conf.json`
- 参考: https://tauri.app/v1/guides/building/

### 2. CI/CD 流水线
没有 `.github/workflows/`，无自动构建、测试、发布流程。
- 建议: 添加 GitHub Actions，在 PR 和 push 到 master 时自动运行 `cargo test` + `vite build`

### 3. 测试编译修复（7 个文件/目标）
预置集成测试因 API 变更无法编译。**注意：关键路径已由 35 个通过的集成测试覆盖。**

| 文件/目标 | 错误数 | 问题 | 建议 |
|-----------|--------|------|------|
| `tests/config_comprehensive_test.rs` | 49 | `into()` 类型、API 全量变更 | 全量重写 |
| `tests/config_integration_test.rs` | 36 | `into()` 类型、API 全量变更 | 全量重写 |
| `tests/config_file_loader_test.rs` | 8 | `crate::` 导入 | 按已修复模式迁移 |
| `tests/config_system_smoke_test.rs` | 3 | `into()` 类型 | 按已修复模式迁移 |
| `tests/password_book_integration_test.rs` | 10 | `crate::` 导入 + API 变更 | 按已修复模式迁移 |
| `tests/memory_usage_test.rs` | 6 | `crate::` 导入 + API mismatch | 按已修复模式迁移 |
| `lib test` (各源文件的 `#[cfg(test)]`) | 48 | 散布在各源文件 | 逐个修复 |

**修复模式**: 参照已修复的 `split_compression_test.rs`/`rar_support_test.rs`/`password_zip_test.rs`：
1. 移除 `#[cfg(test)]` 和 `mod tests {}` 包装
2. 将 `use crate::` 改为 `use long_compress_assistant::`
3. 更新 API 调用以匹配当前方法签名
4. 为 `Result` 类型添加显式类型标注

---

## P1 — 影响用户

### 4. 密码通过 CLI 进程列表暴露
`universal_engine.rs`、`rar_support.rs`、`compression_service.rs` 中 6 处使用 `-p<password>` 传参给 7z/unrar/WinRAR CLI，密码在进程列表中可见。
- **建议**:
  - 7z 可用 `-p{env_var}` 语法通过环境变量传密码
  - unrar 无直接方案，优先使用原生 `unrar` crate（已在第一策略）
  - 或使用命名管道/stdin 传递

### 5. 设置表单输入验证
SettingsView 中的线程数、缓存大小等字段可输入负数/非法值。
- 文件: `views/SettingsView.vue`
- 建议: 在 `updateSettings()` 中添加 min/max 校验

### 6. 密码保险箱导入导出前端接入
后端命令已就绪 (`export_passwords_command`、`import_passwords_command`)，前端 `useTauriCommands` 也已封装 `exportPasswords`/`importPasswords`，但 `PasswordVaultView.vue` 只在导航栏显示了导入导出按钮，实际调用链不完整。
- 文件: `views/PasswordVaultView.vue:73-76`

---

## P2 — 改进体验

### 7. GlassButton 组件设计系统落地
`GlassButton.vue` 提供了 variant/size/loading 完备属性，但生产视图中均使用内联 Tailwind 类。
- 建议: 在各视图中逐步替换主要操作按钮为 `<GlassButton variant="primary">` 等

### 8. 数据库 Schema 版本化迁移
`database/migrations.rs` 仅使用 `CREATE TABLE IF NOT EXISTS`，无版本追踪。
- 建议: 添加 `schema_version` 表 + 增量迁移脚本

### 9. Toast 通知系统整合
存在三套反馈系统: `appStore.setError/setSuccess`、`uiStore.showToast`、`ToastContainer`。
- 当前 `setError/setSuccess` 已支持 auto-dismiss（5s/3s）
- 建议: 废弃 `uiStore.showToast`，统一使用 `appStore.setError/setSuccess`

### 10. Rust 警告清理（9 个）
```
src/services/file_service.rs:53          — field `config` never read
src/services/password_book_service.rs:599 — method `row_to_password_entry` never used
src/database/connection.rs:51            — field `config` never read
src/crypto/key_management.rs:35          — field `keys_dir` never read
src/utils/async_utils.rs:343             — fields `refill_interval`, `max_tokens`
src/utils/async_utils.rs:403             — field `batch_timeout`
src/task_queue/task_executor.rs:22       — field `config` never read
src/task_queue/task_manager.rs:30        — fields `batch_processor`, `persistence_manager`, `config`
src/task_queue/batch_task_processor.rs:68 — field `config` never read
```
- 修复: 在对应 struct 上添加 `#[allow(dead_code)]`，或移除/重命名未使用字段

---

## P3 — 长期改进

### 11. README 英文版
当前 README 只有中文，建议添加英文版本或双语。

### 12. 自动更新
`tauri.conf.json` 无 `updater` 段配置。

### 13. E2E 测试
只有 1 个 Playwright spec (`e2e/specs/file_upload.spec.ts`)。
- 建议: 添加压缩→验证→解压→内容比对的完整管道测试

### 14. 性能优化
- 大文件解压进度反馈
- 任务列表虚拟滚动（50+ 任务时）
- IOBufferPool 内存池优化

---

## 已完成的关键修复（参考）

| 域 | 修复内容 |
|----|----------|
| 安全 | Zip Slip 防护、密码脱敏日志、每安装随机密钥、密码掩码显示 |
| 稳定 | 9 个 unwrap/panic 消除、Tauri 路由修复、非 UTF-8 路径安全 |
| 功能 | 任务队列连通、分卷 ZIP、密码 ZIP、Zstd/LZMA/ISO+20 格式、导入导出、打开文件夹 |
| UX | 按钮样式、错误消息、删除确认(含i18n)、批量容错、状态指示器、消息自动消失、窗口记忆 |
| 代码 | 55 包移除(PrimeVue)、4 个死组件/Store 清理、generateId 统一、主题系统统一、~500 行去重 |
| 测试 | 35 集成测试通过(6 套件)、3 个测试文件 API 迁移修复、新增 fixes_validation_test |
