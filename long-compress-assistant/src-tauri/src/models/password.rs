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
    pub use_count: u32,
    pub custom_fields: Vec<CustomField>,
}

impl PasswordEntry {
    pub fn new(name: String, password: String, category: PasswordCategory) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            username: None,
            password,
            url: None,
            notes: None,
            tags: Vec::new(),
            category,
            strength: PasswordStrength::Medium,
            created_at: now,
            updated_at: now,
            last_used: None,
            expires_at: None,
            favorite: false,
            use_count: 0,
            custom_fields: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub name: String,
    pub value: String,
    pub field_type: CustomFieldType,
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl std::fmt::Display for PasswordCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PasswordStrength {
    VeryWeak,
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

impl std::fmt::Display for PasswordStrength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordGroup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub entry_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PasswordGroup {
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            entry_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_entry(&mut self, entry_id: String) {
        if !self.entry_ids.contains(&entry_id) {
            self.entry_ids.push(entry_id);
        }
    }

    pub fn remove_entry(&mut self, entry_id: &str) {
        self.entry_ids.retain(|id| id != entry_id);
    }
}
