# 增强型解压缩系统设计文档 (Part 1: 核心引擎与错误控制)

## 1. 总体目标
构建一个能够自动关联密码本、支持高并发流式解压、且具备精准错误反馈的解压缩核心。

## 2. 核心开发原则
- **自动尝试**：解压加密文件时，自动按使用频率从高到低尝试密码本中的密码。
- **容错执行**：单个文件失败不阻塞队列，记录详细错误并继续处理后续任务。
- **性能优化**：通过 Buffer Pool 和流式处理减少内存拷贝，针对多核优化并行度。

## 3. 任务分解 (Work Items) - 阶段一：后端引擎与错误映射

### [BE-CORE-001] 完善错误枚举 `CompressionError`
- **目标**：将模糊的“解压失败”细化为可被前端识别的业务状态。
- **细化项**：
    - `PasswordRequired`: 归档已加密但未提供密码。
    - `InvalidPassword`: 提供的密码验证失败。
    - `UnsupportedEncryption`: 使用了库不支持的加密算法（如部分 RAR5 变体）。
    - `DiskFull`: 目标磁盘空间不足。
    - `PartialSuccess`: 批量任务中部分文件成功，部分失败。

### [BE-CORE-002] 引入高级 RAR 引擎支持
- **目标**：解决 `sevenz-rust` 对加密 RAR5 兼容性较弱的问题。
- **方案**：
    - 调研并集成 `unrar` 或 `unpax` 库的 Rust 绑定。
    - 实现格式嗅探逻辑：在解压前通过文件头（Magic Number）精准识别 RAR4 vs RAR5。
    - 确保加密 RAR 的 Header 加密情况也能被正确识别。

### [BE-CORE-003] ZIP 解压流式重构
- **目标**：提升大文件加密 ZIP 的解压速度。
- **方案**：
    - 优化 `do_extract_zip` 中的循环，避免在循环内部重复初始化解密上下文。
    - 引入 `io_buffer_pool`，为解压流分配预设大小的缓冲区。
    - 实现“预检查”逻辑：在正式开始解压前，先尝试解密目录索引（Central Directory），以快速判断密码是否正确。

### [BE-CORE-004] 引擎分发器 (Dispatcher) 升级
- **目标**：更智能的格式路由。
- **方案**：
    - 修改 `extract` 方法，不再仅依赖文件后缀，而是结合 Magic Number 进行分发。
    - 为每个引擎实现 `supports_password()` 和 `requires_password()` 接口。

### [BE-CORE-005] 7z (7-Zip) 深度增强支持
- **目标**：完美支持 7z 格式的各种变体，特别是 Header 加密（隐藏文件名）的情况。
- **方案**：
    - 强化 `sevenz-rust` 的集成，处理 `Solid Archive`（固实压缩）模式下的部分文件提取。
    - **Header 加密处理**：实现预读逻辑，当 7z 文件的目录树被加密时，立即触发密码尝试流程，而不是等到解压具体文件时才报错。
    - 支持 LZMA, LZMA2, PPMd, BZip2 等多种底层压缩算法。

### [BE-CORE-006] 通用格式矩阵 (Universal Format Matrix) 与统一驱动
- **目标**：支持 30+ 格式，且**全格式**接入密码本尝试与进度反馈系统。
- **方案**：
    - **抽象引擎接口 (ArchiveEngine Trait)**：定义统一的 Rust Trait，包含 `try_password(&self, pwd: &str) -> bool` 和 `extract_with_progress(&self, target: &Path, callback: ProgressCallback)`。
    - **集成高性能通用库**：使用 `libarchive` 或 `p7zip` 动态链接/绑定作为底层，这些库天然支持解压绝大多数格式（如 ISO, CAB, VHD, XAR 等）的进度回调和密码输入。
    - **智能密码注入**：无论后端调用哪个具体库，统一由 `CompressionService` 在“预检阶段”循环注入密码本中的高频密码，直到底层驱动返回“解密成功”或“所有密码尝试完毕”。

### [BE-CORE-007] 全格式进度追踪器 (Universal Progress Tracker)
- **目标**：为所有解压任务提供毫秒级、字节级的进度反馈。
- **方案**：
    - 对于不支持内置进度回调的旧式库，通过**流式包装器 (Stream Wrapper)** 实现。在读取压缩包文件流时，通过计算 `Position / TotalSize` 强制生成进度数据。
    - 统一输出 `TaskProgress` 结构，包含：当前文件名、已解压大小、总大小、百分比。


---
*(待续：第二部分将涵盖密码本自动尝试策略、任务队列优化及前后端交互协议)*

## 4. 任务分解 (Work Items) - 阶段二：密码本策略与智能尝试

### [BE-PASS-001] 密码智能尝试策略实现 (PasswordAttemptStrategy)
- **目标**：自动从 `PasswordBookService` 获取高频密码并按顺序尝试。
- **方案**：
    - 在 `compression_service.rs` 中引入对 `password_query_service` 的调用。
    - **优先级算法**：优先按 `use_count` (使用次数) 降序排列，同次数下按 `last_used_at` (最后使用时间) 降序排列。
    - **并发限制**：限制自动尝试的前 N 个密码，防止过多无效读取影响 IO 寿命。
    - **成功回调**：密码尝试成功后，自动调用 `password_book_service` 的更新接口，增加该密码的使用计数。

### [BE-PASS-002] 异步尝试任务封装
- **目标**：在不阻塞主 UI 线程的前提下进行密码暴力尝试。
- **方案**：
    - 为每个加密压缩包生成一个“预检任务”。
    - 如果预检任务成功（找到密码），则直接进入解压阶段。
    - 如果预检失败（所有已知密码均无效），则通过 Tauri 事件向前端发起“需要密码” (PasswordRequired) 请求。

## 5. 任务分解 (Work Items) - 阶段三：性能调优与稳定性

### [BE-PERF-001] 任务并行度控制
- **目标**：避免大量任务同时解压缩导致系统 IO 假死。
- **方案**：
    - 实现全局令牌桶 (Semaphore)，限制最大并发解压任务数为 `num_cpus` 或用户设定值。
    - 针对 SSD vs HDD 的差异，优化并行度控制逻辑。

### [BE-PERF-002] 基于内存映射 (Mmap) 的大文件读取
- **目标**：减少频繁的大文件磁盘读取系统调用。
- **方案**：
    - 在支持的平台上（如 Win32），对超过 1GB 的压缩包采用内存映射读取技术。

## 6. 任务分解 (Work Items) - 阶段四：前端交互协议 (Frontend Handshake)

### [FE-INT-001] 增强型进度上报协议
- **协议格式定义**：
    ```json
    {
      "task_id": "uuid",
      "stage": "Pre-checking" | "Extracting" | "Finalizing",
      "current_password": "****", // 正在尝试的密码标识
      "progress": 0.45,
      "speed": "24MB/s"
    }
    ```

### [FE-INT-002] 密码反馈机制
- **目标**：友好的密码输入弹窗与结果反馈。
- **交互流程**：
    1. 前端监听到 `PasswordRequired` 事件。
    2. 弹出毛玻璃风格的密码输入框，并显示“已自动尝试 5 个已知密码，均未匹配”。
    3. 用户手动输入新密码后，密码自动通过 `add_password` 存入密码本，方便下次使用。

### [FE-INT-003] 失败重试逻辑
- **目标**：允许用户在任务列表中点击“重试”，并重新分配不同的密码。

---
## 7. 验收标准 (Acceptance Criteria)
1.  **ZIP 兼容性**：支持存储加密 (Store) 和压缩加密 (Deflate)，支持 ZipCrypto 和 AES-256 加密。
2.  **7z 兼容性**：支持 LZMA2 算法及 Header 加密。
3.  **RAR 兼容性**：完美支持 RAR4/RAR5 带密码文件，密码错误能立即感知并报错。
4.  **密码本联动**：解压时无需手动填入已知密码，系统能自动识别并解压成功。
5.  **性能基准**：优化后的流式解压在 100MB+ 文件下，内存占用应低于 150MB，CPU 利用率平稳。
