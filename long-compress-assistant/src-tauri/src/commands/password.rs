use crate::services::password_book_service::{
    PasswordBookService, AddPasswordRequest, UpdatePasswordRequest, PasswordBookEntry,
    SortOption, SearchFilters, PasswordStatistics
};
use crate::services::password_category_service::{
    PasswordCategoryService, PasswordCategory as DbPasswordCategory,
    CreateCategoryRequest, UpdateCategoryRequest, CategoryStatistics
};
use crate::services::password_strength_service::{
    PasswordStrengthService, PasswordStrengthAssessment, PasswordPolicy
};
use crate::models::password::{PasswordCategory, PasswordStrength, CustomField, CustomFieldType};
use tauri::command;
use chrono::{DateTime, Utc};

#[command]
pub async fn add_password(
    name: String,
    username: Option<String>,
    password: String,
    url: Option<String>,
    notes: Option<String>,
    tags: Vec<String>,
    category: String,
    expires_at: Option<DateTime<Utc>>,
    custom_fields: Vec<(String, String, String, bool)>, // (name, value, field_type, sensitive)
) -> Result<PasswordBookEntry, String> {
    let service = PasswordBookService::new();

    // 转换分类
    let password_category = match category.as_str() {
        "Personal" => PasswordCategory::Personal,
        "Work" => PasswordCategory::Work,
        "Finance" => PasswordCategory::Finance,
        "Social" => PasswordCategory::Social,
        "Shopping" => PasswordCategory::Shopping,
        "Entertainment" => PasswordCategory::Entertainment,
        "Education" => PasswordCategory::Education,
        "Travel" => PasswordCategory::Travel,
        "Health" => PasswordCategory::Health,
        "Other" => PasswordCategory::Other,
        _ => PasswordCategory::Other,
    };

    // 转换自定义字段
    let custom_fields_converted: Vec<CustomField> = custom_fields.into_iter()
        .map(|(name, value, field_type, sensitive)| {
            let field_type_enum = match field_type.as_str() {
                "Text" => CustomFieldType::Text,
                "Password" => CustomFieldType::Password,
                "Email" => CustomFieldType::Email,
                "Url" => CustomFieldType::Url,
                "Phone" => CustomFieldType::Phone,
                "Date" => CustomFieldType::Date,
                "Number" => CustomFieldType::Number,
                "MultilineText" => CustomFieldType::MultilineText,
                _ => CustomFieldType::Text,
            };

            CustomField {
                name,
                value,
                field_type: field_type_enum,
                sensitive,
            }
        })
        .collect();

    let request = AddPasswordRequest {
        name,
        username,
        password,
        url,
        notes,
        tags,
        category: password_category,
        expires_at,
        custom_fields: custom_fields_converted,
    };

    match service.add_password(request).await {
        Ok(entry) => Ok(entry),
        Err(e) => Err(format!("添加密码失败: {}", e)),
    }
}

#[command]
pub async fn update_password(
    id: String,
    name: Option<String>,
    username: Option<String>,
    password: Option<String>,
    url: Option<String>,
    notes: Option<String>,
    tags: Option<Vec<String>>,
    category: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    favorite: Option<bool>,
    custom_fields: Option<Vec<(String, String, String, bool)>>,
) -> Result<PasswordBookEntry, String> {
    let service = PasswordBookService::new();

    // 转换分类
    let password_category = category.map(|cat| match cat.as_str() {
        "Personal" => PasswordCategory::Personal,
        "Work" => PasswordCategory::Work,
        "Finance" => PasswordCategory::Finance,
        "Social" => PasswordCategory::Social,
        "Shopping" => PasswordCategory::Shopping,
        "Entertainment" => PasswordCategory::Entertainment,
        "Education" => PasswordCategory::Education,
        "Travel" => PasswordCategory::Travel,
        "Health" => PasswordCategory::Health,
        "Other" => PasswordCategory::Other,
        _ => PasswordCategory::Other,
    });

    // 转换自定义字段
    let custom_fields_converted = custom_fields.map(|fields| {
        fields.into_iter()
            .map(|(name, value, field_type, sensitive)| {
                let field_type_enum = match field_type.as_str() {
                    "Text" => CustomFieldType::Text,
                    "Password" => CustomFieldType::Password,
                    "Email" => CustomFieldType::Email,
                    "Url" => CustomFieldType::Url,
                    "Phone" => CustomFieldType::Phone,
                    "Date" => CustomFieldType::Date,
                    "Number" => CustomFieldType::Number,
                    "MultilineText" => CustomFieldType::MultilineText,
                    _ => CustomFieldType::Text,
                };

                CustomField {
                    name,
                    value,
                    field_type: field_type_enum,
                    sensitive,
                }
            })
            .collect()
    });

    let request = UpdatePasswordRequest {
        id,
        name,
        username,
        password,
        url,
        notes,
        tags,
        category: password_category,
        expires_at,
        favorite,
        custom_fields: custom_fields_converted,
    };

    match service.update_password(request).await {
        Ok(entry) => Ok(entry),
        Err(e) => Err(format!("更新密码失败: {}", e)),
    }
}

#[command]
pub async fn get_password(id: String, include_password: bool) -> Result<Option<PasswordBookEntry>, String> {
    let service = PasswordBookService::new();

    match service.get_password(&id, include_password).await {
        Ok(entry) => Ok(entry),
        Err(e) => Err(format!("获取密码失败: {}", e)),
    }
}

#[command]
pub async fn delete_password(id: String) -> Result<String, String> {
    let service = PasswordBookService::new();

    match service.delete_password(&id).await {
        Ok(_) => Ok("密码删除成功".to_string()),
        Err(e) => Err(format!("删除密码失败: {}", e)),
    }
}

#[command]
pub async fn find_password(query: String) -> Result<Vec<PasswordBookEntry>, String> {
    let service = PasswordBookService::new();
    match service.search_passwords(&query, None).await {
        Ok(entries) => Ok(entries),
        Err(e) => Err(format!("查找密码失败: {}", e)),
    }
}

#[command]
pub async fn archive_password(id: String) -> Result<String, String> {
    let service = PasswordBookService::new();

    match service.archive_password(&id).await {
        Ok(_) => Ok("密码归档成功".to_string()),
        Err(e) => Err(format!("归档密码失败: {}", e)),
    }
}

#[command]
pub async fn validate_password(password: String) -> Result<String, String> {
    let service = PasswordBookService::new();

    match service.validate_password(&password).await {
        Ok(validation) => {
            if validation.is_valid {
                Ok(format!("密码强度: {:?}, 分数: {}", validation.strength, validation.score))
            } else {
                Err(format!("密码验证失败: {:?}", validation.issues))
            }
        }
        Err(e) => Err(format!("验证密码失败: {}", e)),
    }
}

#[command]
pub async fn search_passwords(
    query: String,
    categories: Option<Vec<String>>,
    strengths: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    favorite: Option<bool>,
    archived: Option<bool>,
    expired: Option<bool>,
    created_after: Option<DateTime<Utc>>,
    created_before: Option<DateTime<Utc>>,
    sort_by: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<PasswordBookEntry>, String> {
    let service = PasswordBookService::new();

    // 转换分类
    let categories_converted = categories.map(|cats| {
        cats.into_iter()
            .map(|cat| match cat.as_str() {
                "Personal" => PasswordCategory::Personal,
                "Work" => PasswordCategory::Work,
                "Finance" => PasswordCategory::Finance,
                "Social" => PasswordCategory::Social,
                "Shopping" => PasswordCategory::Shopping,
                "Entertainment" => PasswordCategory::Entertainment,
                "Education" => PasswordCategory::Education,
                "Travel" => PasswordCategory::Travel,
                "Health" => PasswordCategory::Health,
                "Other" => PasswordCategory::Other,
                _ => PasswordCategory::Other,
            })
            .collect()
    }).unwrap_or_default();

    // 转换强度
    let strengths_converted = strengths.map(|strs| {
        strs.into_iter()
            .map(|str| match str.as_str() {
                "VeryWeak" => PasswordStrength::VeryWeak,
                "Weak" => PasswordStrength::Weak,
                "Medium" => PasswordStrength::Medium,
                "Strong" => PasswordStrength::Strong,
                "VeryStrong" => PasswordStrength::VeryStrong,
                _ => PasswordStrength::Weak,
            })
            .collect()
    }).unwrap_or_default();

    // 转换排序选项
    let sort_by_converted = sort_by.map(|s| match s.as_str() {
        "NameAsc" => SortOption::NameAsc,
        "NameDesc" => SortOption::NameDesc,
        "UpdatedAtAsc" => SortOption::UpdatedAtAsc,
        "UpdatedAtDesc" => SortOption::UpdatedAtDesc,
        "CreatedAtAsc" => SortOption::CreatedAtAsc,
        "CreatedAtDesc" => SortOption::CreatedAtDesc,
        "LastUsedAsc" => SortOption::LastUsedAsc,
        "LastUsedDesc" => SortOption::LastUsedDesc,
        "ExpiresAtAsc" => SortOption::ExpiresAtAsc,
        "ExpiresAtDesc" => SortOption::ExpiresAtDesc,
        _ => SortOption::UpdatedAtDesc,
    }).unwrap_or(SortOption::UpdatedAtDesc);

    let filters = SearchFilters {
        categories: categories_converted,
        strengths: strengths_converted,
        tags: tags.unwrap_or_default(),
        favorite,
        archived,
        expired,
        created_after,
        created_before,
        sort_by: sort_by_converted,
        limit,
        offset,
    };

    match service.search_passwords(&query, Some(filters)).await {
        Ok(entries) => Ok(entries),
        Err(e) => Err(format!("搜索密码失败: {}", e)),
    }
}

#[command]
pub async fn get_all_passwords(include_password: bool) -> Result<Vec<PasswordBookEntry>, String> {
    let service = PasswordBookService::new();

    match service.get_all_passwords(include_password).await {
        Ok(entries) => Ok(entries),
        Err(e) => Err(format!("获取所有密码失败: {}", e)),
    }
}

#[command]
pub async fn get_favorite_passwords() -> Result<Vec<PasswordBookEntry>, String> {
    let service = PasswordBookService::new();

    match service.get_favorite_passwords().await {
        Ok(entries) => Ok(entries),
        Err(e) => Err(format!("获取收藏密码失败: {}", e)),
    }
}

#[command]
pub async fn get_recently_used_passwords(limit: i32) -> Result<Vec<PasswordBookEntry>, String> {
    let service = PasswordBookService::new();

    match service.get_recently_used_passwords(limit).await {
        Ok(entries) => Ok(entries),
        Err(e) => Err(format!("获取最近使用的密码失败: {}", e)),
    }
}

#[command]
pub async fn get_expiring_passwords(days_before: i32) -> Result<Vec<PasswordBookEntry>, String> {
    let service = PasswordBookService::new();

    match service.get_expiring_passwords(days_before).await {
        Ok(entries) => Ok(entries),
        Err(e) => Err(format!("获取即将过期的密码失败: {}", e)),
    }
}

#[command]
pub async fn get_password_statistics() -> Result<PasswordStatistics, String> {
    let service = PasswordBookService::new();

    match service.get_password_statistics().await {
        Ok(stats) => Ok(stats),
        Err(e) => Err(format!("获取密码统计信息失败: {}", e)),
    }
}

// 注意：导入导出功能需要更复杂的实现
#[command]
pub async fn import_passwords(_file_path: String, _format: String) -> Result<String, String> {
    Err("导入功能暂未实现".to_string())
}

#[command]
pub async fn export_passwords(_file_path: String, _format: String) -> Result<String, String> {
    Err("导出功能暂未实现".to_string())
}

// ==================== 密码分类管理命令 ====================

#[command]
pub async fn get_all_categories() -> Result<Vec<DbPasswordCategory>, String> {
    let service = PasswordCategoryService::new();

    match service.get_all_categories().await {
        Ok(categories) => Ok(categories),
        Err(e) => Err(format!("获取所有分类失败: {}", e)),
    }
}

#[command]
pub async fn get_category_by_id(id: String) -> Result<Option<DbPasswordCategory>, String> {
    let service = PasswordCategoryService::new();

    match service.get_category_by_id(&id).await {
        Ok(category) => Ok(category),
        Err(e) => Err(format!("根据ID获取分类失败: {}", e)),
    }
}

#[command]
pub async fn get_category_by_name(name: String) -> Result<Option<DbPasswordCategory>, String> {
    let service = PasswordCategoryService::new();

    match service.get_category_by_name(&name).await {
        Ok(category) => Ok(category),
        Err(e) => Err(format!("根据名称获取分类失败: {}", e)),
    }
}

#[command]
pub async fn create_category(
    name: String,
    display_name: String,
    description: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    sort_order: Option<i32>,
) -> Result<DbPasswordCategory, String> {
    let service = PasswordCategoryService::new();

    let request = CreateCategoryRequest {
        name,
        display_name,
        description,
        icon,
        color,
        sort_order,
    };

    match service.create_category(request).await {
        Ok(category) => Ok(category),
        Err(e) => Err(format!("创建分类失败: {}", e)),
    }
}

#[command]
pub async fn update_category(
    id: String,
    name: Option<String>,
    display_name: Option<String>,
    description: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    sort_order: Option<i32>,
) -> Result<DbPasswordCategory, String> {
    let service = PasswordCategoryService::new();

    let request = UpdateCategoryRequest {
        id,
        name,
        display_name,
        description,
        icon,
        color,
        sort_order,
    };

    match service.update_category(request).await {
        Ok(category) => Ok(category),
        Err(e) => Err(format!("更新分类失败: {}", e)),
    }
}

#[command]
pub async fn delete_category(id: String) -> Result<String, String> {
    let service = PasswordCategoryService::new();

    match service.delete_category(&id).await {
        Ok(_) => Ok("分类删除成功".to_string()),
        Err(e) => Err(format!("删除分类失败: {}", e)),
    }
}

#[command]
pub async fn reorder_categories(category_orders: Vec<(String, i32)>) -> Result<String, String> {
    let service = PasswordCategoryService::new();

    match service.reorder_categories(category_orders).await {
        Ok(_) => Ok("分类排序更新成功".to_string()),
        Err(e) => Err(format!("重新排序分类失败: {}", e)),
    }
}

#[command]
pub async fn get_category_statistics() -> Result<Vec<CategoryStatistics>, String> {
    let service = PasswordCategoryService::new();

    match service.get_category_statistics().await {
        Ok(stats) => Ok(stats),
        Err(e) => Err(format!("获取分类统计信息失败: {}", e)),
    }
}

#[command]
pub async fn get_category_usage_over_time(days: i32) -> Result<Vec<(String, Vec<(String, i32)>)>, String> {
    let service = PasswordCategoryService::new();

    match service.get_category_usage_over_time(days).await {
        Ok(usage) => Ok(usage),
        Err(e) => Err(format!("获取分类使用情况失败: {}", e)),
    }
}

#[command]
pub async fn batch_update_passwords_category(
    password_ids: Vec<String>,
    new_category_id: String,
) -> Result<u64, String> {
    let service = PasswordCategoryService::new();

    match service.batch_update_passwords_category(password_ids, &new_category_id).await {
        Ok(updated_count) => Ok(updated_count),
        Err(e) => Err(format!("批量更新密码分类失败: {}", e)),
    }
}

#[command]
pub async fn export_categories() -> Result<Vec<DbPasswordCategory>, String> {
    let service = PasswordCategoryService::new();

    match service.export_categories().await {
        Ok(categories) => Ok(categories),
        Err(e) => Err(format!("导出分类配置失败: {}", e)),
    }
}

#[command]
pub async fn import_categories(categories: Vec<DbPasswordCategory>) -> Result<u64, String> {
    let service = PasswordCategoryService::new();

    match service.import_categories(categories).await {
        Ok(imported_count) => Ok(imported_count),
        Err(e) => Err(format!("导入分类配置失败: {}", e)),
    }
}

// ==================== 密码强度评估命令 ====================

#[command]
pub async fn assess_password_strength(password: String) -> Result<PasswordStrengthAssessment, String> {
    let service = PasswordStrengthService::new();

    let assessment = service.assess_password(&password);
    Ok(assessment)
}

#[command]
pub async fn assess_passwords_strength_batch(passwords: Vec<String>) -> Result<Vec<PasswordStrengthAssessment>, String> {
    let service = PasswordStrengthService::new();

    let password_refs: Vec<&str> = passwords.iter().map(|s| s.as_str()).collect();
    let assessments = service.assess_passwords_batch(&password_refs);
    Ok(assessments)
}

#[command]
pub async fn compare_passwords_similarity(password1: String, password2: String) -> Result<f64, String> {
    let service = PasswordStrengthService::new();

    let similarity = service.compare_passwords(&password1, &password2);
    Ok(similarity as f64)
}

#[command]
pub async fn check_password_policy_compliance(password: String) -> Result<(bool, Vec<String>), String> {
    let service = PasswordStrengthService::new();

    let (is_compliant, violations) = service.check_password_policy(&password);
    Ok((is_compliant, violations))
}

#[command]
pub async fn generate_password_strength_report(password: String) -> Result<String, String> {
    let service = PasswordStrengthService::new();

    let report = service.generate_strength_report(&password);
    Ok(report)
}

#[command]
pub async fn get_password_policy() -> Result<PasswordPolicy, String> {
    let _service = PasswordStrengthService::new();

    // 返回默认策略，实际应用中可以从数据库或配置文件中加载
    Ok(PasswordPolicy::default())
}

#[command]
pub async fn update_password_policy(
    min_length: Option<usize>,
    max_length: Option<usize>,
    require_lowercase: Option<bool>,
    require_uppercase: Option<bool>,
    require_digits: Option<bool>,
    require_symbols: Option<bool>,
    min_entropy_bits: Option<f64>,
    max_repeated_chars: Option<usize>,
    max_sequential_chars: Option<usize>,
    check_common_passwords: Option<bool>,
    check_dictionary_words: Option<bool>,
    check_keyboard_patterns: Option<bool>,
    check_date_patterns: Option<bool>,
) -> Result<PasswordPolicy, String> {
    let mut policy = PasswordPolicy::default();

    if let Some(val) = min_length { policy.min_length = val; }
    if let Some(val) = max_length { policy.max_length = val; }
    if let Some(val) = require_lowercase { policy.require_lowercase = val; }
    if let Some(val) = require_uppercase { policy.require_uppercase = val; }
    if let Some(val) = require_digits { policy.require_digits = val; }
    if let Some(val) = require_symbols { policy.require_symbols = val; }
    if let Some(val) = min_entropy_bits { policy.min_entropy_bits = val; }
    if let Some(val) = max_repeated_chars { policy.max_repeated_chars = val; }
    if let Some(val) = max_sequential_chars { policy.max_sequential_chars = val; }
    if let Some(val) = check_common_passwords { policy.check_common_passwords = val; }
    if let Some(val) = check_dictionary_words { policy.check_dictionary_words = val; }
    if let Some(val) = check_keyboard_patterns { policy.check_keyboard_patterns = val; }
    if let Some(val) = check_date_patterns { policy.check_date_patterns = val; }

    Ok(policy)
}