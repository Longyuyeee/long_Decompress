use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use regex::Regex;
use zxcvbn::zxcvbn;

use crate::models::password::PasswordStrength;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordStrengthAssessment {
    pub score: u8, // 0-100分
    pub strength: PasswordStrength,
    pub issues: Vec<PasswordIssue>,
    pub recommendations: Vec<String>,
    pub entropy_bits: f64,
    pub crack_time_seconds: f64,
    pub crack_time_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordIssue {
    pub issue_type: PasswordIssueType,
    pub severity: IssueSeverity,
    pub description: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PasswordIssueType {
    TooShort,
    TooLong,
    NoLowercase,
    NoUppercase,
    NoDigits,
    NoSymbols,
    CommonPassword,
    SequentialChars,
    RepeatedChars,
    DictionaryWord,
    PersonalInfo,
    DatePattern,
    KeyboardPattern,
    LeakedPassword,
    WeakPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub max_length: usize,
    pub require_lowercase: bool,
    pub require_uppercase: bool,
    pub require_digits: bool,
    pub require_symbols: bool,
    pub min_entropy_bits: f64,
    pub max_repeated_chars: usize,
    pub max_sequential_chars: usize,
    pub check_common_passwords: bool,
    pub check_dictionary_words: bool,
    pub check_keyboard_patterns: bool,
    pub check_date_patterns: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            max_length: 128,
            require_lowercase: true,
            require_uppercase: true,
            require_digits: true,
            require_symbols: true,
            min_entropy_bits: 60.0,
            max_repeated_chars: 3,
            max_sequential_chars: 3,
            check_common_passwords: true,
            check_dictionary_words: true,
            check_keyboard_patterns: true,
            check_date_patterns: true,
        }
    }
}

pub struct PasswordStrengthService {
    policy: PasswordPolicy,
    common_passwords: HashSet<String>,
    dictionary_words: HashSet<String>,
}

impl PasswordStrengthService {
    pub fn new() -> Self {
        let common_passwords = Self::load_common_passwords();
        let dictionary_words = Self::load_dictionary_words();

        Self {
            policy: PasswordPolicy::default(),
            common_passwords,
            dictionary_words,
        }
    }

    pub fn with_policy(policy: PasswordPolicy) -> Self {
        let common_passwords = Self::load_common_passwords();
        let dictionary_words = Self::load_dictionary_words();

        Self {
            policy,
            common_passwords,
            dictionary_words,
        }
    }

    /// 评估密码强度
    pub fn assess_password(&self, password: &str) -> PasswordStrengthAssessment {
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // 基本检查
        self.check_length(password, &mut issues, &mut recommendations);
        self.check_character_types(password, &mut issues, &mut recommendations);
        self.check_patterns(password, &mut issues, &mut recommendations);

        // 使用zxcvbn进行高级评估
        let zxcvbn_result = zxcvbn(password, &[]).unwrap_or_else(|_| {
            zxcvbn::Entropy::new(0.0, 0.0, 0.0, vec![])
        });

        let entropy_bits = zxcvbn_result.guesses_log10() * 3.3219280948873626; // log2(10) ≈ 3.3219
        let crack_time_seconds = self.calculate_crack_time(entropy_bits);
        let crack_time_display = self.format_crack_time(crack_time_seconds);

        // 计算综合分数 (0-100)
        let mut score = self.calculate_score(password, &issues, entropy_bits);

        // 确保分数在0-100范围内
        score = score.clamp(0, 100);

        // 转换为PasswordStrength枚举
        let strength = match score {
            0..=20 => PasswordStrength::VeryWeak,
            21..=40 => PasswordStrength::Weak,
            41..=60 => PasswordStrength::Medium,
            61..=80 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        };

        PasswordStrengthAssessment {
            score,
            strength,
            issues,
            recommendations,
            entropy_bits,
            crack_time_seconds,
            crack_time_display,
        }
    }

    /// 批量评估密码强度
    pub fn assess_passwords_batch(&self, passwords: &[&str]) -> Vec<PasswordStrengthAssessment> {
        passwords.iter()
            .map(|password| self.assess_password(password))
            .collect()
    }

    /// 比较两个密码的相似度
    pub fn compare_passwords(&self, password1: &str, password2: &str) -> f64 {
        if password1.is_empty() || password2.is_empty() {
            return 0.0;
        }

        let len1 = password1.len();
        let len2 = password2.len();
        let max_len = len1.max(len2) as f64;

        // 计算编辑距离
        let distance = self.levenshtein_distance(password1, password2) as f64;

        // 相似度 = 1 - (编辑距离 / 最大长度)
        1.0 - (distance / max_len)
    }

    /// 检查密码是否符合策略
    pub fn check_password_policy(&self, password: &str) -> (bool, Vec<String>) {
        let assessment = self.assess_password(password);
        let mut violations = Vec::new();

        for issue in &assessment.issues {
            if matches!(issue.severity, IssueSeverity::High | IssueSeverity::Critical) {
                violations.push(format!("{}: {}", issue.issue_type, issue.description));
            }
        }

        let is_compliant = violations.is_empty() && assessment.score >= 60;
        (is_compliant, violations)
    }

    /// 生成密码强度报告
    pub fn generate_strength_report(&self, password: &str) -> String {
        let assessment = self.assess_password(password);

        let mut report = String::new();
        report.push_str(&format!("密码强度评估报告\n"));
        report.push_str(&format!("==================\n"));
        report.push_str(&format!("密码: {}\n", "*".repeat(password.len().min(20))));
        report.push_str(&format!("强度评分: {}/100\n", assessment.score));
        report.push_str(&format!("强度等级: {:?}\n", assessment.strength));
        report.push_str(&format!("熵值: {:.2} bits\n", assessment.entropy_bits));
        report.push_str(&format!("破解时间: {}\n", assessment.crack_time_display));

        if !assessment.issues.is_empty() {
            report.push_str("\n发现的问题:\n");
            for issue in &assessment.issues {
                report.push_str(&format!("  - [{}] {}: {}\n",
                    match issue.severity {
                        IssueSeverity::Low => "低",
                        IssueSeverity::Medium => "中",
                        IssueSeverity::High => "高",
                        IssueSeverity::Critical => "严重",
                    },
                    issue.issue_type,
                    issue.description
                ));
            }
        }

        if !assessment.recommendations.is_empty() {
            report.push_str("\n改进建议:\n");
            for recommendation in &assessment.recommendations {
                report.push_str(&format!("  - {}\n", recommendation));
            }
        }

        report
    }

    // ========== 私有方法 ==========

    fn load_common_passwords() -> HashSet<String> {
        let common = vec![
            "password", "123456", "12345678", "123456789", "1234567890",
            "qwerty", "abc123", "password1", "12345", "1234567",
            "admin", "welcome", "monkey", "letmein", "dragon",
            "football", "baseball", "mustang", "superman", "hello",
            "master", "sunshine", "password123", "trustno1", "princess",
            "admin123", "welcome123", "login", "passw0rd", "abc123456",
        ];

        common.into_iter().map(String::from).collect()
    }

    fn load_dictionary_words() -> HashSet<String> {
        // 这里可以加载更大的字典文件
        // 暂时使用一些常见单词
        let words = vec![
            "hello", "world", "password", "admin", "user",
            "test", "guest", "access", "login", "secret",
            "private", "public", "system", "network", "server",
            "database", "application", "software", "hardware", "computer",
        ];

        words.into_iter().map(String::from).collect()
    }

    fn check_length(&self, password: &str, issues: &mut Vec<PasswordIssue>, recommendations: &mut Vec<String>) {
        let length = password.len();

        if length < self.policy.min_length {
            issues.push(PasswordIssue {
                issue_type: PasswordIssueType::TooShort,
                severity: IssueSeverity::Critical,
                description: format!("密码长度只有 {} 个字符，至少需要 {} 个字符", length, self.policy.min_length),
                suggestion: format!("增加密码长度到至少 {} 个字符", self.policy.min_length),
            });
            recommendations.push(format!("增加密码长度到至少 {} 个字符", self.policy.min_length));
        }

        if length > self.policy.max_length {
            issues.push(PasswordIssue {
                issue_type: PasswordIssueType::TooLong,
                severity: IssueSeverity::Medium,
                description: format!("密码长度 {} 个字符，超过最大限制 {} 个字符", length, self.policy.max_length),
                suggestion: format!("缩短密码长度到最多 {} 个字符", self.policy.max_length),
            });
        }

        if length < 12 {
            recommendations.push("考虑使用至少12个字符的密码".to_string());
        }
    }

    fn check_character_types(&self, password: &str, issues: &mut Vec<PasswordIssue>, recommendations: &mut Vec<String>) {
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_digits = password.chars().any(|c| c.is_digit(10));
        let has_symbols = password.chars().any(|c| !c.is_alphanumeric());

        if self.policy.require_lowercase && !has_lowercase {
            issues.push(PasswordIssue {
                issue_type: PasswordIssueType::NoLowercase,
                severity: IssueSeverity::High,
                description: "密码不包含小写字母".to_string(),
                suggestion: "添加至少一个小写字母".to_string(),
            });
            recommendations.push("添加小写字母以增加复杂性".to_string());
        }

        if self.policy.require_uppercase && !has_uppercase {
            issues.push(PasswordIssue {
                issue_type: PasswordIssueType::NoUppercase,
                severity: IssueSeverity::High,
                description: "密码不包含大写字母".to_string(),
                suggestion: "添加至少一个大写字母".to_string(),
            });
            recommendations.push("添加大写字母以增加复杂性".to_string());
        }

        if self.policy.require_digits && !has_digits {
            issues.push(PasswordIssue {
                issue_type: PasswordIssueType::NoDigits,
                severity: IssueSeverity::High,
                description: "密码不包含数字".to_string(),
                suggestion: "添加至少一个数字".to_string(),
            });
            recommendations.push("添加数字以增加复杂性".to_string());
        }

        if self.policy.require_symbols && !has_symbols {
            issues.push(PasswordIssue {
                issue_type: PasswordIssueType::NoSymbols,
                severity: IssueSeverity::High,
                description: "密码不包含特殊符号".to_string(),
                suggestion: "添加至少一个特殊符号 (!@#$%^&*等)".to_string(),
            });
            recommendations.push("添加特殊符号以增加复杂性".to_string());
        }
    }

    fn check_patterns(&self, password: &str, issues: &mut Vec<PasswordIssue>, recommendations: &mut Vec<String>) {
        let password_lower = password.to_lowercase();

        // 检查常见密码
        if self.policy.check_common_passwords {
            for common_pass in &self.common_passwords {
                if password_lower.contains(common_pass) {
                    issues.push(PasswordIssue {
                        issue_type: PasswordIssueType::CommonPassword,
                        severity: IssueSeverity::Critical,
                        description: format!("密码包含常见密码模式: {}", common_pass),
                        suggestion: "避免使用常见密码".to_string(),
                    });
                    recommendations.push("避免使用常见密码和模式".to_string());
                    break;
                }
            }
        }

        // 检查字典单词
        if self.policy.check_dictionary_words {
            for word in &self.dictionary_words {
                if password_lower.contains(word) {
                    issues.push(PasswordIssue {
                        issue_type: PasswordIssueType::DictionaryWord,
                        severity: IssueSeverity::High,
                        description: format!("密码包含字典单词: {}", word),
                        suggestion: "避免使用完整的字典单词".to_string(),
                    });
                    recommendations.push("避免使用完整的字典单词，可以添加数字或符号".to_string());
                    break;
                }
            }
        }

        // 检查重复字符
        if self.policy.max_repeated_chars > 0 {
            let mut current_char = None;
            let mut repeat_count = 0;

            for c in password.chars() {
                if Some(c) == current_char {
                    repeat_count += 1;
                    if repeat_count > self.policy.max_repeated_chars {
                        issues.push(PasswordIssue {
                            issue_type: PasswordIssueType::RepeatedChars,
                            severity: IssueSeverity::Medium,
                            description: format!("密码包含 {} 个连续重复的字符 '{}'", repeat_count + 1, c),
                            suggestion: "避免连续重复相同的字符".to_string(),
                        });
                        recommendations.push("避免连续重复相同的字符".to_string());
                        break;
                    }
                } else {
                    current_char = Some(c);
                    repeat_count = 0;
                }
            }
        }

        // 检查序列字符
        if self.policy.max_sequential_chars > 0 {
            let chars: Vec<char> = password.chars().collect();
            for i in 0..chars.len().saturating_sub(self.policy.max_sequential_chars) {
                let mut is_sequential = true;
                for j in 0..self.policy.max_sequential_chars {
                    let current = chars[i + j] as u32;
                    let next = chars.get(i + j + 1).map(|c| *c as u32);

                    if let Some(next_val) = next {
                        if next_val != current + 1 && next_val != current - 1 {
                            is_sequential = false;
                            break;
                        }
                    }
                }

                if is_sequential {
                    issues.push(PasswordIssue {
                        issue_type: PasswordIssueType::SequentialChars,
                        severity: IssueSeverity::Medium,
                        description: "密码包含连续的字符序列".to_string(),
                        suggestion: "避免使用连续的字符序列".to_string(),
                    });
                    recommendations.push("避免使用连续的字符序列（如abc、123）".to_string());
                    break;
                }
            }
        }

        // 检查键盘模式
        if self.policy.check_keyboard_patterns {
            let keyboard_patterns = vec!["qwerty", "asdfgh", "zxcvbn", "123456", "!@#$%^"];
            for pattern in keyboard_patterns {
                if password_lower.contains(pattern) {
                    issues.push(PasswordIssue {
                        issue_type: PasswordIssueType::KeyboardPattern,
                        severity: IssueSeverity::High,
                        description: format!("密码包含键盘模式: {}", pattern),
                        suggestion: "避免使用键盘上的连续按键".to_string(),
                    });
                    recommendations.push("避免使用键盘上的连续按键模式".to_string());
                    break;
                }
            }
        }

        // 检查日期模式
        if self.policy.check_date_patterns {
            let date_regex = Regex::new(r"\d{4}[-\/]\d{1,2}[-\/]\d{1,2}|\d{1,2}[-\/]\d{1,2}[-\/]\d{4}|\d{6,8}").unwrap();
            if date_regex.is_match(password) {
                issues.push(PasswordIssue {
                    issue_type: PasswordIssueType::DatePattern,
                    severity: IssueSeverity::Medium,
                    description: "密码包含日期模式".to_string(),
                    suggestion: "避免使用日期作为密码".to_string(),
                });
                recommendations.push("避免使用日期作为密码的一部分".to_string());
            }
        }
    }

    fn calculate_score(&self, password: &str, issues: &[PasswordIssue], entropy_bits: f64) -> u8 {
        let mut score = 50.0; // 基础分

        // 长度加分
        let length = password.len() as f64;
        if length >= 8.0 {
            score += (length - 8.0).min(20.0); // 最多加20分
        }

        // 字符类型加分
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_digits = password.chars().any(|c| c.is_digit(10));
        let has_symbols = password.chars().any(|c| !c.is_alphanumeric());

        if has_lowercase { score += 5.0; }
        if has_uppercase { score += 5.0; }
        if has_digits { score += 5.0; }
        if has_symbols { score += 10.0; }

        // 熵值加分
        if entropy_bits >= 60.0 {
            score += 20.0;
        } else if entropy_bits >= 40.0 {
            score += 10.0;
        } else if entropy_bits >= 20.0 {
            score += 5.0;
        }

        // 问题扣分
        for issue in issues {
            let penalty = match issue.severity {
                IssueSeverity::Low => 2.0,
                IssueSeverity::Medium => 5.0,
                IssueSeverity::High => 10.0,
                IssueSeverity::Critical => 20.0,
            };
            score -= penalty;
        }

        // 确保分数在0-100范围内
        score = score.max(0.0).min(100.0);

        score as u8
    }

    fn calculate_crack_time(&self, entropy_bits: f64) -> f64 {
        // 假设攻击者每秒尝试10^9次（10亿次）
        let guesses_per_second = 1_000_000_000.0;
        let total_guesses = 2_f64.powf(entropy_bits);

        total_guesses / guesses_per_second
    }

    fn format_crack_time(&self, seconds: f64) -> String {
        if seconds < 1.0 {
            "瞬间".to_string()
        } else if seconds < 60.0 {
            format!("{:.1} 秒", seconds)
        } else if seconds < 3600.0 {
            format!("{:.1} 分钟", seconds / 60.0)
        } else if seconds < 86400.0 {
            format!("{:.1} 小时", seconds / 3600.0)
        } else if seconds < 31536000.0 {
            format!("{:.1} 天", seconds / 86400.0)
        } else if seconds < 3153600000.0 {
            format!("{:.1} 年", seconds / 31536000.0)
        } else {
            "数百年".to_string()
        }
    }

    fn levenshtein_distance(&self, a: &str, b: &str) -> usize {
        let a_len = a.chars().count();
        let b_len = b.chars().count();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        let mut prev_row: Vec<usize> = (0..=b_len).collect();
        let mut curr_row = vec![0; b_len + 1];

        for (i, a_char) in a.chars().enumerate() {
            curr_row[0] = i + 1;

            for (j, b_char) in b.chars().enumerate() {
                let cost = if a_char == b_char { 0 } else { 1 };

                curr_row[j + 1] = (prev_row[j + 1] + 1)
                    .min(curr_row[j] + 1)
                    .min(prev_row[j] + cost);
            }

            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        prev_row[b_len]
    }
}

impl Default for PasswordStrengthService {
    fn default() -> Self {
        Self::new()
    }
}