use anyhow::Result;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

/// 密码生成器服务
/// 生成安全、随机的密码
pub struct PasswordGeneratorService;

/// 密码强度级别
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PasswordStrength {
    /// 弱 (8-10 字符, 仅字母数字)
    Weak,
    /// 中等 (12-16 字符, 字母数字+符号)
    Medium,
    /// 强 (16-20 字符, 全字符集)
    Strong,
    /// 超强 (24-32 字符, 全字符集)
    VeryStrong,
}

/// 密码字符集选项
#[derive(Debug, Clone)]
pub struct CharsetOptions {
    pub lowercase: bool,
    pub uppercase: bool,
    pub numbers: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool, // 排除易混淆字符 (0, O, l, 1, I)
}

impl Default for CharsetOptions {
    fn default() -> Self {
        Self {
            lowercase: true,
            uppercase: true,
            numbers: true,
            symbols: true,
            exclude_ambiguous: true,
        }
    }
}

impl PasswordGeneratorService {
    /// 生成指定强度的密码
    pub fn generate(strength: PasswordStrength) -> String {
        match strength {
            PasswordStrength::Weak => Self::generate_custom(10, CharsetOptions {
                lowercase: true,
                uppercase: true,
                numbers: true,
                symbols: false,
                exclude_ambiguous: true,
            }),
            PasswordStrength::Medium => Self::generate_custom(14, CharsetOptions::default()),
            PasswordStrength::Strong => Self::generate_custom(18, CharsetOptions::default()),
            PasswordStrength::VeryStrong => Self::generate_custom(28, CharsetOptions::default()),
        }
    }

    /// 生成自定义密码
    pub fn generate_custom(length: usize, options: CharsetOptions) -> String {
        let charset = Self::build_charset(&options);

        if charset.is_empty() {
            return String::new();
        }

        let mut rng = thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset[idx]
            })
            .collect()
    }

    /// 生成易记密码 (单词组合)
    pub fn generate_memorable(word_count: usize, separator: &str) -> String {
        let words = vec![
            "dragon", "tiger", "eagle", "phoenix", "warrior", "magic", "crystal",
            "thunder", "shadow", "light", "storm", "blade", "shield", "crown",
            "castle", "forest", "ocean", "mountain", "river", "valley", "sunset",
            "sunrise", "moon", "star", "cloud", "wind", "fire", "water", "earth",
            "silver", "golden", "ruby", "emerald", "diamond", "pearl", "sapphire",
        ];

        let mut rng = thread_rng();
        let mut result = Vec::new();

        for _ in 0..word_count {
            let word = words[rng.gen_range(0..words.len())];
            let capitalized = Self::capitalize_first(word);
            result.push(capitalized);
        }

        // 添加随机数字
        let number: u16 = rng.gen_range(10..9999);
        result.push(number.to_string());

        result.join(separator)
    }

    /// 生成 PIN 码
    pub fn generate_pin(length: usize) -> String {
        let mut rng = thread_rng();
        (0..length)
            .map(|_| rng.gen_range(0..10).to_string())
            .collect()
    }

    /// 生成十六进制密码
    pub fn generate_hex(length: usize) -> String {
        let mut rng = thread_rng();
        let bytes: Vec<u8> = (0..length).map(|_| rng.gen()).collect();
        hex::encode(bytes)
    }

    /// 评估密码强度
    pub fn evaluate_strength(password: &str) -> (PasswordStrength, u8) {
        let len = password.len();
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_digit = password.chars().any(|c| c.is_numeric());
        let has_symbol = password.chars().any(|c| !c.is_alphanumeric());

        let mut score = 0u8;

        // 长度评分
        score += match len {
            0..=7 => 0,
            8..=11 => 10,
            12..=15 => 20,
            16..=19 => 30,
            _ => 40,
        };

        // 字符集多样性评分
        if has_lower { score += 10; }
        if has_upper { score += 10; }
        if has_digit { score += 10; }
        if has_symbol { score += 20; }

        // 无重复字符奖励
        let unique_chars: std::collections::HashSet<char> = password.chars().collect();
        if unique_chars.len() as f32 / len as f32 > 0.8 {
            score += 10;
        }

        let strength = match score {
            0..=30 => PasswordStrength::Weak,
            31..=60 => PasswordStrength::Medium,
            61..=85 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        };

        (strength, score)
    }

    /// 批量生成密码
    pub fn generate_batch(count: usize, strength: PasswordStrength) -> Vec<String> {
        (0..count).map(|_| Self::generate(strength)).collect()
    }

    // === 私有辅助方法 ===

    fn build_charset(options: &CharsetOptions) -> Vec<char> {
        let mut charset = Vec::new();

        if options.lowercase {
            charset.extend('a'..='z');
        }
        if options.uppercase {
            charset.extend('A'..='Z');
        }
        if options.numbers {
            charset.extend('0'..='9');
        }
        if options.symbols {
            charset.extend("!@#$%^&*()_+-=[]{}|;:,.<>?".chars());
        }

        if options.exclude_ambiguous {
            charset.retain(|&c| !matches!(c, '0' | 'O' | 'o' | 'l' | '1' | 'I'));
        }

        charset
    }

    fn capitalize_first(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_weak() {
        let password = PasswordGeneratorService::generate(PasswordStrength::Weak);
        assert_eq!(password.len(), 10);
        assert!(password.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_generate_strong() {
        let password = PasswordGeneratorService::generate(PasswordStrength::Strong);
        assert_eq!(password.len(), 18);
    }

    #[test]
    fn test_generate_memorable() {
        let password = PasswordGeneratorService::generate_memorable(3, "-");
        assert!(password.contains('-'));
        assert!(password.chars().any(|c| c.is_numeric()));
    }

    #[test]
    fn test_generate_pin() {
        let pin = PasswordGeneratorService::generate_pin(6);
        assert_eq!(pin.len(), 6);
        assert!(pin.chars().all(|c| c.is_numeric()));
    }

    #[test]
    fn test_evaluate_strength() {
        let (strength, score) = PasswordGeneratorService::evaluate_strength("Pass123!");
        assert_eq!(strength, PasswordStrength::Weak);
        assert!(score > 0);

        let (strength, score) = PasswordGeneratorService::evaluate_strength("MyV3ry$tr0ng#P@ssw0rd!");
        assert!(matches!(strength, PasswordStrength::Strong | PasswordStrength::VeryStrong));
        assert!(score >= 61);
    }

    #[test]
    fn test_generate_batch() {
        let passwords = PasswordGeneratorService::generate_batch(5, PasswordStrength::Medium);
        assert_eq!(passwords.len(), 5);

        // 确保每个密码都不同
        let unique: std::collections::HashSet<_> = passwords.iter().collect();
        assert_eq!(unique.len(), 5);
    }
}
