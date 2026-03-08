# 胧压缩·方便助手 - Rust + Tauri + Vue3 实施计划

## 技术栈确认
- **前端**: Vue 3 + TypeScript + PrimeVue + Tailwind CSS
- **后端**: Rust + Tauri + SQLite/SQLCipher
- **打包**: Tauri构建（Windows: exe, macOS: dmg, Linux: AppImage）

## 项目结构调整

### 项目目录结构
```
long-compress-assistant/
├── src-tauri/           # Rust后端
│   ├── src/
│   │   ├── main.rs      # Tauri入口
│   │   ├── commands/    # Rust命令（供前端调用）
│   │   │   ├── compression.rs    # 压缩解压命令
│   │   │   ├── password.rs       # 密码本管理命令
│   │   │   ├── file.rs           # 文件操作命令
│   │   │   └── system.rs         # 系统相关命令
│   │   ├── services/    # 业务服务
│   │   │   ├── compression_service.rs
│   │   │   ├── password_service.rs
│   │   │   ├── file_service.rs
│   │   │   └── task_queue.rs
│   │   ├── models/      # 数据模型
│   │   │   ├── password.rs
│   │   │   ├── compression.rs
│   │   │   └── task.rs
│   │   ├── database/    # 数据库模块
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs
│   │   │   ├── password_db.rs
│   │   │   └── migrations/
│   │   ├── crypto/      # 加密模块
│   │   │   ├── mod.rs
│   │   │   ├── encryption.rs
│   │   │   └── key_derivation.rs
│   │   └── utils/       # 工具函数
│   ├── Cargo.toml       # Rust依赖
│   ├── tauri.conf.json  # Tauri配置
│   └── build.rs         # 构建脚本
│
├── src/                 # Vue3前端
│   ├── assets/          # 静态资源
│   ├── components/      # Vue组件
│   │   ├── ui/          # 通用UI组件
│   │   │   ├── GlassCard.vue       # 毛玻璃卡片
│   │   │   ├── ProgressRing.vue    # 进度环
│   │   │   ├── FileDropZone.vue    # 文件拖放区
│   │   │   ├── FileList.vue        # 文件列表
│   │   │   ├── PasswordManager.vue # 密码本管理
│   │   │   ├── TaskQueue.vue       # 任务队列
│   │   │   └── SettingsPanel.vue   # 设置面板
│   │   ├── views/       # 页面组件
│   │   │   ├── MainView.vue        # 主界面
│   │   │   ├── CompressionView.vue # 压缩界面
│   │   │   ├── ExtractionView.vue  # 解压界面
│   │   │   ├── PasswordView.vue    # 密码本界面
│   │   │   ├── SettingsView.vue    # 设置界面
│   │   │   └── ReportView.vue      # 报告界面
│   │   └── layouts/     # 布局组件
│   │       ├── MainLayout.vue
│   │       └── AuthLayout.vue
│   ├── composables/     # Vue组合式函数
│   │   ├── useFileDrop.ts      # 文件拖放逻辑
│   │   ├── useCompression.ts   # 压缩解压逻辑
│   │   ├── usePassword.ts      # 密码本逻辑
│   │   ├── useTaskQueue.ts     # 任务队列逻辑
│   │   └── useSystem.ts        # 系统相关逻辑
│   ├── stores/          # Pinia状态管理
│   │   ├── fileStore.ts        # 文件状态
│   │   ├── taskStore.ts        # 任务状态
│   │   ├── passwordStore.ts    # 密码本状态
│   │   ├── settingsStore.ts    # 设置状态
│   │   └── uiStore.ts          # UI状态
│   ├── router/          # 路由配置
│   │   ├── index.ts
│   │   └── routes.ts
│   ├── types/           # TypeScript类型定义
│   │   ├── file.ts
│   │   ├── compression.ts
│   │   ├── password.ts
│   │   └── task.ts
│   ├── utils/           # 工具函数
│   │   ├── file.ts
│   │   ├── format.ts
│   │   ├── validation.ts
│   │   └── constants.ts
│   ├── App.vue          # 根组件
│   ├── main.ts          # 应用入口
│   └── style.css        # 全局样式
│
├── public/              # 公共资源
│   ├── icons/           # 图标资源
│   └── fonts/           # 字体文件
├── index.html           # HTML入口
├── package.json         # 前端依赖
├── vite.config.ts       # Vite配置
├── tailwind.config.js   # Tailwind CSS配置
├── tsconfig.json        # TypeScript配置
├── postcss.config.js    # PostCSS配置
├── .eslintrc.js         # ESLint配置
├── .prettierrc          # Prettier配置
└── README.md            # 项目说明
```

## Rust依赖调整

### Cargo.toml 配置
```toml
[package]
name = "long-compress-assistant"
version = "0.1.0"
description = "智能压缩解压助手"
authors = ["Your Name"]
license = "MIT"
edition = "2021"

[dependencies]
tauri = { version = "2.0.0", features = ["shell-open", "shell-execute", "notification-all", "tray", "global-shortcut"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
thiserror = "1.0"
log = "0.4"
env_logger = "0.11"

# 数据库相关
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "sqlcipher", "macros"] }
sqlx-sqlcipher = "0.1"  # SQLCipher支持
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# 文件操作
walkdir = "2.5"
filetime = "0.2"
path-absolutize = "3.1"

# 压缩解压库
zip = "0.6"              # ZIP格式支持
flate2 = "1.0"           # gzip支持
tar = "0.4"              # tar格式支持
bzip2 = "0.4"            # bzip2支持
xz2 = "0.1"              # xz支持
sevenz-rust = { git = "https://github.com/azzamsa/sevenz-rust" }  # 7z支持

# 加密和安全
rust-crypto = "0.2"      # 加密算法
ring = "0.17"            # 现代加密库
argon2 = "0.5"           # 密码哈希
aes-gcm = "0.10"         # AES-GCM加密

# 并发处理
rayon = "1.8"            # 数据并行
crossbeam = "0.8"        # 并发数据结构
dashmap = "5.5"          # 并发哈希表

# 工具库
regex = "1.10"           # 正则表达式
percent-encoding = "2.3" # URL编码
base64 = "0.21"          # Base64编码
hex = "0.4"              # 十六进制编码

[build-dependencies]
tauri-build = { version = "2.0.0" }

[dev-dependencies]
tempfile = "3.10"
```

## 开发阶段调整

### 阶段1：基础架构搭建 (3-4周)

#### 第1周：项目初始化和基础配置
1. **创建Tauri + Vue3项目** (1天)
   ```bash
   npm create tauri-app@latest
   # 选择 Vue + TypeScript 模板
   ```

2. **配置开发环境** (2天)
   - TypeScript配置
   - Tailwind CSS集成
   - ESLint + Prettier配置
   - Git版本控制

3. **设计系统建立** (2天)
   - 颜色主题系统（苹果风格）
   - 字体和间距规范
   - 毛玻璃效果CSS基础

#### 第2周：基础界面组件
1. **布局组件开发** (2天)
   - 主布局组件
   - 导航组件
   - 响应式布局

2. **核心UI组件** (3天)
   - GlassCard（毛玻璃卡片）
   - ProgressRing（进度环）
   - FileDropZone（文件拖放区）
   - 按钮和表单组件

#### 第3周：Rust核心服务
1. **Tauri命令系统** (2天)
   - 基础命令定义
   - 错误处理系统
   - 前后端通信接口

2. **文件服务模块** (3天)
   - 文件操作服务
   - 路径处理和安全检查
   - 文件类型检测

#### 第4周：数据库和配置
1. **SQLite + SQLCipher集成** (2天)
   - 数据库连接管理
   - 密码本数据模型
   - 加密存储实现

2. **配置管理系统** (2天)
   - 应用配置管理
   - 用户设置持久化
   - 主题系统实现

### 阶段2：核心功能实现 (4-5周)

#### 第5周：压缩解压基础
1. **ZIP格式支持** (2天)
   - 使用`zip`库实现ZIP解压
   - 使用`zip`库实现ZIP压缩
   - 密码保护支持

2. **其他格式基础** (3天)
   - tar/gz/bz2/xz格式支持
   - 调用系统命令备用方案
   - 格式自动检测

#### 第6周：文件管理和界面
1. **拖放文件管理** (2天)
   - Vue3拖放组件完善
   - 文件列表管理
   - 批量选择操作

2. **任务队列系统** (3天)
   - 任务状态管理
   - 进度反馈系统
   - 取消和暂停功能

#### 第7周：密码本管理基础
1. **密码存储和检索** (2天)
   - 密码CRUD操作
   - 基础搜索功能
   - 导入导出基础

2. **密码尝试逻辑** (3天)
   - 密码本匹配算法
   - 常见密码尝试
   - 成功密码记录

#### 第8周：现代化界面完善
1. **动画和交互效果** (2天)
   - 过渡动画
   - 微交互反馈
   - 加载状态管理

2. **系统集成** (3天)
   - 系统托盘集成
   - 全局快捷键
   - 桌面通知

### 阶段3：高级功能实现 (3-4周)

#### 第9周：7z和RAR格式支持
1. **7z格式集成** (2天)
   - 集成`sevenz-rust`库
   - 7z压缩解压实现
   - 加密7z支持

2. **RAR格式支持** (3天)
   - 调用unrar系统命令
   - RAR格式检测
   - 错误处理

#### 第10周：智能密码功能
1. **智能匹配算法** (2天)
   - 文件名相似度计算
   - 使用频率优先级
   - 智能建议系统

2. **密码生成和猜测** (3天)
   - 密码生成器
   - 常见模式猜测
   - 基于上下文的猜测

#### 第11周：暴力破解功能
1. **暴力破解界面** (2天)
   - 字符集配置界面
   - 破解进度显示
   - 资源使用监控

2. **破解算法实现** (3天)
   - 多线程破解
   - 进度估算算法
   - 结果缓存和恢复

#### 第12周：测试和优化
1. **全面测试** (2天)
   - 单元测试
   - 集成测试
   - 性能测试

2. **性能优化** (3天)
   - 内存使用优化
   - 并发性能优化
   - 启动速度优化

### 阶段4：发布和完善 (2-3周)

#### 第13周：打包和发布
1. **多平台打包** (2天)
   - Windows (exe)
   - macOS (dmg)
   - Linux (AppImage)

2. **安装程序制作** (3天)
   - Windows安装程序
   - macOS应用包
   - Linux包管理器支持

#### 第14周：文档和发布
1. **用户文档** (2天)
   - 使用指南
   - 常见问题
   - 更新日志

2. **正式发布** (3天)
   - 版本发布
   - 推广材料
   - 用户反馈收集

## 关键技术点

### Rust压缩库策略
1. **优先使用Rust原生库**
   - ZIP: `zip`库
   - gzip: `flate2`库
   - tar: `tar`库
   - bzip2: `bzip2`库
   - xz: `xz2`库

2. **7z格式方案**
   - 首选: `sevenz-rust`（可能需要贡献改进）
   - 备选: 调用系统7z命令

3. **RAR格式方案**
   - 调用unrar系统命令
   - 检测用户系统是否安装unrar
   - 提供安装指导

### 数据库加密方案
```rust
// 使用SQLCipher加密数据库
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx_sqlcipher::SqlCipherKey;

let key = SqlCipherKey::Bytes("your-encryption-key".as_bytes().to_vec());
let options = SqliteConnectOptions::new()
    .filename("passwords.db")
    .pragma("key", key.to_string())
    .create_if_missing(true);

let pool = SqlitePoolOptions::new()
    .connect_with(options)
    .await?;
```

### Vue3毛玻璃效果实现
```vue
<!-- GlassCard.vue -->
<template>
  <div class="glass-card" :style="cardStyle">
    <slot></slot>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  blur?: number
  saturation?: number
  opacity?: number
}>()

const cardStyle = computed(() => ({
  'backdrop-filter': `blur(${props.blur || 20}px) saturate(${props.saturation || 180}%)`,
  'background-color': `rgba(255, 255, 255, ${props.opacity || 0.2})`,
  'border-radius': '16px',
  'border': '1px solid rgba(255, 255, 255, 0.3)',
  'box-shadow': '0 8px 32px rgba(0, 0, 0, 0.1)'
}))
</script>

<style scoped>
.glass-card {
  transition: all 0.3s ease;
}

.glass-card:hover {
  background-color: rgba(255, 255, 255, 0.25);
  box-shadow: 0 12px 48px rgba(0, 0, 0, 0.15);
}
</style>
```

## 立即开始步骤

### 第一步：创建项目 (今天)
```bash
# 创建Tauri + Vue3项目
npm create tauri-app@latest long-compress-assistant
cd long-compress-assistant

# 选择模板
# - Which UI framework would you like to add? Vue
# - Which TypeScript mode would you like to add? Strict

# 安装UI库和工具
npm install primevue primeicons tailwindcss@latest postcss@latest autoprefixer@latest
npm install -D @tauri-apps/cli @types/node
```

### 第二步：基础配置 (今天)
1. 配置Tailwind CSS
2. 配置PrimeVue主题
3. 设置TypeScript路径别名
4. 配置Git忽略文件

### 第三步：创建基础组件 (明天)
1. 主布局组件
2. 毛玻璃卡片组件
3. 文件拖放组件
4. 进度显示组件

## 风险缓解

### 技术风险
1. **Rust压缩库不成熟**
   - 缓解：准备系统命令调用备用方案
   - 缓解：参与开源库改进或创建自己的简单实现

2. **Tauri性能问题**
   - 缓解：优化Rust代码性能
   - 缓解：合理使用Web Worker处理计算密集型任务

3. **跨平台兼容性**
   - 缓解：充分测试各平台
   - 缓解：使用条件编译处理平台差异

### 开发风险
1. **Rust学习曲线**
   - 缓解：从简单模块开始
   - 缓解：利用Rust优秀的学习资源
   - 缓解：先实现Python原型验证逻辑

2. **开发周期延长**
   - 缓解：明确优先级，先完成MVP
   - 缓解：定期评估进度，及时调整计划

## 成功指标

### 技术指标
- 启动时间 < 2秒
- 内存使用 < 200MB（处理大文件时）
- 解压速度接近系统原生工具
- 零崩溃率（关键操作）

### 产品指标
- 用户完成核心操作步骤数 ≤ 3
- 用户满意度评分 ≥ 4.5/5
- 月活跃用户增长 ≥ 20%
- 用户留存率 ≥ 60%

### 商业指标
- 专业版转化率 ≥ 5%
- 用户推荐率 ≥ 30%
- 插件市场活跃度（上线后6个月）

## 总结

Rust + Tauri + Vue3技术栈为"胧压缩·方便助手"提供了：
1. **卓越性能**：Rust原生性能，适合压缩解压任务
2. **现代界面**：Vue3 + Tailwind CSS实现苹果毛玻璃效果
3. **简单分发**：单个二进制文件，无环境依赖
4. **长期可维护**：类型安全，内存安全，代码质量高

虽然初期开发周期比Python方案稍长，但长期收益显著。建议按照四阶段计划稳步推进，先完成MVP验证核心功能，再逐步完善高级功能。

现在可以立即开始创建项目骨架！