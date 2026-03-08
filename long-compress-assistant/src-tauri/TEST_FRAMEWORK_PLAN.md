# Rust后端测试框架规划

## 概述

本文档规划"胧压缩·方便助手"项目Rust后端的完整测试框架，确保核心功能模块有充分的测试覆盖。

## 当前测试状态

### 已有测试文件
1. **集成测试**:
   - `tests/integration/commands_test.rs` - 命令集成测试
   - `tests/config_integration_test.rs` - 配置集成测试
   - `tests/config_comprehensive_test.rs` - 配置综合测试

2. **单元测试**:
   - `tests/unit/compression_service_test.rs` - 压缩服务测试
   - `tests/unit/file_service_test.rs` - 文件服务测试
   - `tests/unit/file_service_impl_test.rs` - 文件服务实现测试
   - `tests/unit/file_models_test.rs` - 文件模型测试
   - `tests/unit/models_test.rs` - 模型测试

3. **性能测试**:
   - `tests/benchmarks/` - 性能基准测试目录

## 测试框架目标

### 测试覆盖率目标
- **单元测试**: 80%+ 代码覆盖率
- **集成测试**: 主要工作流100%覆盖
- **性能测试**: 所有核心操作有基准测试
- **安全测试**: 加密和安全模块100%测试

### 核心功能模块测试优先级

#### 高优先级 (P0)
1. **文件服务模块** (`services/file_service.rs`)
2. **压缩解压模块** (`commands/compression.rs`)
3. **密码管理模块** (`commands/password.rs`, `crypto/`)
4. **配置管理模块** (`config/`)

#### 中优先级 (P1)
1. **任务队列模块** (`task_queue/`)
2. **系统集成模块** (`system_integration/`)
3. **数据库模块** (`database/`)
4. **工具函数模块** (`utils/`)

#### 低优先级 (P2)
1. **模型验证测试**
2. **边界条件测试**
3. **错误恢复测试**

## 测试目录结构规划

```
src-tauri/tests/
├── README.md                    # 测试框架文档
├── lib.rs                       # 测试库入口
├── integration/                 # 集成测试
│   ├── commands/               # 命令集成测试
│   │   ├── compression_test.rs
│   │   ├── file_test.rs
│   │   ├── password_test.rs
│   │   └── system_test.rs
│   ├── config/                 # 配置集成测试
│   │   ├── service_test.rs
│   │   ├── repository_test.rs
│   │   └── validation_test.rs
│   ├── crypto/                 # 加密集成测试
│   │   ├── encryption_test.rs
│   │   └── hashing_test.rs
│   └── services/               # 服务集成测试
│       ├── file_service_test.rs
│       └── compression_service_test.rs
├── unit/                       # 单元测试
│   ├── commands/               # 命令单元测试
│   ├── config/                 # 配置单元测试
│   ├── crypto/                 # 加密单元测试
│   ├── database/               # 数据库单元测试
│   ├── models/                 # 模型单元测试
│   ├── services/               # 服务单元测试
│   ├── system_integration/     # 系统集成单元测试
│   ├── task_queue/             # 任务队列单元测试
│   └── utils/                  # 工具函数单元测试
├── benchmarks/                 # 性能基准测试
│   ├── compression_bench.rs
│   ├── encryption_bench.rs
│   ├── file_io_bench.rs
│   └── memory_bench.rs
├── e2e/                        # 端到端测试（与前端结合）
│   ├── file_upload_test.rs
│   ├── compression_workflow_test.rs
│   └── password_management_test.rs
├── fixtures/                   # 测试夹具
│   ├── test_data.rs           # 测试数据生成
│   ├── mock_services.rs       # 模拟服务
│   └── test_helpers.rs        # 测试辅助函数
└── utils/                      # 测试工具
    ├── test_logger.rs         # 测试日志
    ├── temp_file_manager.rs   # 临时文件管理
    └── test_database.rs       # 测试数据库
```

## 测试类型定义

### 1. 单元测试 (Unit Tests)
- **位置**: `tests/unit/`
- **范围**: 单个函数、结构体、trait实现
- **工具**: `cargo test`, `mockall`
- **目标**: 验证代码单元的正确性

### 2. 集成测试 (Integration Tests)
- **位置**: `tests/integration/`
- **范围**: 模块间交互、API端点
- **工具**: `cargo test`, `tempfile`, `assert_fs`
- **目标**: 验证系统组件协同工作

### 3. 性能测试 (Performance Tests)
- **位置**: `tests/benchmarks/`
- **范围**: 性能基准、负载测试
- **工具**: `criterion`, `benchmarking`
- **目标**: 确保性能达标

### 4. 端到端测试 (E2E Tests)
- **位置**: `tests/e2e/`
- **范围**: 完整工作流，与前端结合
- **工具**: `tauri/test`, 自定义测试框架
- **目标**: 验证真实用户场景

## 测试标准

### 单元测试标准
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[test]
    fn test_function_name() {
        // Arrange - 准备测试数据
        let input = "test";

        // Act - 执行测试
        let result = function_under_test(input);

        // Assert - 验证结果
        assert_eq!(result, expected_value);
        assert!(condition);
    }
}
```

### 集成测试标准
```rust
#[test]
fn test_integration_workflow() {
    // 设置测试环境
    let temp_dir = tempfile::tempdir().unwrap();

    // 执行集成操作
    let result = integrated_operation(&temp_dir.path());

    // 验证完整工作流
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected_output);
}
```

### 性能测试标准
```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn compression_benchmark(c: &mut Criterion) {
    c.bench_function("compress_10mb", |b| {
        b.iter(|| {
            compress_large_file();
        });
    });
}

criterion_group!(benches, compression_benchmark);
criterion_main!(benches);
```

## 测试数据管理

### 测试文件生成
```rust
// 生成测试文件
fn create_test_file(size: usize) -> PathBuf {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let data = vec![0u8; size];
    std::fs::write(temp_file.path(), &data).unwrap();
    temp_file.into_temp_path().to_path_buf()
}
```

### 测试数据库
```rust
// 使用内存数据库进行测试
fn setup_test_database() -> SqliteConnection {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    run_migrations(&conn).unwrap();
    conn
}
```

### 模拟服务
```rust
// 使用mockall模拟外部依赖
mock! {
    pub FileSystem {
        fn read_file(&self, path: &str) -> Result<Vec<u8>, Error>;
        fn write_file(&self, path: &str, data: &[u8]) -> Result<(), Error>;
    }
}
```

## 测试执行流程

### 本地开发测试
```bash
# 运行所有测试
cargo test

# 运行单元测试
cargo test --lib

# 运行集成测试
cargo test --test "*"

# 运行性能基准测试
cargo bench

# 运行特定测试
cargo test test_name

# 运行测试并显示详细输出
cargo test -- --nocapture
```

### CI/CD测试流程
1. **代码检查**: `cargo fmt --check`, `cargo clippy`
2. **单元测试**: `cargo test --lib`
3. **集成测试**: `cargo test --test "*"`
4. **性能测试**: `cargo bench`
5. **安全检查**: `cargo audit`
6. **覆盖率检查**: `cargo tarpaulin` 或 `grcov`

## 测试覆盖率监控

### 覆盖率工具
- **tarpaulin**: Rust代码覆盖率工具
- **grcov**: 生成覆盖率报告
- **codecov**: 在线覆盖率平台

### 覆盖率目标
- **总体覆盖率**: 80%+
- **核心模块**: 90%+
- **安全模块**: 100%
- **新代码**: 必须达到覆盖率标准

## 测试质量保证

### 测试编写要求
1. **AAA模式**: Arrange-Act-Assert
2. **描述性名称**: 测试名称应描述预期行为
3. **独立性**: 测试间无依赖
4. **确定性**: 测试结果一致
5. **边界测试**: 测试边界条件和错误情况

### 测试维护要求
1. **代码变更同步**: 代码重构时更新测试
2. **定期审查**: 定期审查测试质量和覆盖率
3. **性能监控**: 监控测试执行时间
4. **文档更新**: 保持测试文档最新

## 实施计划

### 第一阶段 (当前)
1. 完善现有测试目录结构
2. 为文件服务模块添加完整测试
3. 为压缩解压模块添加基础测试
4. 建立测试覆盖率监控

### 第二阶段
1. 为密码管理模块添加完整测试
2. 为配置管理模块添加测试
3. 建立性能测试框架
4. 完善集成测试

### 第三阶段
1. 为所有模块添加完整测试
2. 建立端到端测试框架
3. 优化测试执行性能
4. 建立自动化测试报告

## 依赖工具

### 测试框架
```toml
[dev-dependencies]
mockall = "0.12"
tempfile = "3.10"
assert_fs = "1.0"
predicates = "3.0"
criterion = "0.5"
proptest = "1.4"
test-log = "0.2"
```

### 覆盖率工具
```toml
[dev-dependencies]
tarpaulin = "0.26"
grcov = "0.8"
```

### 代码质量
```toml
[dev-dependencies]
clippy = "0.1"
rustfmt = "0.1"
```

## 结论

本测试框架规划为Rust后端提供了完整的测试基础设施，确保代码质量、功能正确性和性能表现。通过分阶段实施，可以逐步建立完善的测试体系，为项目的长期稳定发展提供保障。