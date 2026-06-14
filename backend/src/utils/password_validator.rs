use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq)]
pub enum PasswordStrength {
    VeryWeak,
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

impl PasswordStrength {
    /// 密码强度等级中文描述
    pub fn description(&self) -> &'static str {
        match self {
            PasswordStrength::VeryWeak => "非常弱",
            PasswordStrength::Weak => "弱",
            PasswordStrength::Medium => "中等",
            PasswordStrength::Strong => "强",
            PasswordStrength::VeryStrong => "非常强",
        }
    }

    /// 强度分数
    pub fn score(&self) -> u8 {
        match self {
            PasswordStrength::VeryWeak => 0,
            PasswordStrength::Weak => 25,
            PasswordStrength::Medium => 50,
            PasswordStrength::Strong => 75,
            PasswordStrength::VeryStrong => 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PasswordValidationResult {
    pub strength: PasswordStrength,
    pub is_valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub max_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_digit: bool,
    pub require_special: bool,
    pub min_strength: PasswordStrength,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            max_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: true,
            min_strength: PasswordStrength::Medium,
        }
    }
}

static RE_UPPERCASE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Z]").unwrap());
static RE_LOWERCASE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-z]").unwrap());
static RE_DIGIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9]").unwrap());

fn has_special_char(password: &str) -> bool {
    password.chars().any(|c| !c.is_alphanumeric())
}

const COMMON_PASSWORDS: &[&str] = &[
    "password",
    "123456",
    "12345678",
    "qwerty",
    "abc123",
    "letmein",
    "welcome",
    "admin",
    "root",
    "toor",
    "123123",
    "111111",
    "000000",
    "password123",
    "admin123",
];

pub fn validate_password(password: &str) -> PasswordValidationResult {
    let policy = PasswordPolicy::default();
    validate_password_with_policy(password, &policy)
}

pub fn validate_password_with_policy(
    password: &str,
    policy: &PasswordPolicy,
) -> PasswordValidationResult {
    let mut errors = Vec::new();
    let mut score = 0u8;

    if password.len() < policy.min_length {
        errors.push(format!("密码长度至少为 {} 个字符", policy.min_length));
    } else {
        score += 20;
    }

    if password.len() > policy.max_length {
        errors.push(format!("密码长度不能超过 {} 个字符", policy.max_length));
    }

    let has_uppercase = RE_UPPERCASE.is_match(password);
    if policy.require_uppercase && !has_uppercase {
        errors.push("密码必须包含大写字母".to_string());
    } else if has_uppercase {
        score += 20;
    }

    let has_lowercase = RE_LOWERCASE.is_match(password);
    if policy.require_lowercase && !has_lowercase {
        errors.push("密码必须包含小写字母".to_string());
    } else if has_lowercase {
        score += 20;
    }

    let has_digit = RE_DIGIT.is_match(password);
    if policy.require_digit && !has_digit {
        errors.push("密码必须包含数字".to_string());
    } else if has_digit {
        score += 20;
    }

    let has_special = has_special_char(password);
    if policy.require_special && !has_special {
        errors.push("密码必须包含特殊字符".to_string());
    } else if has_special {
        score += 20;
    }

    let lower_password = password.to_lowercase();
    if COMMON_PASSWORDS
        .iter()
        .any(|common| lower_password.contains(common))
    {
        errors.push("密码过于常见，不安全".to_string());
        score = score.saturating_sub(30);
    }

    if has_consecutive_chars(password) {
        score = score.saturating_sub(10);
    }

    if has_repeated_chars(password) {
        score = score.saturating_sub(10);
    }

    if password.len() >= 16 {
        score += 10;
    }
    if has_uppercase && has_lowercase && has_digit && has_special {
        score += 10;
    }

    let strength = match score {
        0..=20 => PasswordStrength::VeryWeak,
        21..=40 => PasswordStrength::Weak,
        41..=60 => PasswordStrength::Medium,
        61..=80 => PasswordStrength::Strong,
        _ => PasswordStrength::VeryStrong,
    };

    let strength_score = strength.score();
    let min_strength_score = policy.min_strength.score();
    if strength_score < min_strength_score {
        errors.push(format!(
            "Password strength insufficient current {} min required {}",
            strength.description(),
            policy.min_strength.description()
        ));
    }

    let is_valid = errors.is_empty();

    PasswordValidationResult {
        strength,
        is_valid,
        errors,
    }
}

fn has_consecutive_chars(password: &str) -> bool {
    let chars: Vec<char> = password.chars().collect();
    if chars.len() < 3 {
        return false;
    }
    for i in 0..chars.len() - 2 {
        let c1 = chars[i] as u8;
        let c2 = chars[i + 1] as u8;
        let c3 = chars[i + 2] as u8;
        if c2 == c1 + 1 && c3 == c2 + 1 {
            return true;
        }
        if c2 == c1 - 1 && c3 == c2 - 1 {
            return true;
        }
    }
    false
}

fn has_repeated_chars(password: &str) -> bool {
    let chars: Vec<char> = password.chars().collect();
    if chars.len() < 3 {
        return false;
    }
    for i in 0..chars.len() - 2 {
        if chars[i] == chars[i + 1] && chars[i + 1] == chars[i + 2] {
            return true;
        }
    }
    false
}

pub fn get_password_feedback(result: &PasswordValidationResult) -> String {
    if result.is_valid {
        format!(
            "Password strength {} {} points meets requirements",
            result.strength.description(),
            result.strength.score()
        )
    } else {
        let error_msg = result.errors.join("; ");
        format!("Password validation failed {}", error_msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strong_password() {
        let result = validate_password("MyP@ssw0rd123!");
        assert!(result.is_valid);
        assert!(result.strength.score() >= 60);
    }

    #[test]
    fn test_weak_password_too_short() {
        let result = validate_password("Ab1!");
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("长度")));
    }

    #[test]
    fn test_weak_password_common() {
        let result = validate_password("password123!");
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("常见")));
    }

    #[test]
    fn test_consecutive_chars() {
        assert!(has_consecutive_chars("abc123"));
        assert!(has_consecutive_chars("321cba"));
        assert!(!has_consecutive_chars("a1b2c3"));
    }

    #[test]
    fn test_repeated_chars() {
        assert!(has_repeated_chars("aaabbb"));
        assert!(has_repeated_chars("111222"));
        assert!(!has_repeated_chars("abcdef"));
    }
}
