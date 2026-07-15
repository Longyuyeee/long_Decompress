# CI/CD 配置说明

## GitHub Actions 工作流

本项目包含三个主要的 CI/CD 工作流：

### 1. CI 工作流 (`.github/workflows/ci.yml`)

**触发条件**：
- Push 到 `master` 或 `develop` 分支
- Pull Request 到 `master` 或 `develop` 分支

**执行任务**：
- **Rust 后端测试**：
  - 运行所有 Rust 单元测试
  - 运行 Clippy 静态分析（将警告视为错误）
  - 缓存 Cargo 依赖以加速构建

- **前端测试**：
  - TypeScript 类型检查
  - ESLint 代码检查
  - 构建验证

- **应用构建**：
  - 构建完整的 Tauri 应用
  - 上传 Windows 安装包作为构建产物（保留 7 天）

### 2. Release 工作流 (`.github/workflows/release.yml`)

**触发条件**：
- Push 带 `v*` 前缀的 Git tag（例如 `v1.0.0`）

**执行任务**：
- **创建 GitHub Release**：
  - 自动从 tag 中提取版本号
  - 生成 Release 页面和描述

- **多平台构建**：
  - **Windows**：MSI 和 NSIS 安装包
  - **Linux**：AppImage 和 Debian 包
  - **macOS**：Intel 和 Apple Silicon DMG

- **自动上传**：
  - 所有安装包自动上传到 GitHub Release

### 3. Code Quality 工作流 (`.github/workflows/code-quality.yml`)

**触发条件**：
- Push 或 Pull Request

**执行任务**：
- **Rust 代码质量**：
  - 格式检查 (`cargo fmt`)
  - Clippy 静态分析
  - 未使用依赖检查 (`cargo udeps`)

- **前端代码质量**：
  - ESLint 检查
  - TypeScript 类型检查
  - 死代码检查 (`ts-prune`)

- **安全审计**：
  - Rust 依赖安全审计 (`cargo audit`)
  - npm 依赖安全审计 (`npm audit`)

- **依赖审查**（仅 PR）：
  - 检查新增依赖的安全性和许可证

## 使用指南

### 发布新版本

1. **更新版本号**：
   ```bash
   # 更新 src-tauri/Cargo.toml 中的 version
   # 更新 src-tauri/tauri.conf.json 中的 version
   # 更新 package.json 中的 version
   ```

2. **创建 Git tag**：
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

3. **自动构建和发布**：
   - GitHub Actions 会自动构建所有平台的安装包
   - 自动创建 GitHub Release 并上传文件

### 本地测试 CI 流程

```bash
# 测试 Rust 代码质量
cd src-tauri
cargo fmt --check
cargo clippy -- -D warnings
cargo test

# 测试前端代码质量
npm run lint
npm run type-check
npm run build

# 测试完整构建
npm run tauri build
```

### 查看构建状态

在 GitHub 仓库页面：
- **Actions** 标签页可以查看所有工作流的运行状态
- **Releases** 标签页可以下载发布的安装包

## 配置要求

### GitHub Secrets

当前工作流不需要额外的 secrets（使用默认的 `GITHUB_TOKEN`）

### 分支保护规则（推荐）

建议在 GitHub 仓库设置中配置：
- 要求 PR 必须通过 CI 检查才能合并
- 要求至少 1 个代码审查批准
- 禁止直接 push 到 `master` 分支

## 工作流徽章

可以在 README.md 中添加以下徽章：

```markdown
![CI](https://github.com/Longyuyeee/long_Decompress/workflows/CI/badge.svg)
![Code Quality](https://github.com/Longyuyeee/long_Decompress/workflows/Code%20Quality/badge.svg)
```

## 故障排查

### 常见问题

1. **构建失败：Rust 编译错误**
   - 确保本地能通过 `cargo build --release`
   - 检查是否有未提交的 Cargo.lock 更改

2. **构建失败：npm 依赖问题**
   - 删除本地 node_modules 和 package-lock.json
   - 重新运行 `npm install`
   - 提交更新后的 package-lock.json

3. **Release 失败：文件路径不存在**
   - 检查 tauri.conf.json 中的 bundle 配置
   - 确认版本号格式正确

4. **权限错误**
   - 确保仓库设置中启用了 GitHub Actions
   - 检查 GITHUB_TOKEN 权限

---

**最后更新**: 2026-07-16
**维护者**: 项目开发团队
