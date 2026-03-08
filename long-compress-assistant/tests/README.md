# 胧压缩·方便助手 - 测试框架

## 概述

本目录包含"胧压缩·方便助手"项目的完整测试套件，遵循测试金字塔原则，确保代码质量、功能正确性和性能表现。

## 目录结构

```
tests/
├── README.md                    # 本文件
├── setup.ts                     # 测试全局配置
├── integration/                 # 集成测试
│   ├── compression_integration.test.ts
│   ├── file_management.test.ts
│   └── password_integration.test.ts
├── e2e/                         # 端到端测试
│   ├── specs/                   # 测试规范
│   │   ├── file_upload.spec.ts
│   │   ├── compression_workflow.spec.ts
│   │   └── settings_page.spec.ts
│   ├── fixtures/                # 测试数据
│   └── pages/                   # 页面对象模型
├── performance/                 # 性能测试
│   ├── compression_performance.test.ts
│   ├── memory_usage.test.ts
│   └── benchmarks/              # 基准测试数据
├── data/                        # 测试数据文件
│   ├── test_files/              # 测试用文件
│   │   ├── small.txt
│   │   ├── medium.pdf
│   │   └── large.zip
│   └── passwords.txt            # 测试密码本
└── fixtures/                    # 测试夹具
    ├── mock_data.ts
    └── test_helpers.ts
```

## 测试类型

### 1. 单元测试 (Unit Tests)
- **位置**: `src/**/__tests__/`
- **范围**: 单个函数、类、组件
- **工具**: Vitest, @vue/test-utils
- **目标**: 70%+ 代码覆盖率

### 2. 集成测试 (Integration Tests)
- **位置**: `tests/integration/`
- **范围**: 模块间交互、API调用
- **工具**: Vitest, tauri/test
- **目标**: 验证系统组件协同工作

### 3. 端到端测试 (E2E Tests)
- **位置**: `tests/e2e/`
- **范围**: 完整用户流程
- **工具**: Playwright
- **目标**: 验证真实用户场景

### 4. 性能测试 (Performance Tests)
- **位置**: `tests/performance/`
- **范围**: 性能基准、负载测试
- **工具**: 自定义脚本, 性能监控
- **目标**: 确保性能达标

## 测试数据管理

### 测试文件生成
```bash
# 生成测试数据
./scripts/generate_test_files.sh

# 测试文件类型
- 文本文件 (.txt, .md, .json)
- 压缩文件 (.zip, .tar.gz, .7z)
- 二进制文件 (.bin, .dat)
- 大文件 (10MB, 100MB, 1GB)
```

### 测试数据库
- 使用SQLite内存数据库
- 每个测试用例独立数据库
- 测试后自动清理

### 临时文件
- 使用`tempfile` crate创建临时文件
- 测试后自动删除
- 避免文件系统污染

## 运行测试

### 前端测试
```bash
# 运行所有测试
npm test

# 运行单元测试
npm run test:unit

# 运行单元测试（监视模式）
npm run test:unit:watch

# 运行单元测试并生成覆盖率报告
npm run test:unit:coverage

# 运行E2E测试
npm run test:e2e

# 运行E2E测试UI
npm run test:e2e:ui

# 查看E2E测试报告
npm run test:e2e:report
```

### Rust后端测试
```bash
# 进入Tauri目录
cd src-tauri

# 运行所有测试
cargo test

# 运行特定测试
cargo test test_name

# 运行性能基准测试
cargo bench

# 运行测试并显示详细输出
cargo test -- --nocapture
```

## 测试标准

### 单元测试标准
- 每个测试一个断言（尽量）
- 使用描述性的测试名称
- 测试边界条件和错误情况
- 避免测试间依赖

### 集成测试标准
- 测试真实的数据流
- 验证模块间接口
- 模拟外部依赖
- 测试错误恢复

### E2E测试标准
- 测试完整用户流程
- 验证UI交互
- 测试跨页面导航
- 验证错误处理

### 性能测试标准
| 操作类型 | 文件大小 | 最大时间 | 最大内存 |
|---------|---------|---------|---------|
| 压缩 | 10MB | 3秒 | 50MB |
| 解压 | 10MB | 2秒 | 30MB |
| 批量处理 | 100MB | 15秒 | 200MB |

## 最佳实践

### 测试编写
1. **AAA模式**: Arrange-Act-Assert
2. **描述性名称**: 测试应该描述预期行为
3. **独立性**: 测试不应依赖其他测试
4. **确定性**: 测试结果应该一致

### 测试维护
1. **定期更新**: 测试数据应定期更新
2. **重构同步**: 代码重构时同步更新测试
3. **删除过时**: 删除不再相关的测试
4. **保持快速**: 测试套件应快速运行

### 团队协作
1. **代码审查**: PR必须包含相关测试
2. **知识共享**: 分享测试技巧和模式
3. **持续改进**: 定期回顾测试策略
4. **文档更新**: 保持测试文档最新

## 故障排除

### 常见问题
1. **测试失败**: 检查测试环境配置
2. **覆盖率低**: 检查未覆盖的代码路径
3. **性能问题**: 使用性能分析工具
4. **环境差异**: 确保测试环境一致

### 调试技巧
1. **详细日志**: 使用`--verbose`标志
2. **单步调试**: 使用调试器逐步执行
3. **隔离测试**: 运行单个失败测试
4. **环境检查**: 验证测试环境配置

## 贡献指南

### 添加新测试
1. 确定测试类型（单元/集成/E2E/性能）
2. 选择适当的测试目录
3. 遵循现有测试模式
4. 添加必要的测试数据
5. 确保测试通过CI/CD

### 修改现有测试
1. 理解测试目的
2. 保持向后兼容
3. 更新相关文档
4. 验证所有测试通过

## 相关文档

- [TESTING_GUIDE.md](../TESTING_GUIDE.md) - 详细测试指南
- [Vitest文档](https://vitest.dev/)
- [Playwright文档](https://playwright.dev/)
- [Rust测试指南](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Vue测试指南](https://vuejs.org/guide/scaling-up/testing.html)

---
*最后更新: 2026-03-08*
*版本: 1.0.0*