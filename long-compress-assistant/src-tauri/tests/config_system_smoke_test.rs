//! 配置管理系统冒烟测试
//!
//! 验证配置管理系统的核心功能是否正常工作。

use long_compress_assistant::config::models::{ConfigCategory, ConfigDataType, ConfigItem, ConfigMetadata, DefaultConfigGenerator};
use long_compress_assistant::config::validation::ConfigValidator;
use chrono::Utc;
use serde_json::{json, Value};

#[test]
fn test_config_models_basic() {
    println!("开始配置模型基础测试...");

    // 测试配置元数据创建
    let metadata = ConfigMetadata {
        key: "test.basic".to_string(),
        category: ConfigCategory::System,
        display_name: "基础测试".to_string(),
        description: "基础配置测试".to_string(),
        data_type: ConfigDataType::String,
        default_value: Value::String("default".to_string()),
        validation_rules: vec![],
        is_required: false,
        is_sensitive: false,
        is_readonly: false,
        version: "1.0.0".to_string(),
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    assert_eq!(metadata.key, "test.basic");
    assert_eq!(metadata.category, ConfigCategory::System);
    assert_eq!(metadata.data_type, ConfigDataType::String);

    // 测试配置项创建
    let mut item = ConfigItem::new(metadata.clone());
    assert_eq!(item.current_value, Value::String("default".to_string()));

    // 测试更新值
    item.update_value(Value::String("updated".to_string()), "test");
    assert_eq!(item.current_value, Value::String("updated".to_string()));
    assert_eq!(item.last_modified_by, "test");

    // 测试重置为默认值
    item.reset_to_default("system");
    assert_eq!(item.current_value, Value::String("default".to_string()));
    assert_eq!(item.last_modified_by, "system");

    println!("✓ 配置模型基础测试通过");
}

#[test]
fn test_config_validation_basic() {
    println!("开始配置验证基础测试...");

    // 创建带验证规则的元数据
    let metadata = ConfigMetadata {
        key: "test.validation".to_string(),
        category: ConfigCategory::System,
        display_name: "验证测试".to_string(),
        description: "验证测试配置".to_string(),
        data_type: ConfigDataType::Integer,
        default_value: Value::Number(50.into()),
        validation_rules: vec![
            crate::config::models::ValidationRule::MinValue { value: 0.0 },
            crate::config::models::ValidationRule::MaxValue { value: 100.0 },
        ],
        is_required: false,
        is_sensitive: false,
        is_readonly: false,
        version: "1.0.0".to_string(),
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // 测试有效值
    let result = ConfigValidator::validate(&metadata, &Value::Number(50.into()));
    assert!(result.is_valid, "有效值应该通过验证");

    // 测试无效值（小于最小值）
    let result = ConfigValidator::validate(&metadata, &Value::Number((-10).into()));
    assert!(!result.is_valid, "小于最小值应该失败");
    assert!(!result.errors.is_empty());

    // 测试无效值（大于最大值）
    let result = ConfigValidator::validate(&metadata, &Value::Number(150.into()));
    assert!(!result.is_valid, "大于最大值应该失败");
    assert!(!result.errors.is_empty());

    println!("✓ 配置验证基础测试通过");
}

#[test]
fn test_default_config_generator() {
    println!("开始默认配置生成器测试...");

    let metadata_list = DefaultConfigGenerator::generate_all_metadata();

    // 验证生成了配置
    assert!(!metadata_list.is_empty(), "应该生成默认配置");

    // 验证包含系统配置
    let has_system_config = metadata_list.iter().any(|m| m.category == ConfigCategory::System);
    assert!(has_system_config, "应该包含系统配置");

    // 验证包含压缩配置
    let has_compression_config = metadata_list.iter().any(|m| m.category == ConfigCategory::Compression);
    assert!(has_compression_config, "应该包含压缩配置");

    // 验证包含安全配置
    let has_security_config = metadata_list.iter().any(|m| m.category == ConfigCategory::Security);
    assert!(has_security_config, "应该包含安全配置");

    // 验证配置键格式
    for metadata in &metadata_list {
        assert!(!metadata.key.is_empty(), "配置键不应该为空");
        assert!(!metadata.display_name.is_empty(), "显示名称不应该为空");
        assert!(!metadata.description.is_empty(), "描述不应该为空");
    }

    println!("✓ 默认配置生成器测试通过");
    println!("  生成的配置数量: {}", metadata_list.len());

    // 按分类统计
    use std::collections::HashMap;
    let mut category_counts: HashMap<ConfigCategory, usize> = HashMap::new();
    for metadata in &metadata_list {
        *category_counts.entry(metadata.category).or_insert(0) += 1;
    }

    for (category, count) in category_counts {
        println!("  {}: {} 个配置", category.display_name(), count);
    }
}

#[test]
fn test_config_data_type() {
    println!("开始配置数据类型测试...");

    // 测试字符串解析
    assert_eq!(
        ConfigDataType::from_str("string"),
        Some(ConfigDataType::String)
    );
    assert_eq!(
        ConfigDataType::from_str("integer"),
        Some(ConfigDataType::Integer)
    );
    assert_eq!(
        ConfigDataType::from_str("boolean"),
        Some(ConfigDataType::Boolean)
    );
    assert_eq!(
        ConfigDataType::from_str("float"),
        Some(ConfigDataType::Float)
    );
    assert_eq!(
        ConfigDataType::from_str("array"),
        Some(ConfigDataType::Array)
    );
    assert_eq!(
        ConfigDataType::from_str("object"),
        Some(ConfigDataType::Object)
    );
    assert_eq!(
        ConfigDataType::from_str("enum"),
        Some(ConfigDataType::Enum)
    );
    assert_eq!(ConfigDataType::from_str("unknown"), None);

    // 测试字符串表示
    assert_eq!(ConfigDataType::String.as_str(), "string");
    assert_eq!(ConfigDataType::Integer.as_str(), "integer");
    assert_eq!(ConfigDataType::Boolean.as_str(), "boolean");
    assert_eq!(ConfigDataType::Float.as_str(), "float");
    assert_eq!(ConfigDataType::Array.as_str(), "array");
    assert_eq!(ConfigDataType::Object.as_str(), "object");
    assert_eq!(ConfigDataType::Enum.as_str(), "enum");

    // 测试值验证
    assert!(ConfigDataType::String.validate_value(&Value::String("test".to_string())));
    assert!(!ConfigDataType::String.validate_value(&Value::Number(123.into())));

    assert!(ConfigDataType::Integer.validate_value(&Value::Number(123.into())));
    assert!(!ConfigDataType::Integer.validate_value(&Value::String("123".to_string())));

    assert!(ConfigDataType::Boolean.validate_value(&Value::Bool(true)));
    assert!(!ConfigDataType::Boolean.validate_value(&Value::String("true".to_string())));

    assert!(ConfigDataType::Float.validate_value(&Value::Number(123.45.into())));
    assert!(ConfigDataType::Float.validate_value(&Value::Number(123.into())));

    assert!(ConfigDataType::Array.validate_value(&Value::Array(vec![])));
    assert!(!ConfigDataType::Array.validate_value(&Value::String("[]".to_string())));

    assert!(ConfigDataType::Object.validate_value(&json!({})));
    assert!(!ConfigDataType::Object.validate_value(&Value::String("{}".to_string())));

    assert!(ConfigDataType::Enum.validate_value(&Value::String("option".to_string())));
    assert!(!ConfigDataType::Enum.validate_value(&Value::Number(1.into())));

    println!("✓ 配置数据类型测试通过");
}

#[test]
fn test_config_category() {
    println!("开始配置分类测试...");

    // 测试显示名称
    assert_eq!(ConfigCategory::System.display_name(), "系统配置");
    assert_eq!(ConfigCategory::Compression.display_name(), "压缩配置");
    assert_eq!(ConfigCategory::Security.display_name(), "安全配置");
    assert_eq!(ConfigCategory::Ui.display_name(), "界面配置");
    assert_eq!(ConfigCategory::Network.display_name(), "网络配置");
    assert_eq!(ConfigCategory::Storage.display_name(), "存储配置");
    assert_eq!(ConfigCategory::Advanced.display_name(), "高级配置");
    assert_eq!(ConfigCategory::Other.display_name(), "其他配置");

    // 测试描述
    assert_eq!(ConfigCategory::System.description(), "系统相关配置，如监控、更新等");
    assert_eq!(ConfigCategory::Compression.description(), "压缩解压相关配置");
    assert_eq!(ConfigCategory::Security.description(), "安全相关配置，如密码、加密等");
    assert_eq!(ConfigCategory::Ui.description(), "用户界面相关配置");
    assert_eq!(ConfigCategory::Network.description(), "网络相关配置");
    assert_eq!(ConfigCategory::Storage.description(), "存储相关配置");
    assert_eq!(ConfigCategory::Advanced.description(), "高级功能和实验性配置");
    assert_eq!(ConfigCategory::Other.description(), "其他未分类配置");

    // 测试所有分类
    let all_categories = ConfigCategory::all_categories();
    assert_eq!(all_categories.len(), 8);

    let expected_categories = vec![
        ConfigCategory::System,
        ConfigCategory::Compression,
        ConfigCategory::Security,
        ConfigCategory::Ui,
        ConfigCategory::Network,
        ConfigCategory::Storage,
        ConfigCategory::Advanced,
        ConfigCategory::Other,
    ];

    for expected in expected_categories {
        assert!(all_categories.contains(&expected), "应该包含分类: {:?}", expected);
    }

    println!("✓ 配置分类测试通过");
}

/// 主测试函数
#[test]
fn test_config_system_smoke() {
    println!("开始配置管理系统冒烟测试...");
    println!("{}", "=".repeat(50));

    test_config_models_basic();
    test_config_validation_basic();
    test_default_config_generator();
    test_config_data_type();
    test_config_category();

    println!("{}", "=".repeat(50));
    println!("🎉 配置管理系统冒烟测试通过！");
    println!("核心功能验证：");
    println!("  ✓ 配置模型创建和操作");
    println!("  ✓ 配置验证规则");
    println!("  ✓ 默认配置生成");
    println!("  ✓ 数据类型支持");
    println!("  ✓ 配置分类管理");
    println!();
    println!("配置管理系统已具备基本功能，可以支持：");
    println!("  • 配置项的创建、读取、更新、删除");
    println!("  • 配置值的验证和类型检查");
    println!("  • 默认配置的生成和管理");
    println!("  • 按分类组织配置");
    println!("  • 支持多种数据类型（字符串、整数、布尔值等）");
}