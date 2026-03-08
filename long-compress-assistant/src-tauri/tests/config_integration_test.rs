//! 配置系统集成测试

use crate::config::models::{ConfigCategory, ConfigDataType, ConfigMetadata, DefaultConfigGenerator, ValidationRule};
use crate::config::validation::ConfigValidator;
use crate::config::repository::ConfigRepository;
use crate::config::service::ConfigService;
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tempfile::tempdir;

#[tokio::test]
async fn test_default_config_generator() {
    let metadata_list = DefaultConfigGenerator::generate_all_metadata();

    // 验证生成了配置
    assert!(!metadata_list.is_empty());

    // 验证包含系统配置
    let has_system_config = metadata_list.iter().any(|m| m.key == "system.language");
    assert!(has_system_config);

    // 验证包含压缩配置
    let has_compression_config = metadata_list.iter().any(|m| m.key == "compression.default_format");
    assert!(has_compression_config);

    // 验证包含安全配置
    let has_security_config = metadata_list.iter().any(|m| m.key == "security.master_password_enabled");
    assert!(has_security_config);

    // 验证包含界面配置
    let has_ui_config = metadata_list.iter().any(|m| m.key == "ui.theme");
    assert!(has_ui_config);
}

#[tokio::test]
async fn test_config_validation() {
    // 测试字符串长度验证
    let metadata = ConfigMetadata {
        key: "test.string".to_string(),
        category: ConfigCategory::System,
        display_name: "测试字符串".to_string(),
        description: "测试字符串配置".to_string(),
        data_type: ConfigDataType::String,
        default_value: Value::String("default".to_string()),
        validation_rules: vec![
            ValidationRule::MinLength { value: 5 },
            ValidationRule::MaxLength { value: 10 },
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
    let result = ConfigValidator::validate(&metadata, &Value::String("valid".to_string()));
    assert!(result.is_valid);

    // 太短的值
    let result = ConfigValidator::validate(&metadata, &Value::String("abc".to_string()));
    assert!(!result.is_valid);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].code, "min_length");

    // 太长的值
    let result = ConfigValidator::validate(&metadata, &Value::String("this_is_too_long".to_string()));
    assert!(!result.is_valid);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].code, "max_length");
}

#[tokio::test]
async fn test_config_repository() {
    // 创建临时数据库
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}", db_path.display()))
        .await
        .unwrap();

    let repo = ConfigRepository::new(pool);

    // 初始化表
    repo.init_tables().await.unwrap();

    // 生成测试元数据
    let metadata = ConfigMetadata {
        key: "test.repository".to_string(),
        category: ConfigCategory::System,
        display_name: "仓库测试".to_string(),
        description: "仓库测试配置".to_string(),
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

    // 创建配置项
    use crate::config::models::ConfigItem;
    let mut item = ConfigItem::new(metadata.clone());
    item.update_value(Value::String("test value".to_string()), "test");

    // 保存配置
    repo.save_config(&item).await.unwrap();

    // 获取配置
    let retrieved = repo.get_config("test.repository", &metadata).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved_item = retrieved.unwrap();
    assert_eq!(retrieved_item.current_value, Value::String("test value".to_string()));

    // 删除配置
    repo.delete_config("test.repository").await.unwrap();

    // 验证配置已删除
    let deleted = repo.get_config("test.repository", &metadata).await.unwrap();
    assert!(deleted.is_none());
}

#[tokio::test]
async fn test_config_service() {
    // 创建临时数据库
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}", db_path.display()))
        .await
        .unwrap();

    let service = ConfigService::new(pool);

    // 初始化服务
    service.init().await.unwrap();

    // 获取所有配置
    let configs = service.get_all_configs().await.unwrap();
    assert!(!configs.is_empty());

    // 获取特定配置
    let language_config = service.get_config("system.language").await.unwrap();
    assert!(language_config.is_some());
    let language_value = service.get_string("system.language").await.unwrap();
    assert_eq!(language_value, Some("zh-CN".to_string()));

    // 修改配置
    service.set_config(
        "system.language",
        Value::String("en-US".to_string()),
        "test"
    ).await.unwrap();

    // 验证配置已更新
    let updated_value = service.get_string("system.language").await.unwrap();
    assert_eq!(updated_value, Some("en-US".to_string()));

    // 重置配置
    service.reset_to_default("system.language", "test").await.unwrap();

    // 验证配置已重置
    let reset_value = service.get_string("system.language").await.unwrap();
    assert_eq!(reset_value, Some("zh-CN".to_string()));

    // 获取统计信息
    let stats = service.get_statistics().await.unwrap();
    assert!(stats.total_configs > 0);
    assert!(stats.cached_configs > 0);
}

#[tokio::test]
async fn test_config_categories() {
    let service = create_test_service().await;

    // 测试按分类获取配置
    let system_configs = service.get_configs_by_category(ConfigCategory::System).await.unwrap();
    assert!(!system_configs.is_empty());

    // 验证所有配置都是系统分类
    for config in &system_configs {
        assert_eq!(config.metadata.category, ConfigCategory::System);
    }

    // 测试压缩配置
    let compression_configs = service.get_configs_by_category(ConfigCategory::Compression).await.unwrap();
    assert!(!compression_configs.is_empty());

    for config in &compression_configs {
        assert_eq!(config.metadata.category, ConfigCategory::Compression);
    }
}

#[tokio::test]
async fn test_config_search() {
    let service = create_test_service().await;

    // 搜索包含"language"的配置
    let results = service.search_configs("language").await.unwrap();
    assert!(!results.is_empty());

    // 验证结果包含语言配置
    let has_language = results.iter().any(|item| item.metadata.key.contains("language"));
    assert!(has_language);

    // 搜索不存在的配置
    let empty_results = service.search_configs("nonexistent").await.unwrap();
    assert!(empty_results.is_empty());
}

#[tokio::test]
async fn test_config_validation_service() {
    let service = create_test_service().await;

    // 测试有效值
    let result = service.validate_config(
        "system.update_interval",
        &Value::Number(5000.into())
    ).await.unwrap();
    assert!(result.is_valid);

    // 测试无效值
    let result = service.validate_config(
        "system.update_interval",
        &Value::Number(500.into())
    ).await.unwrap();
    assert!(!result.is_valid);
    assert!(!result.errors.is_empty());
}

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

// 主测试函数
#[tokio::test]
async fn test_config_system_integration() {
    println!("开始配置系统集成测试...");

    // 测试默认配置生成
    test_default_config_generator().await;
    println!("✓ 默认配置生成测试通过");

    // 测试配置验证
    test_config_validation().await;
    println!("✓ 配置验证测试通过");

    // 测试配置仓库
    test_config_repository().await;
    println!("✓ 配置仓库测试通过");

    // 测试配置服务
    test_config_service().await;
    println!("✓ 配置服务测试通过");

    // 测试配置分类
    test_config_categories().await;
    println!("✓ 配置分类测试通过");

    // 测试配置搜索
    test_config_search().await;
    println!("✓ 配置搜索测试通过");

    // 测试配置验证服务
    test_config_validation_service().await;
    println!("✓ 配置验证服务测试通过");

    println!("所有配置系统测试通过！");
}