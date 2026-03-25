#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::password_strength_service::{
        PasswordStrengthService, PasswordStrengthAssessment, PasswordPolicy,
        PasswordIssueType, IssueSeverity
    };

    #[test]
    fn test_password_strength_assessment() {
        let service = PasswordStrengthService::new();

        // 测试弱密码
        let weak_password = "123";
        let assessment = service.assess_password(weak_password);

        assert!(assessment.score < 30);
        assert!(!assessment.issues.is_empty());
        assert!(assessment.entropy_bits < 20.0);

        // 测试中等密码
        let medium_password = "Password123";
        let assessment = service.assess_password(medium_password);

        assert!(assessment.score >= 30 && assessment.score <= 70);
        assert!(assessment.entropy_bits >= 20.0);

        // 测试强密码
        let strong_password = "StrongP@ssw0rd!2024";
        let assessment = service.assess_password(strong_password);

        assert!(assessment.score > 70);
        assert!(assessment.entropy_bits >= 40.0);
    }

    #[test]
    fn test_batch_assessment() {
        let service = PasswordStrengthService::new();

        let passwords = vec![
            "123",
            "password",
            "Password123",
            "StrongP@ssw0rd!2024",
        ];

        let assessments = service.assess_passwords_batch(&passwords);

        assert_eq!(assessments.len(), 4);
        assert!(assessments[0].score < assessments[1].score);
        assert!(assessments[1].score < assessments[2].score);
        assert!(assessments[2].score < assessments[3].score);
    }

    #[test]
    fn test_password_comparison() {
        let service = PasswordStrengthService::new();

        // 相同密码
        let similarity1 = service.compare_passwords("password", "password");
        assert_eq!(similarity1, 1.0);

        // 相似密码
        let similarity2 = service.compare_passwords("password1", "password2");
        assert!(similarity2 > 0.5 && similarity2 < 1.0);

        // 不同密码
        let similarity3 = service.compare_passwords("password", "admin123");
        assert!(similarity3 < 0.5);

        // 空密码
        let similarity4 = service.compare_passwords("", "password");
        assert_eq!(similarity4, 0.0);
    }

    #[test]
    fn test_policy_compliance() {
        let service = PasswordStrengthService::new();

        // 不符合策略的密码
        let (compliant1, violations1) = service.check_password_policy("123");
        assert!(!compliant1);
        assert!(!violations1.is_empty());

        // 符合策略的密码
        let (compliant2, violations2) = service.check_password_policy("StrongP@ssw0rd!2024");
        assert!(compliant2);
        assert!(violations2.is_empty());
    }

    #[test]
    fn test_strength_report() {
        let service = PasswordStrengthService::new();

        let report = service.generate_strength_report("TestPassword123!");

        assert!(report.contains("密码强度评估报告"));
        assert!(report.contains("强度评分"));
        assert!(report.contains("强度等级"));
        assert!(report.contains("熵值"));
        assert!(report.contains("破解时间"));
    }

    #[test]
    fn test_password_policy() {
        let policy = PasswordPolicy {
            min_length: 12,
            max_length: 64,
            require_lowercase: true,
            require_uppercase: true,
            require_numbers: true,
            require_digits: true,
            require_symbols: true,
            min_entropy_bits: 70.0,
            max_repeated_chars: 2,
            max_sequential_chars: 2,
            check_common_passwords: true,
            check_dictionary_words: true,
            check_keyboard_patterns: true,
            check_date_patterns: true,
        };

        assert_eq!(policy.min_length, 12);
        assert_eq!(policy.max_length, 64);
        assert!(policy.require_lowercase);
        assert!(policy.require_uppercase);
        assert!(policy.require_digits);
        assert!(policy.require_symbols);
        assert_eq!(policy.min_entropy_bits, 70.0);
        assert_eq!(policy.max_repeated_chars, 2);
        assert_eq!(policy.max_sequential_chars, 2);
        assert!(policy.check_common_passwords);
        assert!(policy.check_dictionary_words);
        assert!(policy.check_keyboard_patterns);
        assert!(policy.check_date_patterns);
    }

    #[test]
    fn test_issue_types() {
        // 测试问题类型枚举
        let issue_types = vec![
            PasswordIssueType::TooShort,
            PasswordIssueType::TooLong,
            PasswordIssueType::NoLowercase,
            PasswordIssueType::NoUppercase,
            PasswordIssueType::NoDigits,
            PasswordIssueType::NoSymbols,
            PasswordIssueType::CommonPassword,
            PasswordIssueType::SequentialChars,
            PasswordIssueType::RepeatedChars,
            PasswordIssueType::DictionaryWord,
            PasswordIssueType::PersonalInfo,
            PasswordIssueType::DatePattern,
            PasswordIssueType::KeyboardPattern,
            PasswordIssueType::LeakedPassword,
            PasswordIssueType::WeakPattern,
        ];

        assert_eq!(issue_types.len(), 15);
    }

    #[test]
    fn test_severity_levels() {
        // 测试严重级别枚举
        let severities = vec![
            IssueSeverity::Low,
            IssueSeverity::Medium,
            IssueSeverity::High,
            IssueSeverity::Critical,
        ];

        assert_eq!(severities.len(), 4);
    }

    #[test]
    fn test_crack_time_formatting() {
        let service = PasswordStrengthService::new();

        // 测试不同时间段的格式化
        assert_eq!(service.format_crack_time(0.5), "瞬间");
        assert!(service.format_crack_time(30.0).contains("秒"));
        assert!(service.format_crack_time(1800.0).contains("分钟"));
        assert!(service.format_crack_time(7200.0).contains("小时"));
        assert!(service.format_crack_time(172800.0).contains("天"));
        assert!(service.format_crack_time(63072000.0).contains("年"));
        assert_eq!(service.format_crack_time(31536000000.0), "数百年");
    }

    #[test]
    fn test_levenshtein_distance() {
        let service = PasswordStrengthService::new();

        // 相同字符串
        assert_eq!(service.levenshtein_distance("password", "password"), 0);

        // 一个字符不同
        assert_eq!(service.levenshtein_distance("password", "passw0rd"), 1);

        // 完全不同的字符串
        assert_eq!(service.levenshtein_distance("abc", "xyz"), 3);

        // 空字符串
        assert_eq!(service.levenshtein_distance("", "password"), 8);
        assert_eq!(service.levenshtein_distance("password", ""), 8);
    }

    #[test]
    fn test_entropy_calculation() {
        let service = PasswordStrengthService::new();

        // 简单密码的熵值应该较低
        let weak_assessment = service.assess_password("123456");
        assert!(weak_assessment.entropy_bits < 20.0);

        // 复杂密码的熵值应该较高
        let strong_assessment = service.assess_password("P@ssw0rd!2024#Secure");
        assert!(strong_assessment.entropy_bits > 40.0);
    }
}