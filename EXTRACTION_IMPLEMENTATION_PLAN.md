# 解压全流程与智能干预系统 - 实施里程碑 (Implementation Milestones)

## 0. 开发原则 (Core Principles)
1. **最小侵入**：在现有 `CompressionService` 框架下做增量修改，不推翻现有格式识别逻辑。
2. **停顿确认**：每个里程碑完成后停止，由用户评估代码质量和逻辑准确性。
3. **魔鬼颗粒度**：复杂的逻辑（如流式加载、异步挂起）拆分为细小的 PR 级提交，确保无死角。

---

## 里程碑 1：任务状态机扩展与信号中心 (Foundation)
**目标**：构建任务挂起的物理基础，支持信号量的持久占用。

- [ ] **[BE-基础] 状态枚举扩展**：
    - 修改 `src-tauri/src/models/compression.rs`，在 `CompressionStatus` 中增加 `Conflict` 和 `AlgorithmRequired`。
- [ ] **[BE-基础] 信号注册中心 (TaskRegistry)**：
    - 创建 `src-tauri/src/services/task_registry.rs`。
    - 使用 `DashMap` 存储 `task_id -> oneshot::Sender<Resolution>` 的映射。
    - 实现 `register_suspension`, `resolve_task`, `cleanup` 接口。
- [ ] **[BE-基础] Tauri 状态注入**：
    - 在 `main.rs` 中将 `TaskRegistry` 注册为全局状态。

---

## 里程碑 2：智能密码链升级 (Silent Password Chain)
**目标**：实现 [BE-PASS-003] 和 [BE-PASS-004]，确保“静默优先”。

- [ ] **[BE-PASS-003] 流式 JSON 字典加载器**：
    - 在 `PasswordAttemptService` 中实现基于 `tokio::io::BufReader` 的流式读取逻辑。
    - 确保百万级字典不会导致内存溢出，并加入 `tokio::task::yield_now()` 防止阻塞调度。
- [ ] **[BE-PASS-004] 密码尝试链路集成**：
    - 重构 `attempt_passwords_smartly`。
    - 按照权重：高频数据库 -> 文件名上下文猜测 -> JSON 暴力字典。
    - 确保如果所有尝试均失败，不再立即返回 Error，而是返回一个“挂起建议”。

---

## 里程碑 3：挂起引擎与冲突拦截 (Suspension Engine)
**目标**：实现 [BE-FLOW-001]，让任务在遇到障碍时“原地待命”。

- [ ] **[BE-FLOW-001] 信号量持有重构**：
    - 修改 `CompressionService::extract`。
    - 在任务启动时申请 `SemaphorePermit`，并确保该许可跨越整个 `extract` 生命周期（包括挂起期间）。
- [ ] **[BE-FLOW-001] 冲突拦截器实现**：
    - 在解压循环的 IO 写入前插入 `check_conflict` 逻辑。
    - 触发 `file-conflict` 事件，并通过 `oneshot::Receiver` 阻塞当前任务线程。
- [ ] **[BE-FLOW-001] 算法授权拦截**：
    - 当密码链失效且开启了高级模式，进入 `AlgorithmRequired` 挂起态。

---

## 里程碑 4：指令对接与前端交互 (Command & UI)
**目标**：打通闭环，实现用户干预后的任务恢复。

- [ ] **[BE-指令] 恢复命令实现**：
    - 实现 `resolve_conflict(task_id, strategy)`。
    - 实现 `authorize_algorithm(task_id, proceed)`。
    - 通过 `TaskRegistry` 查找对应的 `Sender` 并发送恢复信号。
- [ ] **[FE-FLOW-001] 冲突解决弹窗**：
    - 在 Vue 中实现详细对比弹窗（文件名、大小、修改时间）。
    - 增加“后续全应用”策略记忆逻辑。
- [ ] **[FE-FLOW-002] 终态保留逻辑**：
    - 修改 `TaskStore.ts`，确保失败任务不会被自动移除。

---

## 验收标准 (Acceptance Criteria)
1. **Slot 占用验证**：当一个任务处于冲突挂起时，观察到队列中的并发插槽依然被占用。
2. **原地恢复验证**：用户点击“覆盖”后，任务立即在原进度处继续，无需重启。
3. **内存压力测试**：加载 1GB 的 JSON 字典，内存波动保持在 50MB 以内。
