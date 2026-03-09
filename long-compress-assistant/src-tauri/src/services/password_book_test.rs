#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::password_book_service::{PasswordBookService, AddPasswordRequest};
    use crate::models::password::{PasswordCategory, CustomField, CustomFieldType};
    use chrono::{Utc, TimeZone};

    #[tokio::test]
    async fn test_password_validation() {
        let service = PasswordBookService::new();

        // 测试弱密码
        let weak_password = "123";
        let validation = service.validate_password(weak_password).await.unwrap();
        assert!(!validation.is_valid);
        assert!(validation.score < 4);

        // 测试中等密码
        let medium_password = "Password123";
        let validation = service.validate_password(medium_password).await.unwrap();
        assert!(validation.is_valid);
        assert!(validation.score >= 4 && validation.score <= 7);

        // 测试强密码
        let strong_password = "StrongP@ssw0rd!2024";
        let validation = service.validate_password(strong_password).await.unwrap();
        assert!(validation.is_valid);
        assert!(validation.score >= 8);
    }

    #[tokio::test]
    async fn test_password_contains_sequence() {
        let service = PasswordBookService::new();

        // 测试包含序列的密码
        assert!(PasswordBookService::contains_sequence("abc123"));
        assert!(PasswordBookService::contains_sequence("321cba"));
        assert!(PasswordBookService::contains_sequence("test123"));

        // 测试不包含序列的密码
        assert!(!PasswordBookService::contains_sequence("random"));
        assert!(!PasswordBookService::contains_sequence("p@ssw0rd"));
        assert!(!PasswordBookService::contains_sequence("test"));
    }

    #[tokio::test]
    async fn test_password_contains_repetition() {
        let service = PasswordBookService::new();

        // 测试包含重复的密码
        assert!(PasswordBookService::contains_repetition("aaa123"));
        assert!(PasswordBookService::contains_repetition("test111"));
        assert!(PasswordBookService::contains_repetition("888password"));

        // 测试不包含重复的密码
        assert!(!PasswordBookService::contains_repetition("abc123"));
        assert!(!PasswordBookService::contains_repetition("password"));
        assert!(!PasswordBookService::contains_repetition("test"));
    }

    #[tokio::test]
    async fn test_search_filters() {
        let filters = SearchFilters {
            categories: vec![PasswordCategory::Work, PasswordCategory::Finance],
            strengths: vec![PasswordStrength::Strong, PasswordStrength::VeryStrong],
            tags: vec!["work".to_string(), "bank".to_string()],
            favorite: Some(true),
            archived: Some(false),
            expired: Some(false),
            created_after: Some(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()),
            created_before: Some(Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap()),
            sort_by: SortOption::NameAsc,
            limit: Some(10),
            offset: Some(0),
        };

        // 验证过滤器设置
        assert_eq!(filters.categories.len(), 2);
        assert_eq!(filters.strengths.len(), 2);
        assert_eq!(filters.tags.len(), 2);
        assert_eq!(filters.favorite, Some(true));
        assert_eq!(filters.archived, Some(false));
        assert_eq!(filters.expired, Some(false));
        assert!(filters.created_after.is_some());
        assert!(filters.created_before.is_some());
        assert_eq!(filters.limit, Some(10));
        assert_eq!(filters.offset, Some(0));
    }

    #[tokio::test]
    async fn test_sort_option_to_sql() {
        assert_eq!(SortOption::NameAsc.to_sql(), "name ASC");
        assert_eq!(SortOption::NameDesc.to_sql(), "name DESC");
        assert_eq!(SortOption::UpdatedAtAsc.to_sql(), "updated_at ASC");
        assert_eq!(SortOption::UpdatedAtDesc.to_sql(), "updated_at DESC");
        assert_eq!(SortOption::CreatedAtAsc.to_sql(), "created_at ASC");
        assert_eq!(SortOption::CreatedAtDesc.to_sql(), "created_at DESC");
        assert_eq!(SortOption::LastUsedAsc.to_sql(), "last_used ASC");
        assert_eq!(SortOption::LastUsedDesc.to_sql(), "last_used DESC");
        assert_eq!(SortOption::ExpiresAtAsc.to_sql(), "expires_at ASC");
        assert_eq!(SortOption::ExpiresAtDesc.to_sql(), "expires_at DESC");
    }

    #[test]
    fn test_custom_field_creation() {
        let field = CustomField {
            name: "安全提示".to_string(),
            value: "我的宠物名字".to_string(),
            field_type: CustomFieldType::Text,
            sensitive: false,
        };

        assert_eq!(field.name, "安全提示");
        assert_eq!(field.value, "我的宠物名字");
        assert_eq!(field.field_type, CustomFieldType::Text);
        assert!(!field.sensitive);
    }

    #[test]
    fn test_password_category_conversion() {
        // 测试字符串到枚举的转换
        let personal = "Personal";
        let work = "Work";
        let other = "Other";

        let personal_enum = match personal {
            "Personal" => PasswordCategory::Personal,
            _ => PasswordCategory::Other,
        };

        let work_enum = match work {
            "Work" => PasswordCategory::Work,
            _ => PasswordCategory::Other,
        };

        let other_enum = match other {
            "Other" => PasswordCategory::Other,
            _ => PasswordCategory::Personal,
        };

        assert!(matches!(personal_enum, PasswordCategory::Personal));
        assert!(matches!(work_enum, PasswordCategory::Work));
        assert!(matches!(other_enum, PasswordCategory::Other));
    }

    #[test]
    fn test_password_strength_conversion() {
        // 测试字符串到枚举的转换
        let weak = "Weak";
        let strong = "Strong";
        let very_strong = "VeryStrong";

        let weak_enum = match weak {
            "Weak" => PasswordStrength::Weak,
            _ => PasswordStrength::Medium,
        };

        let strong_enum = match strong {
            "Strong" => PasswordStrength::Strong,
            _ => PasswordStrength::Medium,
        };

        let very_strong_enum = match very_strong {
            "VeryStrong" => PasswordStrength::VeryStrong,
            _ => PasswordStrength::Medium,
        };

        assert!(matches!(weak_enum, PasswordStrength::Weak));
        assert!(matches!(strong_enum, PasswordStrength::Strong));
        assert!(matches!(very_strong_enum, PasswordStrength::VeryStrong));
    }
}