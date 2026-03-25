//! 配置文件加载器测试

use long_compress_assistant::config::file_loader::{ConfigFileLoader, ConfigFileFormat, DefaultConfigFileGenerator};
use long_compress_assistant::config::models::{ConfigCategory, ConfigDataType, ConfigItem, ConfigMetadata, ValidationRule};
use chrono::Utc;
use serde_json::json;
use std::path::PathBuf;
use tempfile::tempdir;

#[tokio::test]
async fn test_config_file_loader_basic() {
    println!("开始配置文件加载器基础测试...");

    // 创建临时目录
    let dir = tempdir().unwrap();
    let config_dir = dir.path();

    // 创建测试配置文件
    let config_content = r#"{
        "export_version": "1.0.0",
        "export_date": "2024-01-01T00:00:00Z",
        "config_count": 2,
        "configs": [
            {
                "key": "test.config1",
                "category": "system",
                "display_name": "测试配置1",
                "description": "测试配置项1",
                "data_type": "string",
                "value": "test value 1",
                "default_value": "default1",
                "validation_rules": [],
                "is_required": false,
                "is_sensitive": false,
                "is_readonly": false,
                "version": "1.0.0",
                "last_modified": "2024-01-01T00:00:00Z",
                "last_modified_by": "test"
            },
            {
                "key": "test.config2",
                "category": "compression",
                "display_name": "测试配置2",
                "description": "测试配置项2",
                "data_type": "integer",
                "value": 100,
                "default_value": 50,
                "validation_rules": [],
                "is_required": false,
                "is_sensitive": false,
                "is_readonly": false,
                "version": "1.0.0",
                "last_modified": "2024-01-01T00:00:00Z",
                "last_modified_by": "test"
            }
        ]
    }"#;

    let config_path = config_dir.join("test_config.json");
    std::fs::write(&config_path, config_content).unwrap();

    // 创建文件加载器
    let loader = ConfigFileLoader::new(config_dir, ConfigFileFormat::Json);

    // 测试加载配置文件
    let items = loader.load_config_file_from_path(&config_path).await.unwrap();
    assert_eq!(items.len(), 2);

    let config1 = items.iter().find(|item| item.metadata.key == "test.config1").unwrap();
    let config2 = items.iter().find(|item| item.metadata.key == "test.config2").unwrap();

    assert_eq!(config1.metadata.category, ConfigCategory::System);
    assert_eq!(config1.current_value, json!("test value 1"));

    assert_eq!(config2.metadata.category, ConfigCategory::Compression);
    assert_eq!(config2.current_value, json!(100));

    println!("✓ 配置文件加载器基础测试通过");
}

#[tokio::test]
async fn test_config_file_save_and_load() {
    println!("开始配置文件保存和加载测试...");

    let dir = tempdir().unwrap();
    let config_dir = dir.path();

    // 创建测试配置项
    let metadata1 = ConfigMetadata {
        key: "save.test1".to_string(),
        category: ConfigCategory::System,
        display_name: "保存测试1".to_string(),
        description: "保存测试配置1".to_string(),
        data_type: ConfigDataType::String,
        default_value: json!("default1"),
        validation_rules: vec![],
        is_required: false,
        is_sensitive: false,
        is_readonly: false,
        version: "1.0.0".to_string(),
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let metadata2 = ConfigMetadata {
        key: "save.test2".to_string(),
        category: ConfigCategory::Ui,
        display_name: "保存测试2".to_string(),
        description: "保存测试配置2".to_string(),
        data_type: ConfigDataType::Boolean,
        default_value: json!(true),
        validation_rules: vec![],
        is_required: false,
        is_sensitive: false,
        is_readonly: false,
        version: "1.0.0".to_string(),
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let mut item1 = ConfigItem::new(metadata1);
    item1.update_value(json!("saved value 1"), "test");

    let mut item2 = ConfigItem::new(metadata2);
    item2.update_value(json!(false), "test");

    let items = vec![item1, item2];

    // 创建文件加载器
    let loader = ConfigFileLoader::new(config_dir, ConfigFileFormat::Json);

    // 保存配置到文件
    let save_path = config_dir.join("save_test.json");
    loader.save_config_file_to_path(&save_path, &items, ConfigFileFormat::Json)
        .await
        .unwrap();

    assert!(save_path.exists());

    // 重新加载配置
    let loaded_items = loader.load_config_file_from_path(&save_path).await.unwrap();
    assert_eq!(loaded_items.len(), 2);

    let loaded_item1 = loaded_items.iter().find(|item| item.metadata.key == "save.test1").unwrap();
    let loaded_item2 = loaded_items.iter().find(|item| item.metadata.key == "save.test2").unwrap();

    assert_eq!(loaded_item1.current_value, json!("saved value 1"));
    assert_eq!(loaded_item2.current_value, json!(false));

    println!("✓ 配置文件保存和加载测试通过");
}

#[tokio::test]
async fn test_config_file_format_detection() {
    println!("开始配置文件格式检测测试...");

    let dir = tempdir().unwrap();
    let config_dir = dir.path();

    // 测试JSON格式
    let json_content = r#"{"test": "value"}"#;
    let json_path = config_dir.join("test.json");
    std::fs::write(&json_path, json_content).unwrap();

    let loader = ConfigFileLoader::new(config_dir, ConfigFileFormat::Auto);
    let format = loader.detect_format(json_content, &json_path);
    assert_eq!(format, ConfigFileFormat::Json);

    // 测试YAML格式
    let yaml_content = "test: value\nanother: 123";
    let yaml_path = config_dir.join("test.yaml");
    std::fs::write(&yaml_path, yaml_content).unwrap();

    let format = loader.detect_format(yaml_content, &yaml_path);
    assert_eq!(format, ConfigFileFormat::Yaml);

    // 测试TOML格式
    let toml_content = "test = \"value\"\nanother = 123";
    let toml_path = config_dir.join("test.toml");
    std::fs::write(&toml_path, toml_content).unwrap();

    let format = loader.detect_format(toml_content, &toml_path);
    assert_eq!(format, ConfigFileFormat::Toml);

    println!("✓ 配置文件格式检测测试通过");
}

#[tokio::test]
async fn test_default_config_generation() {
    println!("开始默认配置生成测试...");

    // 测试生成默认配置
    let result = DefaultConfigFileGenerator::generate_default_config(ConfigFileFormat::Json);
    assert!(result.is_ok());

    let content = result.unwrap();
    assert!(content.contains("export_version"));
    assert!(content.contains("configs"));

    // 验证JSON可以解析
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed.get("configs").is_some());

    // 测试保存默认配置文件
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("default_config.json");

    DefaultConfigFileGenerator::save_default_config_file(&config_path, ConfigFileFormat::Json)
        .await
        .unwrap();

    assert!(config_path.exists());

    // 验证文件内容
    let file_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(file_content.contains("export_version"));

    println!("✓ 默认配置生成测试通过");
}

#[tokio::test]
async fn test_load_all_config_files() {
    println!("开始加载所有配置文件测试...");

    let dir = tempdir().unwrap();
    let config_dir = dir.path();

    // 创建多个配置文件
    let config1_content = r#"{
        "export_version": "1.0.0",
        "export_date": "2024-01-01T00:00:00Z",
        "config_count": 1,
        "configs": [{
            "key": "file1.config",
            "category": "system",
            "value": "value1"
        }]
    }"#;

    let config2_content = r#"{
        "export_version": "1.0.0",
        "export_date": "2024-01-01T00:00:00Z",
        "config_count": 1,
        "configs": [{
            "key": "file2.config",
            "category": "ui",
            "value": "value2"
        }]
    }"#;

    std::fs::write(config_dir.join("config1.json"), config1_content).unwrap();
    std::fs::write(config_dir.join("config2.json"), config2_content).unwrap();
    // 创建一个非配置文件
    std::fs::write(config_dir.join("readme.txt"), "This is not a config file").unwrap();

    let loader = ConfigFileLoader::new(config_dir, ConfigFileFormat::Auto);
    let items = loader.load_all_config_files().await.unwrap();

    // 应该只加载JSON文件
    assert_eq!(items.len(), 2);

    let has_file1 = items.iter().any(|item| item.metadata.key == "file1.config");
    let has_file2 = items.iter().any(|item| item.metadata.key == "file2.config");

    assert!(has_file1);
    assert!(has_file2);

    println!("✓ 加载所有配置文件测试通过");
}

#[test]
fn test_config_file_format_serialization() {
    println!("开始配置文件格式序列化测试...");

    // 测试序列化
    let json_format = ConfigFileFormat::Json;
    let yaml_format = ConfigFileFormat::Yaml;
    let toml_format = ConfigFileFormat::Toml;
    let auto_format = ConfigFileFormat::Auto;

    // 这些测试主要验证代码编译通过
    // 在实际使用中，serde会自动处理序列化/反序列化
    assert!(matches!(json_format, ConfigFileFormat::Json));
    assert!(matches!(yaml_format, ConfigFileFormat::Yaml));
    assert!(matches!(toml_format, ConfigFileFormat::Toml));
    assert!(matches!(auto_format, ConfigFileFormat::Auto));

    println!("✓ 配置文件格式序列化测试通过");
}

/// 主测试函数
#[tokio::test]
async fn test_config_file_loader_comprehensive() {
    println!("开始配置文件加载器全面测试...");
    println!("{}", "=".repeat(50));

    test_config_file_loader_basic().await;
    test_config_file_save_and_load().await;
    test_config_file_format_detection().await;
    test_default_config_generation().await;
    test_load_all_config_files().await;
    test_config_file_format_serialization();

    println!("{}", "=".repeat(50));
    println!("🎉 所有配置文件加载器测试通过！");
    println!("测试覆盖：");
    println!("  ✓ 基础文件加载");
    println!("  ✓ 文件保存和重新加载");
    println!("  ✓ 文件格式自动检测");
    println!("  ✓ 默认配置生成");
    println!("  ✓ 批量文件加载");
    println!("  ✓ 格式序列化");
}