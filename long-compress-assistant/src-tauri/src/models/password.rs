use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: String,
    pub name: String,
    pub username: Option<String>,
    pub password: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub category: PasswordCategory,
    pub strength: PasswordStrength,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub favorite: bool,
    pub custom_fields: Vec<CustomField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub name: String,
    pub value: String,
    pub field_type: CustomFieldType,
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CustomFieldType {
    Text,
    Password,
    Email,
    Url,
    Phone,
    Date,
    Number,
    MultilineText,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PasswordCategory {
    Personal,
    Work,
    Finance,
    Social,
    Shopping,
    Entertainment,
    Education,
    Travel,
    Health,
    Other,
}

impl PasswordCategory {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Personal,
            Self::Work,
            Self::Finance,
            Self::Social,
            Self::Shopping,
            Self::Entertainment,
            Self::Education,
            Self::Travel,
            Self::Health,
            Self::Other,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Personal => "个人",
            Self::Work => "工作",
            Self::Finance => "金融",
            Self::Social => "社交",
            Self::Shopping => "购物",
            Self::Entertainment => "娱乐",
            Self::Education => "教育",
            Self::Travel => "旅行",
            Self::Health => "健康",
            Self::Other => "其他",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Personal => "👤",
            Self::Work => "💼",
            Self::Finance => "💰",
            Self::Social => "👥",
            Self::Shopping => "🛒",
            Self::Entertainment => "🎮",
            Self::Education => "📚",
            Self::Travel => "✈️",
            Self::Health => "🏥",
            Self::Other => "📁",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PasswordStrength {
    VeryWeak,
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

impl PasswordStrength {
    pub fn from_score(score: u8) -> Self {
        match score {
            0..=1 => Self::VeryWeak,
            2..=3 => Self::Weak,
            4..=6 => Self::Medium,
            7..=8 => Self::Strong,
            _ => Self::VeryStrong,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::VeryWeak => "非常弱",
            Self::Weak => "弱",
            Self::Medium => "中等",
            Self::Strong => "强",
            Self::VeryStrong => "非常强",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::VeryWeak => "#ff4444",
            Self::Weak => "#ff8800",
            Self::Medium => "#ffbb33",
            Self::Strong => "#00c851",
            Self::VeryStrong => "#007e33",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordGroup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: PasswordCategory,
    pub entry_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordAuditResult {
    pub entry_id: String,
    pub issues: Vec<PasswordIssue>,
    pub score: u8,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordIssue {
    pub issue_type: PasswordIssueType,
    pub severity: IssueSeverity,
    pub description: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PasswordIssueType {
    WeakPassword,
    ReusedPassword,
    OldPassword,
    NoTwoFactor,
    ExposedInBreach,
    MissingUsername,
    MissingUrl,
    ExpiredPassword,
    NoSpecialChars,
    TooShort,
    NoNumbers,
    NoUppercase,
    NoLowercase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl Default for PasswordGeneratorOptions {
    fn default() -> Self {
        Self {
            length: 16,
            include_uppercase: true,
            include_lowercase: true,
            include_numbers: true,
            include_symbols: true,
            exclude_similar: true,
            exclude_ambiguous: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordImportExportOptions {
    pub format: ImportExportFormat,
    pub include_passwords: bool,
    pub include_metadata: bool,
    pub encrypt: bool,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImportExportFormat {
    Json,
    Csv,
    Xml,
    KeePass,
    LastPass,
    Bitwarden,
}

impl PasswordEntry {
    pub fn new(
        name: String,
        username: Option<String>,
        password: String,
        url: Option<String>,
        category: PasswordCategory,
    ) -> Self {
        let now = Utc::now();
        let strength = Self::evaluate_password_strength(&password);

        Self {
            id: Uuid::new_v4().to_string(),
            name,
            username,
            password,
            url,
            notes: None,
            tags: Vec::new(),
            category,
            strength,
            created_at: now,
            updated_at: now,
            last_used: None,
            expires_at: None,
            favorite: false,
            custom_fields: Vec::new(),
        }
    }

    pub fn evaluate_password_strength(password: &str) -> PasswordStrength {
        let mut score = 0;

        // 长度评分
        if password.len() >= 8 {
            score += 1;
        }
        if password.len() >= 12 {
            score += 1;
        }
        if password.len() >= 16 {
            score += 1;
        }

        // 字符类型评分
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_digit = password.chars().any(|c| c.is_digit(10));
        let has_symbol = password.chars().any(|c| !c.is_alphanumeric());

        if has_lowercase { score += 1; }
        if has_uppercase { score += 1; }
        if has_digit { score += 1; }
        if has_symbol { score += 1; }

        // 避免常见模式
        let common_passwords = ["password", "123456", "qwerty", "admin", "welcome"];
        if common_passwords.iter().any(|&p| password.to_lowercase().contains(p)) {
            score = score.saturating_sub(2);
        }

        PasswordStrength::from_score(score)
    }

    pub fn update_password(&mut self, new_password: String) {
        self.password = new_password;
        self.strength = Self::evaluate_password_strength(&self.password);
        self.updated_at = Utc::now();
    }

    pub fn mark_as_used(&mut self) {
        self.last_used = Some(Utc::now());
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
        self.updated_at = Utc::now();
    }

    pub fn add_custom_field(&mut self, field: CustomField) {
        self.custom_fields.push(field);
        self.updated_at = Utc::now();
    }

    pub fn remove_custom_field(&mut self, field_name: &str) {
        self.custom_fields.retain(|f| f.name != field_name);
        self.updated_at = Utc::now();
    }
}

impl PasswordGroup {
    pub fn new(name: String, description: Option<String>, category: PasswordCategory) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            category,
            entry_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_entry(&mut self, entry_id: String) {
        if !self.entry_ids.contains(&entry_id) {
            self.entry_ids.push(entry_id);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_entry(&mut self, entry_id: &str) {
        self.entry_ids.retain(|id| id != entry_id);
        self.updated_at = Utc::now();
    }
}