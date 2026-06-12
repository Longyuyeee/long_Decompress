# 胧解压·方便助手 (LongDecompress)

一款基于 **Rust + Tauri 1.5 + Vue 3** 构建的桌面压缩/解压工具。支持 37+ 种解压格式和 16 种压缩格式，提供密码管理、批量处理、智能密码尝试等功能，界面采用苹果毛玻璃设计风格。

> **English**: A desktop compression/decompression tool built with Rust + Tauri 1.5 + Vue 3. Supports 37+ extraction formats and 16 compression formats with password management, batch processing, and a frosted-glass UI.
>
> **Key highlights**: ZIP/7Z/RAR password extraction, AES-256 encrypted vault, split archives, content preview, integrity testing, ZIP repair, 35 integration tests passing, zero Rust warnings.

---

## 目录

- [功能特点](#功能特点)
- [格式支持](#格式支持)
- [技术架构](#技术架构)
- [项目结构](#项目结构)
- [快速开始](#快速开始)
- [开发指南](#开发指南)
- [当前状态](#当前状态)
- [剩余工作](#剩余工作)

---

## 功能特点

### 解压
- **拖放操作**：将压缩文件直接拖入窗口即可添加任务
- **批量解压**：支持同时处理多个压缩文件，每个文件独立追踪进度
- **选择性启动**：通过复选框选择要执行的待处理任务
- **智能密码尝试**：自动从密码保险箱中检索高频密码尝试解锁加密文件
- **密码输入弹窗**：当后端检测到加密文件但密码为空时，自动弹出密码需求提示
- **任务状态可视化**：每个任务有独立的颜色编码状态图标和进度条
  - 蓝色旋转图标：进行中（preparing/extracting/compressing/finalizing）
  - 绿色勾：已完成
  - 红色感叹号：失败
  - 灰色禁止：已取消
- **取消反馈**：取消成功/失败有明确的计数提示
- **路径穿越防护**：Zip Slip 攻击防护，所有归档条目路径经过规范化校验
- **7z CLI 兜底引擎**：对于原生库不支持的格式，自动回退到系统安装的 7z 处理

### 压缩
- **多格式选择**：ZIP / 7Z / RAR / TAR / GZ / BZ2 / XZ / Zstd / LZMA / TAR.GZ / TAR.BZ2 / TAR.XZ / TAR.Zst
- **密码压缩**：ZIP（通过 7z CLI）、7Z、RAR 支持设置密码
- **分卷压缩**：支持按指定大小拆分为多个卷（ZIP 格式）
- **压缩组**：可将多个文件/文件夹磁吸成组，每组独立配置压缩参数
- **组内文件管理**：支持从压缩组中移除单个文件
- **删除源文件选项**：压缩完成后可选择删除原始文件（带安全检查）
- **格式校验**：压缩前自动校验格式-选项兼容性（密码支持、单文件流限制等）

### 密码保险箱
- **加密存储**：AES-256-GCM 加密存储所有密码凭证
- **每安装随机主密钥**：每个安装实例生成唯一的 32 字节随机主密钥，不再使用硬编码默认密码
- **密码掩码**：保险箱表格默认显示为 `•••••••`，点击表头眼睛图标切换全局可见
- **一键复制**：鼠标悬停时每行密码旁出现复制按钮
- **JSON 导入/导出**：支持将密码本导出为 JSON 文件备份，或从 JSON 文件批量导入
- **使用频率追踪**：每个密码条目记录累计使用次数，支持按热度排序
- **搜索过滤**：按名称、密码内容、备注实时过滤
- **删除确认**：单条删除和全部清空均有确认弹窗
- **空状态引导**：保险箱为空时显示友好提示

### 界面与体验
- **5 种主题模式**：亮色 (Light)、暗色 (Dark)、赛博粉蓝 (Cyberpunk)、暮色极光 (Twilight)、纸质护眼 (Sepia)
- **13 种强调色**：在设置中心自由切换
- **窗口状态记忆**：启动时恢复上次的窗口位置和大小
- **消息自动消失**：错误消息 5 秒、成功消息 3 秒后自动清除
- **首次使用引导**：解压视图空状态显示 App 名称、标语和支持的格式徽章
- **国际化**：简体中文 / English 双语言支持
- **响应式布局**：左侧导航栏 + 右侧内容区，适配不同窗口大小
- **无障碍**：Modal 支持 `aria-labelledby` 和焦点管理，FileDropzone 支持 `role="button"` 和键盘操作，ProgressBar 支持 `role="progressbar"` 和 `aria-valuenow`

### 设置与持久化
- **设置后端同步**：应用设置同时存储于 localStorage 和后端 `app_settings.json`，浏览器数据清除后可从后端恢复
- **暴力破解引擎**：支持导入 TXT 密码本文件，解压时自动遍历尝试
- **Windows 自启动**：支持注册表写入，开机自动启动
- **自动锁定**：可配置空闲时间后自动锁定密码保险箱

---

## 格式支持

### 解压格式（37+ 种）

| 引擎 | 格式 | 密码支持 |
|------|------|----------|
| **原生 Rust 库** | ZIP (.zip) | ✅ 解压支持 |
| **原生 Rust 库** | 7Z (.7z) | ✅ 解压支持 |
| **原生 Rust 库 + CLI 回退** | RAR (.rar, .cbr) | ✅ 解压支持 |
| **原生 Rust 库** | TAR (.tar) | ❌ 无加密 |
| **原生 Rust 库** | GZIP (.gz) | ❌ 无加密 |
| **原生 Rust 库** | BZIP2 (.bz2) | ❌ 无加密 |
| **原生 Rust 库** | XZ (.xz) | ❌ 无加密 |
| **原生 Rust 库** | TAR.GZ (.tar.gz, .tgz) | ❌ 无加密 |
| **原生 Rust 库** | TAR.BZ2 (.tar.bz2, .tbz, .tbz2) | ❌ 无加密 |
| **原生 Rust 库** | TAR.XZ (.tar.xz, .txz) | ❌ 无加密 |
| **7z CLI 兜底** | Zstandard (.zst, .zstd) | ❌ |
| **7z CLI 兜底** | TAR.Zst (.tar.zst, .tzst) | ❌ |
| **7z CLI 兜底** | ISO 光盘映像 (.iso, .img) | ❌ |
| **7z CLI 兜底** | CAB (.cab) | ❌ |
| **7z CLI 兜底** | LZH/LHA (.lzh, .lha) | ❌ |
| **7z CLI 兜底** | ARJ (.arj) | ❌ |
| **7z CLI 兜底** | DMG (.dmg) | ❌ |
| **7z CLI 兜底** | WIM (.wim) | ❌ |
| **7z CLI 兜底** | VHD/VHDX (.vhd, .vhdx) | ❌ |
| **7z CLI 兜底** | CHM (.chm) | ❌ |
| **7z CLI 兜底** | DEB (.deb) | ❌ |
| **7z CLI 兜底** | RPM (.rpm) | ❌ |
| **7z CLI 兜底** | SQUASHFS (.squashfs, .sfs) | ❌ |
| **7z CLI 兜底** | NSIS (.nsis) | ❌ |
| **7z CLI 兜底** | MSI (.msi) | ❌ |
| **7z CLI 兜底** | XAR (.xar) | ❌ |
| **7z CLI 兜底** | CPIO (.cpio) | ❌ |
| **7z CLI 兜底** | UDF (.udf) | ❌ |
| **7z CLI 兜底** | FAT (.fat) | ❌ |
| **7z CLI 兜底** | NTFS (.ntfs) | ❌ |
| **7z CLI 兜底** | HFS/HFSX (.hfs, .hfsx) | ❌ |
| **7z CLI 兜底** | LZMA (.lzma) | ❌ |
| **7z CLI 兜底** | ALZ (.alz) | ❌ |
| **7z CLI 兜底** | ARC (.arc) | ❌ |
| **7z CLI 兜底** | APFS (.apfs) | ❌ |
| **7z CLI 兜底** | EXT2/3/4 (.ext2, .ext3, .ext4) | ❌ |

### 压缩格式（16 种）

| 格式 | 引擎 | 密码 | 说明 |
|------|------|------|------|
| ZIP (.zip) | `zip` crate 0.6 Deflated | ❌ 无密码 | 标准 ZIP 压缩，支持多文件/文件夹 |
| ZIP 密码 (.zip) | 7z CLI | ✅ | 通过 7z CLI 创建 AES 加密 ZIP |
| ZIP 分卷 (.z01, .z02...) | SplitCompressionService | ❌ | 按指定大小拆分为多个卷 |
| 7Z (.7z) | `sevenz-rust` crate | ✅ | 支持 AES-256 加密 |
| RAR (.rar) | WinRAR/rar CLI | ✅ | 需要系统安装 WinRAR 或 rar 命令行工具 |
| TAR (.tar) | `tar` crate | ❌ | 仅打包，不压缩 |
| GZ (.gz) | `flate2` crate | ❌ | 单文件流压缩 |
| BZ2 (.bz2) | `bzip2` crate | ❌ | 单文件流压缩 |
| XZ (.xz) | `xz2` crate | ❌ | 单文件流压缩 |
| Zstd (.zst) | 7z CLI | ❌ | 单文件流压缩 |
| LZMA (.lzma) | 7z CLI | ❌ | 单文件流压缩 |
| TAR.GZ (.tar.gz, .tgz) | tar + flate2 | ❌ | tar 打包 + gzip 压缩 |
| TAR.BZ2 (.tar.bz2, .tbz2) | tar + bzip2 | ❌ | tar 打包 + bzip2 压缩 |
| TAR.XZ (.tar.xz, .txz) | tar + xz2 | ❌ | tar 打包 + xz 压缩 |
| TAR.Zst (.tar.zst, .tzst) | 7z CLI | ❌ | tar 打包 + zstd 压缩 |

---

## 技术架构

### 前端 (long-compress-assistant/src/)

| 层 | 技术 |
|----|------|
| 框架 | Vue 3.4 + TypeScript 5.3 |
| 状态管理 | Pinia |
| 路由 | Vue Router 4 (Hash 模式) |
| 样式 | Tailwind CSS 3.4 + 自定义 CSS 变量 |
| 图标 | PrimeIcons (pi-* CSS 类) |
| 构建 | Vite 5 |
| 测试 | Vitest |

### 后端 (long-compress-assistant/src-tauri/)

| 层 | 技术 |
|----|------|
| 框架 | Tauri 1.5 |
| 语言 | Rust edition 2021 |
| 数据库 | SQLite (sqlx 0.7) |
| 加密 | AES-256-GCM (aes-gcm 0.10), Argon2 (argon2 0.5) |
| 压缩库 | zip 0.6, sevenz-rust 0.5, unrar 0.5, flate2 1.0, tar 0.4, bzip2 0.4, xz2 0.1 |
| 异步 | Tokio 1.36 |
| 并行 | Rayon 1.8, Crossbeam 0.8 |
| 序列化 | Serde + serde_json |
| 测试 | cargo test（35 个集成测试，6 个测试套件） |

### 关键依赖表

```toml
[dependencies]
tauri = { version = "1.5", features = ["api-all"] }
tokio = { version = "1.36", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "chrono", "uuid"] }
zip = "0.6"
sevenz-rust = { version = "0.5", features = ["aes256"] }
unrar = "0.5"
flate2 = "1.0"
tar = "0.4"
bzip2 = "0.4"
xz2 = "0.1"
aes-gcm = "0.10"
argon2 = "0.5"
rand = "0.8"
base64 = "0.21"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.7", features = ["v4", "serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
dashmap = "5.5"
rayon = "1.8"
walkdir = "2.5"
zxcvbn = "2.2"
```

---

## 项目结构

```
long_Decompress/
├── README.md                         # 本文件
├── REMAINING_WORK.md                 # 剩余工作清单（14 项 P0-P3）
├── long-compress-assistant/          # Tauri + Vue 3 主项目
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── tailwind.config.js
│   ├── src/                          # Vue 3 前端
│   │   ├── main.ts                   # App 入口（无 PrimeVue）
│   │   ├── App.vue                   # 根组件（窗口状态持久化 + 自动锁定）
│   │   ├── router/index.ts           # Hash 路由（4 条路由 + 404 catch-all）
│   │   ├── views/
│   │   │   ├── DecompressView.vue    # 解压工作区（拖放 + 任务表格 + 密码输入）
│   │   │   ├── CompressionView.vue   # 压缩工作区（格式选择 + 压缩组 + 分卷）
│   │   │   ├── PasswordVaultView.vue # 密码保险箱（CRUD + 掩码 + 导入导出 + 空状态）
│   │   │   └── SettingsView.vue      # 设置中心（主题/语言/强调色/暴破引擎）
│   │   ├── stores/                   # Pinia 状态管理
│   │   │   ├── app.ts                # 全局设置（主题、语言、错误消息自动消失）
│   │   │   ├── compression.ts        # 压缩状态（文件/组/全局设置）
│   │   │   ├── task.ts               # 任务状态（进度监听/冲突检测/密码需求/ls）
│   │   │   ├── password.ts           # 密码保险箱（初始化/解锁/强度评估）
│   │   │   ├── ui.ts                 # UI 状态（侧边栏/暗色模式委派 appStore）
│   │   │   └── config.ts             # 配置状态
│   │   ├── components/
│   │   │   ├── layouts/
│   │   │   │   ├── MainLayout.vue    # 主布局（侧边导航 + 标题栏 + 内容区）
│   │   │   │   └── WindowTitleBar.vue # 自定义窗口标题栏
│   │   │   ├── tasks/
│   │   │   │   ├── AeroTable.vue     # 解压任务表格（复选框/状态图标/密码输入/日志）
│   │   │   │   ├── TaskList.vue      # 通用任务列表
│   │   │   │   ├── TaskDetailPanel.vue
│   │   │   │   ├── CompressionAeroTable.vue
│   │   │   │   └── ConflictResolutionModal.vue
│   │   │   ├── compression/
│   │   │   │   └── CompressionSettingsPanel.vue  # 压缩配置面板
│   │   │   ├── passwords/
│   │   │   │   ├── PasswordEntryModal.vue       # 密码录入弹窗
│   │   │   │   ├── PasswordModal.vue            # 密码弹窗（旧版）
│   │   │   │   └── PasswordManagerNew.vue       # 新密码管理器
│   │   │   ├── ui/
│   │   │   │   ├── GlassCard.vue       # 毛玻璃卡片
│   │   │   │   ├── GlassButton.vue     # 毛玻璃按钮（variant/size/loading）
│   │   │   │   ├── EnhancedFileDropzone.vue  # 增强拖放区（含 ARIA）
│   │   │   │   ├── FileDropzone.vue    # 旧版拖放区
│   │   │   │   ├── Modal.vue           # 模态弹窗（含 ARIA + 焦点管理）
│   │   │   │   ├── ProgressBar.vue     # 进度条（含 ARIA）
│   │   │   │   ├── ThemeToggle.vue     # 主题切换
│   │   │   │   ├── ToastContainer.vue  # Toast 通知容器
│   │   │   │   └── ...
│   │   │   └── transitions/
│   │   │       └── PageTransition.vue
│   │   ├── composables/
│   │   │   ├── useTauriCommands.ts    # 前后端命令桥接（20+ 方法）
│   │   │   └── useTheme.ts            # 主题切换逻辑
│   │   ├── i18n/index.ts             # 中英文翻译包
│   │   ├── utils/index.ts            # 工具函数（extractErrorMessage, generateId 等）
│   │   └── styles/
│   │       ├── design-tokens.css      # 设计令牌（CSS 变量 + 5 种主题模式）
│   │       ├── responsive-utilities.css
│   │       └── animation-utilities.css
│   └── src-tauri/                     # Rust 后端
│       ├── Cargo.toml                 # Rust 依赖
│       ├── tauri.conf.json            # Tauri 配置
│       ├── build.rs
│       ├── src/
│       │   ├── main.rs                # Tauri 入口（28 个命令注册）
│       │   ├── lib.rs                 # 模块声明
│       │   ├── commands/
│       │   │   ├── compression.rs     # extract_file, extract_multiple, compress_files, cancel_compression, check_rar_compression_support
│       │   │   ├── file.rs            # list_files, get_file_info, validate_wordlists
│       │   │   ├── password.rs        # CRUD 密码操作
│       │   │   ├── encrypted_password.rs  # 加密密码服务命令
│       │   │   ├── system.rs          # 系统信息 / 自启动 / 设置持久化
│       │   │   ├── system_integration.rs  # 通知 / 权限 / 打开文件夹
│       │   │   └── task_queue.rs      # 任务队列管理命令
│       │   ├── services/
│       │   │   ├── compression_service.rs   # 【核心】1627 行压缩/解压主引擎
│       │   │   ├── archive_engine.rs        # 归档引擎 trait（extract_with_progress）
│       │   │   ├── universal_engine.rs      # 7z CLI 兜底引擎
│       │   │   ├── rar_support.rs           # RAR 支持（原生 crate → unrar CLI → 7z CLI）
│       │   │   ├── split_compression.rs     # 分卷压缩（SplitCompressionService）
│       │   │   ├── parallel_extraction.rs   # 并行解压（Rayon）
│       │   │   ├── password_attempt_service.rs  # 密码尝试策略
│       │   │   ├── password_query_service.rs    # 密码本查询
│       │   │   ├── password_book_service.rs     # 密码本服务
│       │   │   ├── password_strength_service.rs # 密码强度评估
│       │   │   ├── password_category_service.rs # 密码分类
│       │   │   ├── encrypted_password_service.rs # 加密密码核心（含随机主密钥）
│       │   │   ├── file_service.rs          # 文件操作
│       │   │   ├── io_buffer_pool.rs        # IO 缓冲池
│       │   │   └── system_service.rs        # 系统服务
│       │   ├── models/
│       │   │   ├── compression.rs       # 压缩/解压数据模型（格式/选项/状态/日志）
│       │   │   ├── file.rs              # 文件模型
│       │   │   ├── password.rs          # 密码模型
│       │   │   └── system.rs            # 系统模型
│       │   ├── database/
│       │   │   ├── connection.rs        # 数据库连接管理
│       │   │   ├── models.rs            # 数据库模型
│       │   │   ├── migrations.rs        # 数据迁移
│       │   │   └── repositories.rs      # 数据仓库
│       │   ├── crypto/
│       │   │   ├── encryption.rs        # AES-256-GCM 加密
│       │   │   ├── hashing.rs           # Argon2 哈希
│       │   │   └── key_management.rs    # 密钥管理
│       │   ├── config/
│       │   │   ├── models.rs            # 配置模型
│       │   │   ├── service.rs           # 配置服务
│       │   │   ├── repository.rs        # 配置仓库
│       │   │   ├── commands.rs          # 配置命令
│       │   │   └── validation.rs        # 配置验证
│       │   ├── task_queue/
│       │   │   ├── task_queue.rs        # 任务队列
│       │   │   ├── task_scheduler.rs    # 后台调度循环（资源感知）
│       │   │   ├── task_executor.rs     # 任务执行器（连通 CompressionService）
│       │   │   ├── task_manager.rs      # 任务管理器
│       │   │   ├── models.rs            # 队列数据模型
│       │   │   ├── batch_task_processor.rs  # 批量任务处理
│       │   │   └── task_persistence.rs  # 任务持久化
│       │   ├── system_integration/
│       │   │   ├── notification.rs      # 系统通知
│       │   │   ├── file_association.rs  # 文件关联
│       │   │   ├── global_shortcut.rs   # 全局快捷键
│       │   │   ├── permission_manager.rs # 权限管理
│       │   │   └── platform_compatibility.rs # 平台兼容
│       │   └── utils/
│       │       ├── error.rs             # 错误类型
│       │       ├── file_utils.rs        # 文件工具（含路径安全函数）
│       │       ├── io_utils.rs          # IO 工具
│       │       ├── async_utils.rs       # 异步工具
│       │       ├── validation.rs        # 验证工具
│       │       └── formatting.rs        # 格式化工具
│       └── tests/                       # 集成测试
│           ├── compression_capabilities_regression.rs  # 7 测（格式验证/源文件清理/UI 属性）
│           ├── fixes_validation_test.rs                # 7 测（Zip Slip/分卷写入/魔术字节/验证规则）
│           ├── split_compression_test.rs               # 5 测（基本/不分卷/零大小/大文件/不存在）
│           ├── rar_support_test.rs                     # 7 测（安装检测/创建/不存在/验证/列表/密码）
│           ├── password_zip_test.rs                    # 4 测（选项/ZIP拒绝/7Z接受/密码检测）
│           ├── zip_compression_test.rs                 # 5 测（基本/多文件/默认值/可删除源/等待输出）
│           └── fixtures/test_helpers.rs
├── long_decompress/                    # Python 原型（遗留）
├── prototype/                          # 原型代码
├── docs/                               # 设计文档
└── icon.png
```

---

## 快速开始

### 环境要求

| 工具 | 版本 |
|------|------|
| Node.js | 18+ |
| Rust | 1.70+ |
| 7-Zip | 推荐安装（用于通用格式支持） |
| WinRAR/rar | 可选（用于创建 RAR 压缩文件） |

### 克隆与安装

```bash
# 克隆仓库
git clone https://github.com/Longyuyeee/long_Decompress.git
cd long_Decompress/long-compress-assistant

# 安装前端依赖
npm install
```

### 开发模式

```bash
# 启动 Tauri 开发模式（热重载前端 + Rust 后端）
npm run tauri dev
```

### 生产构建

```bash
# 构建可分发的安装包
npm run tauri build
```

### 运行测试

```bash
# ---- 前端单元测试 ----
npm test

# ---- Rust 集成测试 ----
cd src-tauri

# 运行所有 6 个测试套件（共 35 个测试）
cargo test --test compression_capabilities_regression  # 7 passed
cargo test --test fixes_validation_test                # 7 passed
cargo test --test split_compression_test               # 5 passed
cargo test --test rar_support_test                     # 7 passed
cargo test --test password_zip_test                    # 4 passed
cargo test --test zip_compression_test                 # 5 passed

# 检查编译（零错误）
cargo check --lib
```

---

## 当前状态

项目经历了一次全面的审计和系统化修复（56 个提交，17 轮迭代），主要改进：

| 维度 | 修复前 | 修复后 |
|------|--------|--------|
| CRITICAL 安全漏洞 | 6 | **0** |
| HIGH 问题 | 13 | **5** |
| MEDIUM 问题 | 24 | **9** |
| Rust 库编译错误 | 38+ | **0** |
| 前端编译错误 | 多个 | **0** |
| 集成测试 | 0 通过 | **35 全部通过** |
| 死包依赖 | PrimeVue 55 个包 | **已移除** |
| 代码重复 | ~500 行 | **已消除** |
| 崩溃风险 (.unwrap) | 9 处 | **0** |

### 安全修复亮点

- Zip Slip 路径穿越防护（`normalize_archive_path` + `verify_extract_path` 共享工具函数）
- 密码日志脱敏（移除明文密码，改为只记录长度和索引）
- 每安装实例随机主密钥（替换硬编码 `'long-decompress-default-key'`）
- 密码保险箱默认掩码显示（带全局切换开关）
- Cyberpunk 主题品红色文本 WCAG AA 对比度修复

### 测试覆盖

| 测试套件 | 测试数 | 验证范围 |
|----------|--------|----------|
| compression_capabilities_regression | 7 | 格式校验、源文件清理、覆盖模式 |
| fixes_validation_test | 7 | Zip Slip 防护、分卷写入、魔术字节检测、格式规则 |
| split_compression_test | 5 | 基本分卷、不分卷、零大小、大文件、不存在文件 |
| rar_support_test | 7 | RAR 工具检测、创建、解压不存在、验证、信息、列表 |
| password_zip_test | 4 | 密码选项、ZIP 密码支持、7Z 密码支持、密码检测 |
| zip_compression_test | 5 | 基本 ZIP、多文件、默认值、可删除源、等待输出 |

---

## 剩余工作

详见仓库根目录 **[REMAINING_WORK.md](REMAINING_WORK.md)**。按优先级：

| 优先级 | 项目 | 说明 |
|--------|------|------|
| P0 | 安装器配置 | `tauri.conf.json` 无 `bundle` 段 |
| P0 | CI/CD | 无 GitHub Actions |
| P0 | 11 个测试文件 | 需 API 迁移（参照已修复的模式） |
| P1 | 密码 CLI 暴露 | 进程列表中可见密码参数 |
| P1 | 表单验证 | 设置页面缺少输入校验 |
| P2 | GlassButton | 设计系统未在生产视图落地 |
| P2 | DB 版本迁移 | 仅 `CREATE TABLE IF NOT EXISTS` |
| P2 | Toast 统一 | 三套反馈系统并存 |
| P2 | Rust 警告 9 个 | unused field 清理 |
| P3 | 英文 README | 当前为中文 |
| P3 | 自动更新 | 无 updater 配置 |
| P3 | E2E 测试 | 仅 1 个 Playwright spec |

---

## 开发规范

- 提交遵循 `type: description` 格式（如 `fix: ...`、`feat: ...`、`refactor: ...`）
- 后端 `unwrap()` / `panic!()` 一律替换为 `Result<T>` 或 graceful fallback
- 新增格式需同步更新：后端 ArchiveFormat/CompressionFormat + 魔术字节检测 + 扩展名映射 + 前端 accept 列表
- 密码相关代码不得写入日志（使用脱敏版本）
- 所有新增修改需通过 `cargo check --lib` 和 `npx vite build` 验证

---

## 许可证

MIT

---

**胧解压·方便助手 (LongDecompress)** — 引擎强大，界面极简。
