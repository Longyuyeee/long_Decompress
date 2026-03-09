# ZIP格式解压完善设计文档 (BE-004)

## 概述
完善现有ZIP解压功能，提供更稳定、安全、用户友好的解压体验。

## 当前状态分析

### 现有实现 (`compression_service.rs`)
- 基础ZIP解压功能已实现
- 支持多格式解压（ZIP, tar.gz, tar, tar.bz2, tar.xz）
- 基础错误处理使用anyhow
- 缺少密码保护支持
- 缺少进度报告
- 测试覆盖不足

### 技术栈
- zip库版本：0.6
- 异步运行时：tokio
- 错误处理：anyhow + thiserror

## 改进目标

### 1. 密码保护支持
- 支持加密ZIP文件的解压
- 提供密码验证和错误提示
- 支持多种加密算法（如果库支持）

### 2. 错误处理完善
- 定义具体的错误类型
- 提供详细的错误信息
- 支持错误恢复和重试

### 3. 进度报告
- 实时解压进度反馈
- 文件数量和大小的统计
- 支持取消操作

### 4. 性能优化
- 流式处理大文件
- 内存使用优化
- 并发解压支持

### 5. 测试完善
- 单元测试覆盖所有功能
- 集成测试验证端到端流程
- 边界测试处理异常情况

## 详细设计

### 1. 错误类型定义
```rust
#[derive(Debug, Error)]
pub enum ZipExtractionError {
    #[error("ZIP文件不存在: {0}")]
    FileNotFound(String),

    #[error("ZIP文件损坏或格式无效: {0}")]
    InvalidFormat(String),

    #[error("需要密码: {0}")]
    PasswordRequired(String),

    #[error("密码错误: {0}")]
    InvalidPassword(String),

    #[error("解压进度: {0}/{1}")]
    Progress(usize, usize),

    #[error("磁盘空间不足: {0}")]
    DiskSpaceFull(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("操作被取消")]
    Cancelled,

    #[error("ZIP文件包含恶意路径: {0}")]
    MaliciousPath(String),
}
```

### 2. 密码支持实现
```rust
/// 支持密码的ZIP解压
async fn extract_zip_with_password(
    zip_path: &Path,
    output_dir: &Path,
    password: Option<&str>,
) -> Result<(), ZipExtractionError> {
    // 检查zip库版本是否支持密码
    // 如果不支持，考虑升级库或使用替代方案

    // 实现密码验证逻辑
    // 支持重试机制
}
```

### 3. 进度报告机制
```rust
pub struct ExtractionProgress {
    pub total_files: usize,
    pub extracted_files: usize,
    pub total_size: u64,
    pub extracted_size: u64,
    pub current_file: String,
    pub percentage: f32,
}

pub trait ProgressCallback: Send + Sync {
    fn on_progress(&self, progress: &ExtractionProgress);
    fn should_cancel(&self) -> bool;
}
```

### 4. 安全考虑
- 路径遍历攻击防护
- 符号链接处理
- 文件权限检查
- 文件大小限制

### 5. 性能优化
- 使用缓冲读写
- 限制并发解压数量
- 内存使用监控
- 大文件分块处理

## 实施步骤

### 阶段1：错误处理完善 (2小时)
1. 定义详细的错误类型
2. 更新现有错误处理逻辑
3. 添加错误恢复机制

### 阶段2：密码支持实现 (3小时)
1. 研究zip库0.6的密码支持
2. 实现密码验证逻辑
3. 添加密码错误处理

### 阶段3：进度报告机制 (2小时)
1. 设计进度回调接口
2. 实现进度跟踪
3. 添加取消支持

### 阶段4：性能优化 (2小时)
1. 实现流式处理
2. 添加内存使用限制
3. 优化文件IO

### 阶段5：测试完善 (1小时)
1. 编写单元测试
2. 编写集成测试
3. 测试边界情况

## 测试计划

### 单元测试
1. 正常ZIP文件解压
2. 加密ZIP文件解压（正确密码）
3. 加密ZIP文件解压（错误密码）
4. 损坏ZIP文件处理
5. 大文件解压性能
6. 进度报告准确性

### 集成测试
1. 端到端解压流程
2. 并发解压测试
3. 错误恢复测试
4. 内存使用测试

### 边界测试
1. 空ZIP文件
2. 超大ZIP文件
3. 包含特殊字符的文件名
4. 嵌套目录结构
5. 符号链接处理

## 依赖检查

### zip库0.6功能调查
需要检查：
1. 是否支持密码保护
2. 加密算法支持情况
3. 大文件处理能力
4. 内存使用情况

### 备选方案
如果zip库0.6不支持密码：
1. 升级到支持密码的版本
2. 使用其他ZIP处理库
3. 实现自定义解密逻辑

## 风险评估

### 技术风险
1. zip库功能限制
2. 密码支持兼容性
3. 性能瓶颈

### 缓解措施
1. 提前验证库功能
2. 提供降级方案
3. 性能测试和优化

## 成功标准

### 功能标准
1. 支持加密ZIP文件解压
2. 提供详细的错误信息
3. 实时进度报告
4. 支持取消操作

### 性能标准
1. 大文件解压内存使用稳定
2. 解压速度可接受
3. 错误恢复快速

### 质量标准
1. 测试覆盖率 > 90%
2. 代码文档完整
3. 错误处理完善

## 后续优化建议

### 短期优化
1. 添加压缩比计算
2. 支持更多压缩格式
3. 添加批量解压功能

### 长期优化
1. GPU加速解压
2. 云解压支持
3. 智能解压（自动分类文件）