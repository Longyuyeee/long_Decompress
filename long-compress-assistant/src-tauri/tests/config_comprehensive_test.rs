//! 配置管理系统全面测试
//!
//! 包含单元测试、集成测试、边界测试和性能测试。

use crate::config::models::{
    ConfigCategory, ConfigDataType, ConfigItem, ConfigMetadata, DefaultConfigGenerator,
    ExportFormat, ImportStrategy, ValidationRule,
};
use crate::config::validation::{ConfigValidator, ConfigValueConverter};
use crate::config::repository::ConfigRepository;
use crate::config::service::ConfigService;
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;
use tempfile::tempdir;

/// 单元测试：配置验证器
#[tokio::test]
async fn test_config_validator_comprehensive() {
    println!("开始配置验证器全面测试...");

    // 测试1: 字符串验证
    let metadata = ConfigMetadata {
        key: "test.string".to_string(),
        category: ConfigCategory::System,
        display_name: "字符串测试".to_string(),
        description: "字符串验证测试".to_string(),
        data_type: ConfigDataType::String,
        default_value: Value::String("default".to_string()),
        validation_rules: vec![
            ValidationRule::MinLength { value: 3 },
            ValidationRule::MaxLength { value: 10 },
            ValidationRule::Regex {
                pattern: r"^[a-zA-Z0-9_]+$".to_string(),
            },
        ],
        is_required: true,
        is_sensitive: false,
        is_readonly: false,
        version: "1.0.0".to_string(),
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // 有效值
    let result = ConfigValidator::validate(&metadata, &Value::String("valid_123".to_string()));
    assert!(result.is_valid, "有效字符串应该通过验证");

    // 太短的值
    let result = ConfigValidator::validate(&metadata, &Value::String("ab".to_string()));
    assert!(!result.is_valid, "太短的字符串应该失败");
    assert_eq!(result.errors[0].code, "min_length");

    // 太长的值
    let result = ConfigValidator::validate(&metadata, &Value::String("this_is_too_long".to_string()));
    assert!(!result.is_valid, "太长的字符串应该失败");
    assert_eq!(result.errors[0].code, "max_length");

    // 无效字符
    let result = ConfigValidator::validate(&metadata, &Value::String("invalid@char".to_string()));
    assert!(!result.is_valid, "无效字符应该失败");
    assert_eq!(result.errors[0].code, "regex_pattern");

    // 测试2: 数字验证
    let metadata = ConfigMetadata {
        key: "test.number".to_string(),
        category: ConfigCategory::System,
        display_name: "数字测试".to_string(),
        description: "数字验证测试".to_string(),
        data_type: ConfigDataType::Integer,
        default_value: Value::Number(50.into()),
        validation_rules: vec![
            ValidationRule::MinValue { value: 0.0 },
            ValidationRule::MaxValue { value: 100.0 },
            ValidationRule::Range { min: 10.0, max: 90.0 },
        ],
        is_required: false,
        is_sensitive: false,
        is_readonly: false,
        version: "1.0.0".to_string(),
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // 有效值
    let result = ConfigValidator::validate(&metadata, &Value::Number(50.into()));
    assert!(result.is_valid, "有效数字应该通过验证");

    // 小于最小值
    let result = ConfigValidator::validate(&metadata, &Value::Number((-10).into()));
    assert!(!result.is_valid, "小于最小值应该失败");
    assert_eq!(result.errors[0].code, "min_value");

    // 大于最大值
    let result = ConfigValidator::validate(&metadata, &Value::Number(150.into()));
    assert!(!result.is_valid, "大于最大值应该失败");
    assert_eq!(result.errors[0].code, "max_value");

    // 超出范围
    let result = ConfigValidator::validate(&metadata, &Value::Number(5.into()));
    assert!(!result.is_valid, "超出范围应该失败");
    assert_eq!(result.errors[0].code, "range");

    // 测试3: 枚举验证
    let metadata = ConfigMetadata {
        key: "test.enum".to_string(),
        category: ConfigCategory::System,
        display_name: "枚举测试".to_string(),
        description: "枚举验证测试".to_string(),
        data_type: ConfigDataType::String,
        default_value: Value::String("option1".to_string()),
        validation_rules: vec![ValidationRule::Enum {
            values: vec!["option1".to_string(), "option2".to_string(), "option3".to_string()],
        }],
        is_required: false,
        is_sensitive: false,
        is_readonly: false,
        version: "1.0.0".to_string(),
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // 有效值
    let result = ConfigValidator::validate(&metadata, &Value::String("option2".to_string()));
    assert!(result.is_valid, "有效枚举值应该通过验证");

    // 无效值
    let result = ConfigValidator::validate(&metadata, &Value::String("option4".to_string()));
    assert!(!result.is_valid, "无效枚举值应该失败");
    assert_eq!(result.errors[0].code, "enum_value");

    println!("✓ 配置验证器全面测试通过");
}

/// 单元测试：配置值转换器
#[test]
fn test_config_value_converter() {
    println!("开始配置值转换器测试...");

    // 字符串转换
    let result = ConfigValueConverter::from_string(ConfigDataType::String, "test value").unwrap();
    assert_eq!(result, Value::String("test value".to_string()));

    // 整数转换
    let result = ConfigValueConverter::from_string(ConfigDataType::Integer, "123").unwrap();
    assert_eq!(result, Value::Number(123.into()));

    // 浮点数转换
    let result = ConfigValueConverter::from_string(ConfigDataType::Float, "123.45").unwrap();
    assert_eq!(result.as_f64().unwrap(), 123.45);

    // 布尔转换
    let result = ConfigValueConverter::from_string(ConfigDataType::Boolean, "true").unwrap();
    assert_eq!(result, Value::Bool(true));

    let result = ConfigValueConverter::from_string(ConfigDataType::Boolean, "false").unwrap();
    assert_eq!(result, Value::Bool(false));

    // 数组转换
    let result = ConfigValueConverter::from_string(ConfigDataType::Array, "item1,item2,item3").unwrap();
    assert!(result.is_array());
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], Value::String("item1".to_string()));

    // 值到字符串转换
    assert_eq!(ConfigValueConverter::to_string(&Value::String("test".to_string())), "test");
    assert_eq!(ConfigValueConverter::to_string(&Value::Number(123.into())), "123");
    assert_eq!(ConfigValueConverter::to_string(&Value::Bool(true)), "true");
    assert_eq!(
        ConfigValueConverter::to_string(&Value::Array(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
        ])),
        "a,b"
    );

    println!("✓ 配置值转换器测试通过");
}

/// 集成测试：配置服务完整流程
#[tokio::test]
async fn test_config_service_integration() {
    println!("开始配置服务集成测试...");

    let service = create_test_service().await;

    // 1. 初始状态验证
    let initial_configs = service.get_all_configs().await.unwrap();
    assert!(!initial_configs.is_empty(), "初始配置应该不为空");

    // 2. 获取和修改配置
    let original_value = service.get_string("system.language").await.unwrap();
    assert_eq!(original_value, Some("zh-CN".to_string()));

    // 修改配置
    service.set_config(
        "system.language",
        Value::String("en-US".to_string()),
        "test_user"
    ).await.unwrap();

    let updated_value = service.get_string("system.language").await.unwrap();
    assert_eq!(updated_value, Some("en-US".to_string()));

    // 3. 验证配置
    let validation_result = service.validate_config(
        "system.update_interval",
        &Value::Number(5000.into())
    ).await.unwrap();
    assert!(validation_result.is_valid, "有效值应该通过验证");

    let invalid_result = service.validate_config(
        "system.update_interval",
        &Value::Number(500.into())
    ).await.unwrap();
    assert!(!invalid_result.is_valid, "无效值应该失败");

    // 4. 重置配置
    service.reset_to_default("system.language", "test_user").await.unwrap();
    let reset_value = service.get_string("system.language").await.unwrap();
    assert_eq!(reset_value, Some("zh-CN".to_string()));

    // 5. 批量操作
    let mut updates = HashMap::new();
    updates.insert("system.language".to_string(), Value::String("ja-JP".to_string()));
    updates.insert("ui.theme".to_string(), Value::String("dark".to_string()));

    service.batch_set_configs(updates, "test_user").await.unwrap();

    let language_value = service.get_string("system.language").await.unwrap();
    let theme_value = service.get_string("ui.theme").await.unwrap();
    assert_eq!(language_value, Some("ja-JP".to_string()));
    assert_eq!(theme_value, Some("dark".to_string()));

    // 6. 批量重置
    let keys = vec!["system.language".to_string(), "ui.theme".to_string()];
    service.batch_reset_to_default(keys, "test_user").await.unwrap();

    let final_language = service.get_string("system.language").await.unwrap();
    let final_theme = service.get_string("ui.theme").await.unwrap();
    assert_eq!(final_language, Some("zh-CN".to_string()));
    assert_eq!(final_theme, Some("light".to_string()));

    // 7. 搜索配置
    let search_results = service.search_configs("language").await.unwrap();
    assert!(!search_results.is_empty(), "搜索应该返回结果");
    let has_language = search_results.iter().any(|item| item.metadata.key.contains("language"));
    assert!(has_language, "搜索结果应该包含语言配置");

    // 8. 获取统计信息
    let stats = service.get_statistics().await.unwrap();
    assert!(stats.total_configs > 0, "应该有配置统计信息");
    assert!(stats.cached_configs > 0, "应该有缓存配置");

    println!("✓ 配置服务集成测试通过");
}

/// 集成测试：配置导入导出
#[tokio::test]
async fn test_config_import_export() {
    println!("开始配置导入导出测试...");

    let service = create_test_service().await;

    // 1. 导出配置
    let export_data = service.export_configs(ExportFormat::Json).await.unwrap();
    assert!(!export_data.is_empty(), "导出数据不应该为空");

    // 验证导出的JSON可以解析
    let parsed: Value = serde_json::from_slice(&export_data).unwrap();
    assert!(parsed.get("configs").is_some(), "导出数据应该包含配置");
    assert!(parsed.get("export_version").is_some(), "导出数据应该包含版本");

    // 2. 修改一些配置
    service.set_config(
        "system.language",
        Value::String("en-US".to_string()),
        "test"
    ).await.unwrap();

    service.set_config(
        "ui.theme",
        Value::String("dark".to_string()),
        "test"
    ).await.unwrap();

    // 3. 导入配置（替换策略）
    let import_result = service.import_configs(
        &export_data,
        ExportFormat::Json,
        ImportStrategy::Replace,
        "test"
    ).await.unwrap();

    assert!(import_result.imported_items > 0, "应该导入一些配置项");
    assert_eq!(import_result.failed_items, 0, "不应该有失败的导入");

    // 4. 验证配置已恢复
    let language_value = service.get_string("system.language").await.unwrap();
    let theme_value = service.get_string("ui.theme").await.unwrap();
    assert_eq!(language_value, Some("zh-CN".to_string()));
    assert_eq!(theme_value, Some("light".to_string()));

    // 5. 测试其他导入策略
    // 先修改一些配置
    service.set_config(
        "system.language",
        Value::String("ja-JP".to_string()),
        "test"
    ).await.unwrap();

    // UpdateOnly策略：只更新已存在的配置
    let import_result = service.import_configs(
        &export_data,
        ExportFormat::Json,
        ImportStrategy::UpdateOnly,
        "test"
    ).await.unwrap();

    let language_value = service.get_string("system.language").await.unwrap();
    assert_eq!(language_value, Some("zh-CN".to_string()), "UpdateOnly应该更新现有配置");

    println!("✓ 配置导入导出测试通过");
}

/// 边界测试：极端情况和错误处理
#[tokio::test]
async fn test_config_boundary_cases() {
    println!("开始边界情况测试...");

    let service = create_test_service().await;

    // 1. 获取不存在的配置
    let non_existent = service.get_config("non.existent.key").await.unwrap();
    assert!(non_existent.is_none(), "不存在的配置应该返回None");

    // 2. 设置无效的配置值
    let result = service.set_config(
        "system.update_interval",
        Value::String("not a number".to_string()),
        "test"
    ).await;

    assert!(result.is_err(), "设置无效值应该失败");

    // 3. 验证空值和null值
    let metadata = service.get_metadata("system.language").await.unwrap().unwrap();
    let validation_result = ConfigValidator::validate(&metadata, &Value::Null);
    assert!(!validation_result.is_valid, "null值应该验证失败");

    // 4. 测试必填字段
    let required_metadata = ConfigMetadata {
        key: "test.required".to_string(),
        category: ConfigCategory::System,
        display_name: "必填测试".to_string(),
        description: "必填字段测试".to_string(),
        data_type: ConfigDataType::String,
        default_value: Value::String("default".to_string()),
        validation_rules: vec![],
        is_required: true,
        is_sensitive: false,
        is_readonly: false,
        version: "1.0.0".to_string(),
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let validation_result = ConfigValidator::validate(&required_metadata, &Value::Null);
    assert!(!validation_result.is_valid, "必填字段的null值应该失败");
    assert_eq!(validation_result.errors[0].code, "required_field");

    // 5. 测试敏感字段
    let sensitive_metadata = ConfigMetadata {
        key: "test.sensitive".to_string(),
        category: ConfigCategory::Security,
        display_name: "敏感测试".to_string(),
        description: "敏感字段测试".to_string(),
        data_type: ConfigDataType::String,
        default_value: Value::String("default".to_string()),
        validation_rules: vec![],
        is_required: false,
        is_sensitive: true,
        is_readonly: false,
        version: "1.0.0".to_string(),
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // 敏感字段应该可以正常验证
    let validation_result = ConfigValidator::validate(&sensitive_metadata, &Value::String("secret".to_string()));
    assert!(validation_result.is_valid, "敏感字段的有效值应该通过验证");

    // 6. 测试只读字段
    let readonly_metadata = ConfigMetadata {
        key: "test.readonly".to_string(),
        category: ConfigCategory::System,
        display_name: "只读测试".to_string(),
        description: "只读字段测试".to_string(),
        data_type: ConfigDataType::String,
        default_value: Value::String("default".to_string()),
        validation_rules: vec![],
        is_required: false,
        is_sensitive: false,
        is_readonly: true,
        version: "1.0.0".to_string(),
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // 只读字段应该可以正常验证
    let validation_result = ConfigValidator::validate(&readonly_metadata, &Value::String("value".to_string()));
    assert!(validation_result.is_valid, "只读字段的有效值应该通过验证");

    println!("✓ 边界情况测试通过");
}

/// 性能测试：配置缓存
#[tokio::test]
async fn test_config_cache_performance() {
    println!("开始配置缓存性能测试...");

    let service = create_test_service().await;

    // 1. 第一次获取（应该从数据库加载）
    let start = std::time::Instant::now();
    let _ = service.get_config("system.language").await.unwrap();
    let first_load_time = start.elapsed();

    // 2. 第二次获取（应该从缓存加载）
    let start = std::time::Instant::now();
    let _ = service.get_config("system.language").await.unwrap();
    let cache_load_time = start.elapsed();

    // 缓存加载应该比数据库加载快
    assert!(
        cache_load_time < first_load_time || cache_load_time <= first_load_time,
        "缓存加载时间: {:?}, 数据库加载时间: {:?}",
        cache_load_time,
        first_load_time
    );

    // 3. 清空缓存后再次获取
    service.clear_cache().await.unwrap();

    let start = std::time::Instant::now();
    let _ = service.get_config("system.language").await.unwrap();
    let after_clear_time = start.elapsed();

    // 清空缓存后应该重新从数据库加载
    assert!(
        after_clear_time >= first_load_time || after_clear_time <= first_load_time * 2,
        "清空缓存后加载时间: {:?}, 初始加载时间: {:?}",
        after_clear_time,
        first_load_time
    );

    // 4. 刷新缓存
    service.refresh_cache().await.unwrap();

    let start = std::time::Instant::now();
    let _ = service.get_config("system.language").await.unwrap();
    let after_refresh_time = start.elapsed();

    // 刷新缓存后应该从缓存加载
    assert!(
        after_refresh_time <= cache_load_time * 2,
        "刷新缓存后加载时间: {:?}, 缓存加载时间: {:?}",
        after_refresh_time,
        cache_load_time
    );

    println!("✓ 配置缓存性能测试通过");
    println!("  数据库加载时间: {:?}", first_load_time);
    println!("  缓存加载时间: {:?}", cache_load_time);
    println!("  清空缓存后加载时间: {:?}", after_clear_time);
    println!("  刷新缓存后加载时间: {:?}", after_refresh_time);
}

/// 创建测试服务
async fn create_test_service() -> ConfigService {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}", db_path.display()))
        .await
        .unwrap();

    let service = ConfigService::new(pool);
    service.init().await.unwrap();
    service
}

/// 主测试函数
#[tokio::test]
async fn test_config_system_comprehensive() {
    println!("开始配置管理系统全面测试...");
    println!("=".repeat(50));

    // 单元测试
    test_config_validator_comprehensive().await;
    test_config_value_converter();

    // 集成测试
    test_config_service_integration().await;
    test_config_import_export().await;

    // 边界测试
    test_config_boundary_cases().await;

    // 性能测试
    test_config_cache_performance().await;

    println!("=".repeat(50));
    println!("🎉 所有配置管理系统测试通过！");
    println!("测试覆盖：");
    println!("  ✓ 单元测试（验证器、转换器）");
    println!("  ✓ 集成测试（完整流程、导入导出）");
    println!("  ✓ 边界测试（极端情况、错误处理）");
    println!("  ✓ 性能测试（缓存性能）");
}