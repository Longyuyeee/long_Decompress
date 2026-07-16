<div align="center">

# 🗜️ 胧解压·方便助手

**专业级解压缩工具 | Professional Archive Manager**

[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](https://github.com/Longyuyeee/long_Decompress)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-1.5-orange.svg)](https://tauri.app/)
[![Vue](https://img.shields.io/badge/Vue-3.4-brightgreen.svg)](https://vuejs.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75-red.svg)](https://www.rust-lang.org/)
[![WCAG](https://img.shields.io/badge/WCAG%202.1-AA%2FAAA-success.svg)](https://www.w3.org/WAI/WCAG21/quickref/)

[English](README_EN.md) | 简体中文

---

**现代化 · 高性能 · 无障碍**

支持 37+ 解压格式 · 16 种压缩格式 · 智能密码破解 · 分卷压缩 · 文件完整性校验

</div>

---

## ✨ 核心特性

### 🎯 格式支持全面

#### 解压支持 (37+ 格式)
- **原生 Rust**: ZIP, 7Z, RAR, TAR, GZ, BZ2, XZ, ZSTD
- **磁盘镜像**: ISO, IMG, VHD, VHDX, DMG
- **系统包**: DEB, RPM, MSI, NSIS
- **历史格式**: CAB, LZH, ARJ, ARC, ALZ
- **文件系统**: UDF, FAT, NTFS, HFS, APFS, EXT2/3/4, SquashFS

#### 压缩支持 (16 种)
- **标准格式**: ZIP, 7Z, TAR, GZ, BZ2, XZ
- **组合格式**: TAR.GZ, TAR.BZ2, TAR.XZ, TAR.ZSTD
- **加密压缩**: ZIP(密码), 7Z(密码), RAR(需WinRAR)
- **分卷压缩**: ZIP 分卷

---

### 🔐 智能密码管理

#### 密码保险箱
- **AES-256-GCM** 军事级加密
- **Argon2** 密钥派生算法
- 使用统计与热门密码排序
- 批量导入导出 (JSON/CSV)
- 自动保存成功密码

#### 智能破解引擎
- **490,000+ 密码字典攻击**
- 常用密码库 (Top 100)
- 数字组合 (0000-9999)
- 日期格式 (1990-2030)
- 键盘模式 (qwerty, asdfgh)
- 文件名关键词派生
- 自动重试机制

---

### 🎨 现代化界面

#### 5 种主题模式
- 🌞 **Light** - 锐利灰白对比
- 🌙 **Dark** - 深海军蓝基调
- 🎮 **Cyberpunk** - 赛博霓虹
- 🌅 **Twilight** - 暮色极光
- 📜 **Sepia** - 纸质护眼

#### 13 种强调色
天蓝 · 靛蓝 · 紫罗兰 · 品红 · 粉红 · 玫瑰红 · 橙色 · 琥珀 · 青柠 · 翡翠 · 青色 · 蓝绿 · 石板灰

#### Aero 毛玻璃效果
- 背景模糊 + 半透明叠加
- 动态阴影与边框光晕
- 流畅的页面切换动画
- 响应式布局 (桌面/平板/触屏)

---

### ♿ 无障碍访问 (WCAG 2.1 AA/AAA)

#### 可访问性功能
- ✅ **字体大小调节**: 标准 / 较大 / 超大 (16-20px)
- ✅ **高对比度模式**: 21:1 对比度 (AAA 级)
- ✅ **色盲模式**: 红色盲 / 绿色盲 / 蓝色盲
- ✅ **减少动画**: 前庭障碍友好
- ✅ **增强焦点指示器**: 键盘导航 3px 轮廓
- ✅ **跳过导航链接**: 快速访问主内容
- ✅ **屏幕阅读器优化**: ARIA 标签完整
- ✅ **触摸目标**: 44×44px 最小尺寸
- ✅ **打印友好**: 黑白配色 + URL 显示

#### WCAG 合规性

| 主题 | 对比度 | WCAG 等级 |
|------|--------|-----------|
| Light | 6.2:1 | AA+ |
| Dark | 5.8:1 | AA |
| Cyberpunk | 8.1:1 | **AAA** ⭐ |
| Twilight | 5.2:1 | AA |
| Sepia | 8.5:1 | **AAA** ⭐ |

---

### 🛡️ 安全与可靠

#### 安全特性
- ✅ Zip Slip 路径穿越防护
- ✅ 随机主密钥 (每安装独立)
- ✅ 密码日志脱敏
- ✅ 安全文件路径验证
- ✅ 非 UTF-8 路径兼容

#### 质量保证
- ✅ 54/54 单元测试通过
- ✅ 35 个集成测试通过
- ✅ 零 Clippy 错误
- ✅ TypeScript 类型安全
- ✅ 优雅错误处理

---

### ⚡ 高性能设计

#### 性能特性
- **并行处理**: 1-16 线程可配置
- **流式解压**: 内存占用低
- **缓冲池**: I/O 缓冲复用
- **任务队列**: 批量操作优化
- **取消支持**: 随时中断长任务

#### 技术栈
- **后端**: Rust + Tauri 1.5
- **前端**: Vue 3 + Pinia + Tailwind CSS
- **数据库**: SQLite (sqlx)
- **加密**: ring (AES-256-GCM + Argon2)
- **压缩库**: zip 0.6, sevenz-rust 0.5, unrar 0.5

---

## 📸 界面预览

### 解压视图
- 拖拽上传 + 批量处理
- 实时进度条 + 日志查看
- 自动密码重试
- 分卷压缩自动检测

### 压缩视图
- 磁吸成组功能
- 全局/独立配置
- 分卷大小设置
- RAR 格式支持 (需 WinRAR)

### 密码保险箱
- 卡片式布局
- 使用统计热图
- 快速搜索过滤
- 导入导出功能

### 文件完整性
- CRC32 / MD5 / SHA256 校验
- 批量计算
- 校验文件验证 (.sfv, .md5, .sha256)
- 导出校验报告

### 设置中心
- 外观个性化
- 性能调优
- 可访问性设置
- 格式支持查看

---

## 🚀 快速开始

### 系统要求

- **操作系统**: Windows 10+ / macOS 10.15+ / Linux (Debian/Ubuntu/Fedora)
- **内存**: 4GB+ 推荐
- **磁盘**: 200MB 安装空间

### 安装方式

#### Windows
```bash
# 下载 .msi 安装包
# 双击安装，支持自动更新
```

#### macOS
```bash
# 下载 .dmg 镜像
# 拖拽至应用程序文件夹
```

#### Linux
```bash
# Debian/Ubuntu
sudo dpkg -i long-decompress_0.1.0_amd64.deb

# Fedora/RHEL
sudo rpm -i long-decompress-0.1.0-1.x86_64.rpm

# AppImage (通用)
chmod +x long-decompress_0.1.0_amd64.AppImage
./long-decompress_0.1.0_amd64.AppImage
```

---

## 🛠️ 开发指南

### 环境准备

```bash
# 克隆仓库
git clone https://github.com/Longyuyeee/long_Decompress.git
cd long_Decompress/long-compress-assistant

# 安装依赖
npm install

# Rust 工具链 (1.75+)
rustup default stable
```

### 开发命令

```bash
# 开发模式 (热重载)
npm run tauri dev

# 构建生产版本
npm run tauri build

# 运行测试
cargo test                    # Rust 单元测试
npm run test:unit            # Vue 单元测试

# 代码检查
cargo clippy                 # Rust linter
npm run lint                 # Vue/TypeScript linter
```

### 项目结构

```
long-compress-assistant/
├── src/                    # Vue 前端
│   ├── views/             # 页面视图
│   ├── components/        # Vue 组件
│   ├── stores/            # Pinia 状态管理
│   ├── styles/            # 样式文件
│   └── i18n/              # 国际化
├── src-tauri/             # Rust 后端
│   ├── src/
│   │   ├── models/        # 数据模型
│   │   ├── services/      # 业务逻辑
│   │   └── utils/         # 工具函数
│   ├── Cargo.toml         # Rust 依赖
│   └── tauri.conf.json    # Tauri 配置
└── package.json           # Node 依赖
```

---

## 📝 更新日志

### v0.1.0 (2026-07-16) - 首个正式版本

#### 新增功能
- ✅ 37+ 解压格式支持
- ✅ 16 种压缩格式支持
- ✅ 智能密码破解引擎 (490k 字典)
- ✅ 分卷压缩自动检测
- ✅ 文件完整性校验器
- ✅ 密码保险箱 (AES-256-GCM)
- ✅ 5 种主题 + 13 种强调色
- ✅ 完整可访问性功能 (WCAG 2.1 AA/AAA)

#### 优化改进
- 🎨 界面清晰度全面提升 (+35-50% 字体大小)
- 🎨 所有主题对比度达到 WCAG AA 标准
- 🛡️ 安全修复: Zip Slip 防护 + 密码脱敏
- ⚡ 性能优化: I/O 缓冲池 + 并行处理
- 🐛 修复: 59 个 Bug 修复，86 个 Clippy 警告清理

#### 技术债务清理
- ♻️ 移除 PrimeVue (减少 55 个包)
- ♻️ 统一主题系统 (消除双系统冲突)
- ♻️ 代码去重 (~500 行重复代码移除)
- ♻️ 测试覆盖: 35 个集成测试 + 54 个单元测试

---

## 🤝 贡献指南

欢迎贡献代码、报告问题或提出建议！

### 提交 PR 流程

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### Commit 规范

```
feat: 新增功能
fix: 修复 Bug
docs: 文档更新
style: 代码格式调整
refactor: 重构代码
perf: 性能优化
test: 测试相关
chore: 构建/工具链更新
```

---

## 📄 开源协议

本项目采用 [MIT 协议](LICENSE) 开源。

---

## 🙏 致谢

### 依赖库

- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [Vue.js](https://vuejs.org/) - 渐进式前端框架
- [Tailwind CSS](https://tailwindcss.com/) - 实用优先的 CSS 框架
- [zip-rs](https://github.com/zip-rs/zip) - Rust ZIP 实现
- [sevenz-rust](https://github.com/dyz1990/sevenz-rust) - 7Z 格式支持
- [unrar](https://github.com/muja/unrar.rs) - RAR 解压库

### 特别感谢

- Claude Opus 4.8 - AI 开发助手
- Anthropic - 提供强大的 AI 能力
- 开源社区 - 无私的知识分享

---

## 📞 联系方式

- **项目主页**: https://github.com/Longyuyeee/long_Decompress
- **问题反馈**: https://github.com/Longyuyeee/long_Decompress/issues
- **作者**: 刘若晨 (Longyuyeee)

---

<div align="center">

**如果这个项目对你有帮助，请给个 ⭐ Star！**

Made with ❤️ by [Longyuyeee](https://github.com/Longyuyeee)

Co-Authored-By: Claude Opus 4.8 (1M context)

</div>
