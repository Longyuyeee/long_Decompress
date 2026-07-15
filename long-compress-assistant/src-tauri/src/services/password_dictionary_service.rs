use anyhow::{Result, Context, anyhow};
use std::path::Path;
use std::collections::HashMap;

/// 密码字典攻击服务
/// 支持多种策略：常用密码、数字组合、弱密码等
pub struct PasswordDictionaryService {
    dictionaries: HashMap<String, Vec<String>>,
}

impl PasswordDictionaryService {
    pub fn new() -> Self {
        let mut dictionaries = HashMap::new();

        // 1. 常用密码字典（Top 100）
        dictionaries.insert("common".to_string(), vec![
            "123456", "password", "123456789", "12345678", "12345",
            "1234567", "password1", "123123", "1234567890", "000000",
            "abc123", "111111", "qwerty", "1234", "password123",
            "iloveyou", "admin", "welcome", "monkey", "login",
            "letmein", "dragon", "master", "sunshine", "princess",
            "654321", "666666", "123321", "888888", "football",
            "shadow", "michael", "jennifer", "computer", "baseball",
            "superman", "charlie", "qwerty123", "121212", "trustno1",
            "flower", "passw0rd", "1q2w3e4r", "password!", "qwertyuiop",
            "123qwe", "1q2w3e", "123abc", "password1!", "Password1",
            "admin123", "root", "toor", "pass", "test",
            "guest", "123", "1234qwer", "password@123", "P@ssw0rd",
            "welcome123", "admin@123", "root123", "123456a", "a123456",
            "abcd1234", "123456abc", "1qaz2wsx", "zxcvbnm", "asdfghjkl",
            "qazwsx", "1234abcd", "pass123", "Password", "P@ssword",
            "Welcome1", "Admin123", "qwerty12345", "Password123", "12341234",
            "password12", "admin1234", "root1234", "test123", "guest123",
            "user", "user123", "demo", "demo123", "sample",
            "sample123", "temp", "temp123", "password1234", "admin@123456",
            "123456789a", "a123456789", "1234567a", "a1234567", "123456@",
            "!@#$%^&*", "1234!@#$", "qwer1234", "asdf1234", "zxcv1234",
        ].iter().map(|s| s.to_string()).collect());

        // 2. 数字组合（4-8位）
        let mut numeric = Vec::new();
        for i in 0..10000 {
            numeric.push(format!("{:04}", i)); // 0000-9999
        }
        dictionaries.insert("numeric_4digit".to_string(), numeric);

        // 3. 日期格式密码
        let mut dates = Vec::new();
        for year in 1990..=2030 {
            for month in 1..=12 {
                for day in 1..=31 {
                    dates.push(format!("{:04}{:02}{:02}", year, month, day)); // YYYYMMDD
                    dates.push(format!("{:02}{:02}{:04}", day, month, year)); // DDMMYYYY
                    dates.push(format!("{:02}{:02}{:02}", year % 100, month, day)); // YYMMDD
                }
            }
        }
        dictionaries.insert("dates".to_string(), dates);

        // 4. 简单模式
        dictionaries.insert("simple_patterns".to_string(), vec![
            "aaaaaa", "111111", "000000", "123123", "abc123",
            "qwerty", "qwertyuiop", "asdfgh", "zxcvbn",
            "abcdef", "fedcba", "123321", "112233", "121212",
        ].iter().map(|s| s.to_string()).collect());

        // 5. 键盘模式
        dictionaries.insert("keyboard_patterns".to_string(), vec![
            "qwerty", "qwertyui", "asdfghjk", "zxcvbnm", "1qaz2wsx",
            "qazwsx", "qazwsxedc", "1qazxsw2", "qweasd", "qweasdzxc",
        ].iter().map(|s| s.to_string()).collect());

        Self { dictionaries }
    }

    /// 获取指定字典
    pub fn get_dictionary(&self, name: &str) -> Option<&Vec<String>> {
        self.dictionaries.get(name)
    }

    /// 获取所有字典名称
    pub fn list_dictionaries(&self) -> Vec<String> {
        self.dictionaries.keys().cloned().collect()
    }

    /// 获取字典大小
    pub fn dictionary_size(&self, name: &str) -> usize {
        self.dictionaries.get(name).map(|d| d.len()).unwrap_or(0)
    }

    /// 生成自定义字典（基于用户名、文件名等）
    pub fn generate_custom_dictionary(&self, base_words: &[String]) -> Vec<String> {
        let mut passwords = Vec::new();

        for word in base_words {
            let lower = word.to_lowercase();
            let upper = word.to_uppercase();
            let title = {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            };

            // 原始词
            passwords.push(word.clone());
            passwords.push(lower.clone());
            passwords.push(upper.clone());
            passwords.push(title.clone());

            // 添加数字后缀
            for i in 0..100 {
                passwords.push(format!("{}{}", word, i));
                passwords.push(format!("{}{}", lower, i));
                passwords.push(format!("{}{:02}", word, i));
            }

            // 添加年份
            for year in 2010..=2030 {
                passwords.push(format!("{}{}", word, year));
                passwords.push(format!("{}{}", lower, year));
            }

            // 添加常见符号
            passwords.push(format!("{}!", word));
            passwords.push(format!("{}@", word));
            passwords.push(format!("{}#", word));
            passwords.push(format!("{}123", word));
            passwords.push(format!("{}@123", word));
        }

        passwords
    }

    /// 从文件名提取关键词
    pub fn extract_keywords_from_filename(filename: &str) -> Vec<String> {
        let path = Path::new(filename);
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename);

        let mut keywords = Vec::new();

        // 分割常见分隔符
        let separators = [' ', '_', '-', '.', '@', '#'];
        let mut words = vec![stem.to_string()];

        for sep in separators {
            let mut new_words = Vec::new();
            for word in words {
                new_words.extend(word.split(sep).map(|s| s.to_string()));
            }
            words = new_words;
        }

        // 过滤空字符串和太短的词
        keywords.extend(words.into_iter().filter(|w| w.len() >= 3));

        keywords
    }

    /// 合并多个字典
    pub fn merge_dictionaries(&self, dict_names: &[&str]) -> Vec<String> {
        let mut merged = Vec::new();

        for name in dict_names {
            if let Some(dict) = self.get_dictionary(name) {
                merged.extend(dict.clone());
            }
        }

        // 去重
        merged.sort();
        merged.dedup();

        merged
    }

    /// 获取推荐字典策略
    pub fn get_recommended_strategy(&self, filename: Option<&str>) -> Vec<String> {
        let mut passwords = Vec::new();

        // 1. 常用密码（必选）
        if let Some(common) = self.get_dictionary("common") {
            passwords.extend(common.clone());
        }

        // 2. 如果有文件名，生成自定义字典
        if let Some(fname) = filename {
            let keywords = Self::extract_keywords_from_filename(fname);
            let custom = self.generate_custom_dictionary(&keywords);
            passwords.extend(custom);
        }

        // 3. 简单模式
        if let Some(simple) = self.get_dictionary("simple_patterns") {
            passwords.extend(simple.clone());
        }

        // 4. 键盘模式
        if let Some(keyboard) = self.get_dictionary("keyboard_patterns") {
            passwords.extend(keyboard.clone());
        }

        // 去重
        passwords.sort();
        passwords.dedup();

        passwords
    }
}

impl Default for PasswordDictionaryService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_service() {
        let service = PasswordDictionaryService::new();

        // 测试获取字典
        assert!(service.get_dictionary("common").is_some());
        assert!(service.get_dictionary("numeric_4digit").is_some());

        // 测试字典大小
        let common_size = service.dictionary_size("common");
        assert!(common_size > 0, "Common dictionary should not be empty");

        let numeric_size = service.dictionary_size("numeric_4digit");
        assert_eq!(numeric_size, 10000, "Numeric 4-digit dictionary should have 10000 entries");
    }

    #[test]
    fn test_extract_keywords() {
        let keywords = PasswordDictionaryService::extract_keywords_from_filename("project_backup_2024.zip");
        assert!(keywords.contains(&"project".to_string()));
        assert!(keywords.contains(&"backup".to_string()));
        assert!(keywords.contains(&"2024".to_string()));
    }

    #[test]
    fn test_custom_dictionary() {
        let service = PasswordDictionaryService::new();
        let custom = service.generate_custom_dictionary(&vec!["test".to_string()]);

        assert!(custom.contains(&"test".to_string()));
        assert!(custom.contains(&"test123".to_string()));
        assert!(custom.contains(&"test2024".to_string()));
        assert!(custom.contains(&"test!".to_string()));
    }
}
