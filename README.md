# 胧解压·方便助手 (Long Decompress)

一款功能强大、易于使用的压缩文件解压工具，专注于提供便捷、高效、安全的解压体验。

## 功能特点

### 🚀 核心功能
- **多格式支持**: ZIP, RAR, 7Z, TAR, GZ, BZ2, XZ, ISO等
- **批量解压**: 一次性处理多个压缩文件
- **密码管理**: 智能密码尝试和密码本管理
- **错误恢复**: 完善的错误处理和恢复机制
- **性能优化**: 多线程解压，快速稳定

### 🎯 易用性设计
- **拖放操作**: 简单拖放即可开始解压
- **智能默认**: 合理的默认设置，新手友好
- **清晰提示**: 易懂的错误提示和帮助信息
- **一键操作**: 常用功能快速访问
- **渐进式界面**: 基础功能简单，高级功能可发现

### 🔒 安全可靠
- **文件验证**: 解压前后完整性检查
- **安全解压**: 防范路径遍历等安全风险
- **隐私保护**: 本地处理，不上传用户数据
- **加密存储**: 密码等敏感信息加密存储

## 快速开始

### 安装方法

#### Windows用户
1. 下载最新版本的安装程序
2. 运行安装程序，按照向导完成安装
3. 从开始菜单或桌面快捷方式启动

#### 绿色版（免安装）
1. 下载绿色版压缩包
2. 解压到任意目录
3. 运行 `long_decompress.exe`

### 基本使用

1. **拖放解压**: 将压缩文件拖放到程序窗口
2. **右键解压**: 在文件资源管理器右键选择"用胧解压打开"
3. **批量处理**: 选择多个文件或文件夹进行批量解压

## 详细功能说明

### 支持的压缩格式

| 格式 | 支持情况 | 说明 |
|------|----------|------|
| ZIP | ✅ 完全支持 | 包括加密ZIP和ZIP64 |
| RAR | ✅ 完全支持 | RAR4/RAR5格式，需要unrar |
| 7Z | ✅ 完全支持 | 包括加密7z文件 |
| TAR | ✅ 完全支持 | 包括各种压缩变体 |
| GZ/BZ2/XZ | ✅ 完全支持 | 单个压缩文件 |
| ISO | ✅ 基本支持 | 光盘映像文件 |
| CAB/ARJ | ✅ 基本支持 | 老式压缩格式 |

### 批量解压功能

- **智能分类**: 自动按格式分类处理
- **并行解压**: 多线程同时处理多个文件
- **进度跟踪**: 实时显示总体进度和当前文件
- **错误处理**: 错误文件单独记录，不影响其他文件
- **结果报告**: 生成详细的解压报告

### 密码管理

- **密码本支持**: 支持TXT格式密码本文件
- **智能尝试**: 常用密码优先尝试
- **密码记忆**: 成功密码可选择性保存
- **加密存储**: 保存的密码加密存储
- **批量尝试**: 批量文件共用密码本尝试

### 高级功能

- **命令行接口**: 支持命令行操作，便于自动化
- **格式转换**: 支持格式间转换
- **文件修复**: 尝试修复损坏的压缩文件
- **完整性验证**: 解压前后文件完整性检查
- **日志系统**: 详细的操作日志和错误日志

## 配置说明

### 配置文件位置
- Windows: `%APPDATA%\胧解压\config.yaml`
- macOS: `~/Library/Application Support/胧解压/config.yaml`
- Linux: `~/.config/胧解压/config.yaml`

### 主要配置项

```yaml
# 解压设置
default_output_dir: null  # 默认解压目录
create_subfolder: true    # 创建子文件夹
overwrite_existing: rename # 冲突处理：skip/overwrite/rename

# 批量设置
batch_threads: 4          # 批量处理线程数
batch_output_pattern: batch_{date}_{time}

# 密码设置
remember_passwords: true  # 记住成功密码
password_retention_days: 30
max_password_attempts: 100

# 界面设置
theme: light              # light/dark/auto
language: zh_CN           # 界面语言
show_hidden_files: false
```

## 命令行使用

### 基本命令
```bash
# 解压单个文件
long_decompress extract file.zip

# 解压到指定目录
long_decompress extract file.zip -o ./output

# 批量解压目录
long_decompress batch ./compressed_files -o ./output

# 使用密码本尝试
long_decompress extract encrypted.zip -p passwords.txt

# 列出压缩包内容
long_decompress list archive.rar
```

### 命令行选项
```
通用选项:
  -h, --help            显示帮助信息
  -v, --version         显示版本信息
  -q, --quiet           安静模式，减少输出
  -V, --verbose         详细模式，更多输出

解压选项:
  -o, --output DIR      指定输出目录
  -p, --password FILE   指定密码本文件
  -f, --force           强制覆盖已存在文件
  -k, --keep            保留目录结构

批量选项:
  -t, --threads N       指定线程数（默认：4）
  -r, --recursive       递归处理子目录
  --skip-errors         跳过错误文件继续处理
```

## 开发指南

### 环境要求
- Python 3.8+
- 系统要求：Windows 7+/macOS 10.13+/Linux

### 开发环境设置
```bash
# 克隆仓库
git clone https://github.com/yourusername/long-decompress.git
cd long-decompress

# 创建虚拟环境
python -m venv venv

# 激活虚拟环境
# Windows:
venv\Scripts\activate
# Linux/macOS:
source venv/bin/activate

# 安装依赖
pip install -r requirements.txt

# 安装开发依赖
pip install -r requirements-dev.txt
```

### 项目结构
```
long_decompress/
├── main.py              # 应用入口
├── core/                # 核心逻辑
│   ├── extractor.py     # 解压器基类
│   ├── batch_processor.py
│   └── password_manager.py
├── engines/             # 解压引擎
│   ├── zip_engine.py
│   ├── rar_engine.py
│   └── ...
├── gui/                 # 图形界面
│   ├── main_window.py
│   └── widgets/
├── services/            # 服务层
│   ├── config_service.py
│   └── log_service.py
├── utils/               # 工具函数
└── tests/               # 测试代码
```

### 运行测试
```bash
# 运行所有测试
pytest

# 运行特定测试
pytest tests/test_zip_engine.py

# 带覆盖率报告
pytest --cov=long_decompress

# 性能测试
pytest tests/performance/ -v
```

## 常见问题

### Q: 为什么RAR文件解压需要额外组件？
A: RAR是专有格式，需要unrar可执行文件。程序会引导下载或使用系统已安装的unrar。

### Q: 如何添加自定义密码本？
A: 在设置界面添加密码本文件，或直接将TXT格式密码本文件放在程序目录下的`passwords`文件夹。

### Q: 解压大文件时内存占用高怎么办？
A: 程序支持流式处理，可在设置中启用"低内存模式"。

### Q: 如何报告bug或请求功能？
A: 请通过GitHub Issues提交问题或功能请求。

## 更新日志

### v1.0.0 (计划中)
- 基础ZIP/RAR解压功能
- 拖放操作支持
- 批量文件处理
- 密码本管理
- 多语言界面

### v1.1.0 (计划中)
- 更多格式支持（7z, tar系列）
- 格式转换功能
- 性能优化
- 命令行接口完善

## 贡献指南

我们欢迎各种形式的贡献：

1. **报告bug**: 在GitHub Issues提交详细的问题描述
2. **功能建议**: 描述你需要的功能和使用场景
3. **代码贡献**: Fork仓库，创建功能分支，提交Pull Request
4. **文档改进**: 帮助改进文档和翻译
5. **测试帮助**: 帮助测试不同系统和环境

### 开发规范
- 遵循PEP 8代码风格
- 添加适当的单元测试
- 更新相关文档
- 保持向后兼容性

## 许可证

本项目采用MIT许可证。详见 [LICENSE](LICENSE) 文件。

## 联系方式

- 项目主页: https://github.com/yourusername/long-decompress
- 问题反馈: https://github.com/yourusername/long-decompress/issues
- 邮箱: support@example.com

## 致谢

感谢以下开源项目的支持：
- [py7zr](https://github.com/miurahr/py7zr) - 7z格式支持
- [rarfile](https://github.com/markokr/rarfile) - RAR格式支持
- [patool](https://github.com/wummel/patool) - 多格式统一接口
- [PyQt5](https://www.riverbankcomputing.com/software/pyqt/) - GUI框架

---

**胧解压·方便助手** - 让解压变得更简单！