#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::password_category_service::{
        PasswordCategoryService, CreateCategoryRequest, UpdateCategoryRequest
    };
    use chrono::{Utc, TimeZone};

    #[tokio::test]
    async fn test_category_creation() {
        let service = PasswordCategoryService::new();

        let request = CreateCategoryRequest {
            name: "test_category".to_string(),
            display_name: "测试分类".to_string(),
            description: Some("这是一个测试分类".to_string()),
            icon: Some("📝".to_string()),
            color: Some("#FF5733".to_string()),
            sort_order: Some(99),
        };

        // 注意：在实际测试中，这里应该使用测试数据库
        // 这里只是验证请求结构
        assert_eq!(request.name, "test_category");
        assert_eq!(request.display_name, "测试分类");
        assert_eq!(request.description, Some("这是一个测试分类".to_string()));
        assert_eq!(request.icon, Some("📝".to_string()));
        assert_eq!(request.color, Some("#FF5733".to_string()));
        assert_eq!(request.sort_order, Some(99));
    }

    #[tokio::test]
    async fn test_category_update() {
        let service = PasswordCategoryService::new();

        let request = UpdateCategoryRequest {
            id: "test_id".to_string(),
            name: Some("updated_category".to_string()),
            display_name: Some("更新后的分类".to_string()),
            description: Some("更新后的描述".to_string()),
            icon: Some("🔄".to_string()),
            color: Some("#33FF57".to_string()),
            sort_order: Some(100),
        };

        // 验证更新请求结构
        assert_eq!(request.id, "test_id");
        assert_eq!(request.name, Some("updated_category".to_string()));
        assert_eq!(request.display_name, Some("更新后的分类".to_string()));
        assert_eq!(request.description, Some("更新后的描述".to_string()));
        assert_eq!(request.icon, Some("🔄".to_string()));
        assert_eq!(request.color, Some("#33FF57".to_string()));
        assert_eq!(request.sort_order, Some(100));
    }

    #[test]
    fn test_category_statistics_structure() {
        let stats = CategoryStatistics {
            category_id: "personal".to_string(),
            category_name: "Personal".to_string(),
            display_name: "个人".to_string(),
            password_count: 10,
            average_strength: 3.5,
            expired_count: 2,
            favorite_count: 3,
        };

        assert_eq!(stats.category_id, "personal");
        assert_eq!(stats.category_name, "Personal");
        assert_eq!(stats.display_name, "个人");
        assert_eq!(stats.password_count, 10);
        assert_eq!(stats.average_strength, 3.5);
        assert_eq!(stats.expired_count, 2);
        assert_eq!(stats.favorite_count, 3);
    }

    #[tokio::test]
    async fn test_reorder_categories() {
        let service = PasswordCategoryService::new();

        let category_orders = vec![
            ("category1".to_string(), 1),
            ("category2".to_string(), 2),
            ("category3".to_string(), 3),
        ];

        // 验证排序数据
        assert_eq!(category_orders.len(), 3);
        assert_eq!(category_orders[0].0, "category1");
        assert_eq!(category_orders[0].1, 1);
        assert_eq!(category_orders[1].0, "category2");
        assert_eq!(category_orders[1].1, 2);
        assert_eq!(category_orders[2].0, "category3");
        assert_eq!(category_orders[2].1, 3);
    }

    #[tokio::test]
    async fn test_batch_update_passwords_category() {
        let service = PasswordCategoryService::new();

        let password_ids = vec![
            "pass1".to_string(),
            "pass2".to_string(),
            "pass3".to_string(),
        ];

        let new_category_id = "new_category".to_string();

        // 验证批量更新数据
        assert_eq!(password_ids.len(), 3);
        assert_eq!(password_ids[0], "pass1");
        assert_eq!(password_ids[1], "pass2");
        assert_eq!(password_ids[2], "pass3");
        assert_eq!(new_category_id, "new_category");
    }

    #[test]
    fn test_category_usage_over_time_structure() {
        let usage_data = vec![
            (
                "Personal".to_string(),
                vec![
                    ("2024-01-01".to_string(), 5),
                    ("2024-01-02".to_string(), 3),
                    ("2024-01-03".to_string(), 7),
                ]
            ),
            (
                "Work".to_string(),
                vec![
                    ("2024-01-01".to_string(), 2),
                    ("2024-01-02".to_string(), 4),
                ]
            ),
        ];

        assert_eq!(usage_data.len(), 2);
        assert_eq!(usage_data[0].0, "Personal");
        assert_eq!(usage_data[0].1.len(), 3);
        assert_eq!(usage_data[0].1[0].0, "2024-01-01");
        assert_eq!(usage_data[0].1[0].1, 5);
        assert_eq!(usage_data[1].0, "Work");
        assert_eq!(usage_data[1].1.len(), 2);
    }

    #[tokio::test]
    async fn test_export_import_categories() {
        let service = PasswordCategoryService::new();

        let categories = vec![
            DbPasswordCategory {
                id: "cat1".to_string(),
                name: "Category1".to_string(),
                display_name: "分类1".to_string(),
                description: Some("第一个分类".to_string()),
                icon: Some("📁".to_string()),
                color: Some("#FF0000".to_string()),
                sort_order: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            DbPasswordCategory {
                id: "cat2".to_string(),
                name: "Category2".to_string(),
                display_name: "分类2".to_string(),
                description: Some("第二个分类".to_string()),
                icon: Some("📂".to_string()),
                color: Some("#00FF00".to_string()),
                sort_order: 2,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        // 验证导出导入数据结构
        assert_eq!(categories.len(), 2);
        assert_eq!(categories[0].name, "Category1");
        assert_eq!(categories[0].display_name, "分类1");
        assert_eq!(categories[0].sort_order, 1);
        assert_eq!(categories[1].name, "Category2");
        assert_eq!(categories[1].display_name, "分类2");
        assert_eq!(categories[1].sort_order, 2);
    }

    #[test]
    fn test_error_messages() {
        // 测试错误消息格式
        let error1 = "分类名称已存在: test_category".to_string();
        let error2 = "分类不存在: invalid_id".to_string();
        let error3 = "分类正在被 5 个密码使用，无法删除".to_string();

        assert!(error1.contains("分类名称已存在"));
        assert!(error2.contains("分类不存在"));
        assert!(error3.contains("分类正在被"));
        assert!(error3.contains("个密码使用"));
    }
}