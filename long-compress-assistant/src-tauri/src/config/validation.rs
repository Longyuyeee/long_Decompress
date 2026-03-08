use crate::config::models::{
    ConfigDataType, ConfigMetadata, ValidationError, ValidationResult, ValidationRule, ValidationWarning,
};
use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// 配置验证器
pub struct ConfigValidator;

impl ConfigValidator {
    /// 验证配置值
    pub fn validate(
        metadata: &ConfigMetadata,
        value: &Value,
    ) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // 1. 检查数据类型
        if !metadata.data_type.validate_value(value) {
            result.add_error(ValidationError::new(
                "invalid_data_type",
                &format!(
                    "值的数据类型不符合要求，期望类型：{}",
                    metadata.data_type.as_str()
                ),
                &metadata.key,
            ));
            return result;
        }

        // 2. 检查必填项
        if metadata.is_required && value.is_null() {
            result.add_error(ValidationError::new(
                "required_field",
                "此配置项为必填项",
                &metadata.key,
            ));
        }

        // 3. 应用验证规则
        for rule in &metadata.validation_rules {
            Self::apply_validation_rule(rule, &metadata.key, value, &mut result);
        }

        result
    }

    /// 应用单个验证规则
    fn apply_validation_rule(
        rule: &ValidationRule,
        key: &str,
        value: &Value,
        result: &mut ValidationResult,
    ) {
        match rule {
            ValidationRule::MinLength { value: min_len } => {
                if let Some(s) = value.as_str() {
                    if s.len() < *min_len {
                        result.add_error(ValidationError::with_details(
                            "min_length",
                            &format!("字符串长度不能小于 {} 个字符", min_len),
                            key,
                            Value::Number((*min_len).into()),
                        ));
                    }
                }
            }
            ValidationRule::MaxLength { value: max_len } => {
                if let Some(s) = value.as_str() {
                    if s.len() > *max_len {
                        result.add_error(ValidationError::with_details(
                            "max_length",
                            &format!("字符串长度不能超过 {} 个字符", max_len),
                            key,
                            Value::Number((*max_len).into()),
                        ));
                    }
                }
            }
            ValidationRule::MinValue { value: min_val } => {
                if let Some(num) = value.as_f64() {
                    if num < *min_val {
                        result.add_error(ValidationError::with_details(
                            "min_value",
                            &format!("值不能小于 {}", min_val),
                            key,
                            Value::Number(serde_json::Number::from_f64(*min_val).unwrap()),
                        ));
                    }
                } else if let Some(num) = value.as_i64() {
                    if (num as f64) < *min_val {
                        result.add_error(ValidationError::with_details(
                            "min_value",
                            &format!("值不能小于 {}", min_val),
                            key,
                            Value::Number(serde_json::Number::from_f64(*min_val).unwrap()),
                        ));
                    }
                }
            }
            ValidationRule::MaxValue { value: max_val } => {
                if let Some(num) = value.as_f64() {
                    if num > *max_val {
                        result.add_error(ValidationError::with_details(
                            "max_value",
                            &format!("值不能大于 {}", max_val),
                            key,
                            Value::Number(serde_json::Number::from_f64(*max_val).unwrap()),
                        ));
                    }
                } else if let Some(num) = value.as_i64() {
                    if (num as f64) > *max_val {
                        result.add_error(ValidationError::with_details(
                            "max_value",
                            &format!("值不能大于 {}", max_val),
                            key,
                            Value::Number(serde_json::Number::from_f64(*max_val).unwrap()),
                        ));
                    }
                }
            }
            ValidationRule::Regex { pattern } => {
                if let Some(s) = value.as_str() {
                    match Regex::new(pattern) {
                        Ok(regex) => {
                            if !regex.is_match(s) {
                                result.add_error(ValidationError::with_details(
                                    "regex_pattern",
                                    &format!("值不符合正则表达式模式：{}", pattern),
                                    key,
                                    Value::String(pattern.clone()),
                                ));
                            }
                        }
                        Err(e) => {
                            result.add_error(ValidationError::with_details(
                                "invalid_regex",
                                &format!("正则表达式无效：{}", e),
                                key,
                                Value::String(pattern.clone()),
                            ));
                        }
                    }
                }
            }
            ValidationRule::Range { min, max } => {
                if let Some(num) = value.as_f64() {
                    if num < *min || num > *max {
                        result.add_error(ValidationError::with_details(
                            "range",
                            &format!("值必须在 {} 到 {} 之间", min, max),
                            key,
                            Value::Array(vec![
                                Value::Number(serde_json::Number::from_f64(*min).unwrap()),
                                Value::Number(serde_json::Number::from_f64(*max).unwrap()),
                            ]),
                        ));
                    }
                } else if let Some(num) = value.as_i64() {
                    let num_f64 = num as f64;
                    if num_f64 < *min || num_f64 > *max {
                        result.add_error(ValidationError::with_details(
                            "range",
                            &format!("值必须在 {} 到 {} 之间", min, max),
                            key,
                            Value::Array(vec![
                                Value::Number(serde_json::Number::from_f64(*min).unwrap()),
                                Value::Number(serde_json::Number::from_f64(*max).unwrap()),
                            ]),
                        ));
                    }
                }
            }
            ValidationRule::Enum { values } => {
                if let Some(s) = value.as_str() {
                    if !values.contains(&s.to_string()) {
                        result.add_error(ValidationError::with_details(
                            "enum_value",
                            &format!("值必须是以下之一：{}", values.join(", ")),
                            key,
                            Value::Array(values.iter().map(|v| Value::String(v.clone())).collect()),
                        ));
                    }
                }
            }
            ValidationRule::Custom { function } => {
                // 自定义验证函数，目前只记录警告
                result.add_warning(ValidationWarning::new(
                    "custom_validator",
                    &format!("自定义验证函数 '{}' 需要手动实现", function),
                    key,
                ));
            }
        }
    }

    /// 批量验证配置
    pub fn validate_batch(
        items: &HashMap<String, (ConfigMetadata, Value)>,
    ) -> HashMap<String, ValidationResult> {
        let mut results = HashMap::new();

        for (key, (metadata, value)) in items {
            let result = Self::validate(metadata, value);
            results.insert(key.clone(), result);
        }

        results
    }

    /// 检查配置项之间的一致性
    pub fn validate_consistency(
        items: &HashMap<String, (ConfigMetadata, Value)>,
    ) -> Vec<ValidationWarning> {
        let mut warnings = Vec::new();

        // 示例：检查相关配置的一致性
        // 例如：如果启用了代理，代理URL必须设置
        if let Some((_, proxy_enabled_value)) = items.get("network.proxy_enabled") {
            if let Some(true) = proxy_enabled_value.as_bool() {
                if let Some((_, proxy_url_value)) = items.get("network.proxy_url") {
                    if proxy_url_value.as_str().map(|s| s.trim().is_empty()).unwrap_or(true) {
                        warnings.push(ValidationWarning::new(
                            "consistency",
                            "代理已启用但代理URL未设置",
                            "network.proxy_url",
                        ));
                    }
                }
            }
        }

        warnings
    }
}

/// 配置值转换器
pub struct ConfigValueConverter;

impl ConfigValueConverter {
    /// 将字符串转换为指定类型的值
    pub fn from_string(
        data_type: ConfigDataType,
        value: &str,
    ) -> Result<Value> {
        match data_type {
            ConfigDataType::String => Ok(Value::String(value.to_string())),
            ConfigDataType::Integer => {
                let num = value
                    .parse::<i64>()
                    .context("无法将字符串转换为整数")?;
                Ok(Value::Number(num.into()))
            }
            ConfigDataType::Float => {
                let num = value
                    .parse::<f64>()
                    .context("无法将字符串转换为浮点数")?;
                Ok(Value::Number(
                    serde_json::Number::from_f64(num)
                        .ok_or_else(|| anyhow::anyhow!("无效的浮点数"))?,
                ))
            }
            ConfigDataType::Boolean => {
                let bool_val = match value.to_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => true,
                    "false" | "0" | "no" | "off" => false,
                    _ => return Err(anyhow::anyhow!("无法将字符串转换为布尔值")),
                };
                Ok(Value::Bool(bool_val))
            }
            ConfigDataType::Array => {
                // 简单实现：逗号分隔的数组
                let items: Vec<Value> = value
                    .split(',')
                    .map(|s| Value::String(s.trim().to_string()))
                    .collect();
                Ok(Value::Array(items))
            }
            ConfigDataType::Object => {
                // 简单实现：JSON对象
                serde_json::from_str(value).context("无法将字符串解析为JSON对象")
            }
            ConfigDataType::Enum => Ok(Value::String(value.to_string())),
        }
    }

    /// 将值转换为字符串
    pub fn to_string(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| Self::to_string(v))
                    .collect();
                items.join(",")
            }
            Value::Object(obj) => {
                serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string())
            }
            Value::Null => "".to_string(),
        }
    }

    /// 将值转换为特定类型的Rust值
    pub fn to_typed_value<T>(value: &Value) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_value(value.clone()).context("无法将值转换为目标类型")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::{ConfigCategory, ConfigMetadata};
    use chrono::Utc;

    fn create_test_metadata(
        key: &str,
        data_type: ConfigDataType,
        validation_rules: Vec<ValidationRule>,
    ) -> ConfigMetadata {
        ConfigMetadata {
            key: key.to_string(),
            category: ConfigCategory::System,
            display_name: "Test".to_string(),
            description: "Test".to_string(),
            data_type,
            default_value: Value::Null,
            validation_rules,
            is_required: false,
            is_sensitive: false,
            is_readonly: false,
            version: "1.0.0".to_string(),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_validate_min_length() {
        let metadata = create_test_metadata(
            "test.string",
            ConfigDataType::String,
            vec![ValidationRule::MinLength { value: 5 }],
        );

        // 有效值
        let result = ConfigValidator::validate(&metadata, &Value::String("12345".to_string()));
        assert!(result.is_valid);

        // 无效值
        let result = ConfigValidator::validate(&metadata, &Value::String("123".to_string()));
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, "min_length");
    }

    #[test]
    fn test_validate_max_length() {
        let metadata = create_test_metadata(
            "test.string",
            ConfigDataType::String,
            vec![ValidationRule::MaxLength { value: 10 }],
        );

        // 有效值
        let result = ConfigValidator::validate(&metadata, &Value::String("1234567890".to_string()));
        assert!(result.is_valid);

        // 无效值
        let result = ConfigValidator::validate(&metadata, &Value::String("12345678901".to_string()));
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, "max_length");
    }

    #[test]
    fn test_validate_min_value() {
        let metadata = create_test_metadata(
            "test.number",
            ConfigDataType::Integer,
            vec![ValidationRule::MinValue { value: 10.0 }],
        );

        // 有效值
        let result = ConfigValidator::validate(&metadata, &Value::Number(15.into()));
        assert!(result.is_valid);

        // 无效值
        let result = ConfigValidator::validate(&metadata, &Value::Number(5.into()));
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, "min_value");
    }

    #[test]
    fn test_validate_max_value() {
        let metadata = create_test_metadata(
            "test.number",
            ConfigDataType::Integer,
            vec![ValidationRule::MaxValue { value: 100.0 }],
        );

        // 有效值
        let result = ConfigValidator::validate(&metadata, &Value::Number(50.into()));
        assert!(result.is_valid);

        // 无效值
        let result = ConfigValidator::validate(&metadata, &Value::Number(150.into()));
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, "max_value");
    }

    #[test]
    fn test_validate_regex() {
        let metadata = create_test_metadata(
            "test.email",
            ConfigDataType::String,
            vec![ValidationRule::Regex {
                pattern: r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string(),
            }],
        );

        // 有效值
        let result = ConfigValidator::validate(&metadata, &Value::String("test@example.com".to_string()));
        assert!(result.is_valid);

        // 无效值
        let result = ConfigValidator::validate(&metadata, &Value::String("invalid-email".to_string()));
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, "regex_pattern");
    }

    #[test]
    fn test_validate_enum() {
        let metadata = create_test_metadata(
            "test.enum",
            ConfigDataType::String,
            vec![ValidationRule::Enum {
                values: vec!["option1".to_string(), "option2".to_string(), "option3".to_string()],
            }],
        );

        // 有效值
        let result = ConfigValidator::validate(&metadata, &Value::String("option1".to_string()));
        assert!(result.is_valid);

        // 无效值
        let result = ConfigValidator::validate(&metadata, &Value::String("option4".to_string()));
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, "enum_value");
    }

    #[test]
    fn test_validate_range() {
        let metadata = create_test_metadata(
            "test.range",
            ConfigDataType::Integer,
            vec![ValidationRule::Range { min: 1.0, max: 10.0 }],
        );

        // 有效值
        let result = ConfigValidator::validate(&metadata, &Value::Number(5.into()));
        assert!(result.is_valid);

        // 低于最小值
        let result = ConfigValidator::validate(&metadata, &Value::Number(0.into()));
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, "range");

        // 高于最大值
        let result = ConfigValidator::validate(&metadata, &Value::Number(11.into()));
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, "range");
    }

    #[test]
    fn test_value_converter_string() {
        let result = ConfigValueConverter::from_string(
            ConfigDataType::String,
            "test value",
        ).unwrap();
        assert_eq!(result, Value::String("test value".to_string()));
    }

    #[test]
    fn test_value_converter_integer() {
        let result = ConfigValueConverter::from_string(
            ConfigDataType::Integer,
            "123",
        ).unwrap();
        assert_eq!(result, Value::Number(123.into()));
    }

    #[test]
    fn test_value_converter_float() {
        let result = ConfigValueConverter::from_string(
            ConfigDataType::Float,
            "123.45",
        ).unwrap();
        assert_eq!(result.as_f64().unwrap(), 123.45);
    }

    #[test]
    fn test_value_converter_boolean() {
        let result = ConfigValueConverter::from_string(
            ConfigDataType::Boolean,
            "true",
        ).unwrap();
        assert_eq!(result, Value::Bool(true));

        let result = ConfigValueConverter::from_string(
            ConfigDataType::Boolean,
            "false",
        ).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_value_converter_array() {
        let result = ConfigValueConverter::from_string(
            ConfigDataType::Array,
            "item1,item2,item3",
        ).unwrap();
        assert!(result.is_array());
        let array = result.as_array().unwrap();
        assert_eq!(array.len(), 3);
        assert_eq!(array[0], Value::String("item1".to_string()));
    }

    #[test]
    fn test_to_string() {
        assert_eq!(
            ConfigValueConverter::to_string(&Value::String("test".to_string())),
            "test"
        );
        assert_eq!(
            ConfigValueConverter::to_string(&Value::Number(123.into())),
            "123"
        );
        assert_eq!(
            ConfigValueConverter::to_string(&Value::Bool(true)),
            "true"
        );
        assert_eq!(
            ConfigValueConverter::to_string(&Value::Array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ])),
            "a,b"
        );
    }
}