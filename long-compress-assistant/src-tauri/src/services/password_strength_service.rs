use serde::{Deserialize, Serialize};
use crate::models::password::{PasswordStrength};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordAuditResult {
    pub entry_id: String,
    pub score: u8,
    pub issues: Vec<PasswordIssue>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordIssue {
    pub issue_type: PasswordIssueType,
    pub severity: IssueSeverity,
    pub description: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PasswordIssueType {
    WeakPassword,
    ReusedPassword,
    ExpiredPassword,
    OldPassword,
    MissingUsername,
    MissingUrl,
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

impl std::fmt::Display for PasswordIssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordGeneratorOptions {
    pub length: usize,
    pub include_uppercase: bool,
    pub include_lowercase: bool,
    pub include_numbers: bool,
    pub include_symbols: bool,
    pub exclude_similar: bool,
    pub exclude_ambiguous: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordImportExportOptions {
    pub format: ImportExportFormat,
    pub encrypt: bool,
    pub include_passwords: bool,
    pub include_metadata: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ImportExportFormat {
    Json,
    Csv,
    KeePass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub max_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_numbers: bool,
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
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_digits: true,
            require_symbols: false,
            min_entropy_bits: 40.0,
            max_repeated_chars: 3,
            max_sequential_chars: 3,
            check_common_passwords: true,
            check_dictionary_words: true,
            check_keyboard_patterns: true,
            check_date_patterns: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PasswordPolicyDb {
    pub id: String,
    pub name: String,
    pub min_length: i32,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_numbers: bool,
    pub require_symbols: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordStrengthAssessment {
    pub score: u8,
    pub strength: PasswordStrength,
    pub entropy: f64,
    pub crack_time_seconds: f64,
    pub issues: Vec<PasswordIssue>,
    pub recommendations: Vec<String>,
}

pub struct PasswordStrengthService;

impl PasswordStrengthService {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(&self, password: &str) -> PasswordStrength {
        let mut score: u8 = 0;
        if password.len() >= 8 { score += 1; }
        if password.len() >= 12 { score += 1; }
        if password.chars().any(|c| c.is_uppercase()) && password.chars().any(|c| c.is_lowercase()) { score += 1; }
        if password.chars().any(|c| c.is_numeric()) { score += 1; }
        if password.chars().any(|c| !c.is_alphanumeric()) { score += 1; }

        PasswordStrength::from_score(score)
    }

    pub fn assess_password(&self, password: &str) -> PasswordStrengthAssessment {
        let strength = self.evaluate(password);
        PasswordStrengthAssessment {
            score: 3,
            strength,
            entropy: 40.0,
            crack_time_seconds: 3600.0,
            issues: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn assess_passwords_batch(&self, passwords: &[&str]) -> Vec<PasswordStrengthAssessment> {
        passwords.iter().map(|p| self.assess_password(p)).collect()
    }

    pub fn compare_passwords(&self, _p1: &str, _p2: &str) -> f32 {
        0.0
    }

    pub fn check_password_policy(&self, password: &str) -> (bool, Vec<String>) {
        (password.len() >= 8, Vec::new())
    }

    pub fn generate_strength_report(&self, password: &str) -> String {
        format!("密码强度: {:?}", self.evaluate(password))
    }
}

impl Default for PasswordStrengthService {
    fn default() -> Self {
        Self::new()
    }
}
