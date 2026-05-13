#![allow(dead_code)]

use regex::Regex;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, PartialEq)]
pub enum PasswordStrength {
    VeryWeak,
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

impl PasswordStrength {
    pub fn description(&self) -> &'static str {
        match self {
            PasswordStrength::VeryWeak => "Very Weak",
            PasswordStrength::Weak => "Weak",
            PasswordStrength::Medium => "Medium",
            PasswordStrength::Strong => "Strong",
            PasswordStrength::VeryStrong => "Very Strong",
        }
    }

    pub fn is_acceptable(&self) -> bool {
        matches!(self, PasswordStrength::Medium | PasswordStrength::Strong | PasswordStrength::VeryStrong)
    }

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
    pub suggestions: Vec<String>,
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

impl PasswordPolicy {
    pub fn lenient() -> Self {
        Self {
            min_length: 6,
            max_length: 128,
            require_uppercase: false,
            require_lowercase: true,
            require_digit: true,
            require_special: false,
            min_strength: PasswordStrength::Weak,
        }
    }

    pub fn strict() -> Self {
        Self {
            min_length: 12,
            max_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: true,
            min_strength: PasswordStrength::Strong,
        }
    }
}

static RE_UPPERCASE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[A-Z]").unwrap());
static RE_LOWERCASE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[a-z]").unwrap());
static RE_DIGIT: Lazy<Regex> = Lazy::new(|| Regex::new(r"[0-9]").unwrap());

fn has_special_char(password: &str) -> bool {
    password.chars().any(|c| !c.is_alphanumeric())
}

const COMMON_PASSWORDS: &[&str] = &[
    "password", "123456", "12345678", "qwerty", "abc123",
    "letmein", "welcome", "admin", "root", "toor",
    "123123", "111111", "000000", "password123", "admin123",
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
    let mut suggestions = Vec::new();
    let mut score = 0u8;

    if password.len() < policy.min_length {
        errors.push(format!("Password must be at least {} chars", policy.min_length));
    } else {
        score += 20;
    }

    if password.len() > policy.max_length {
        errors.push(format!("Password must not exceed {} chars", policy.max_length));
    }

    let has_uppercase = RE_UPPERCASE.is_match(password);
    if policy.require_uppercase && !has_uppercase {
        errors.push("Password must contain uppercase".to_string());
        suggestions.push("Add uppercase A-Z".to_string());
    } else if has_uppercase {
        score += 20;
    }

    let has_lowercase = RE_LOWERCASE.is_match(password);
    if policy.require_lowercase && !has_lowercase {
        errors.push("Password must contain lowercase".to_string());
        suggestions.push("Add lowercase a-z".to_string());
    } else if has_lowercase {
        score += 20;
    }

    let has_digit = RE_DIGIT.is_match(password);
    if policy.require_digit && !has_digit {
        errors.push("Password must contain digit".to_string());
        suggestions.push("Add digits 0-9".to_string());
    } else if has_digit {
        score += 20;
    }

    let has_special = has_special_char(password);
    if policy.require_special && !has_special {
        errors.push("Password must contain special char".to_string());
        suggestions.push("Add special chars !@#$$%".to_string());
    } else if has_special {
        score += 20;
    }

    let lower_password = password.to_lowercase();
    if COMMON_PASSWORDS.iter().any(|common| lower_password.contains(common)) {
        errors.push("Password is too common".to_string());
        suggestions.push("Use more unique password".to_string());
        score = score.saturating_sub(30);
    }

    if has_consecutive_chars(password) {
        suggestions.push("Avoid consecutive abc 123".to_string());
        score = score.saturating_sub(10);
    }

    if has_repeated_chars(password) {
        suggestions.push("Avoid repeated aaa 111".to_string());
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
        suggestions,
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
        assert!(result.errors.iter().any(|e| e.contains("at least")));
    }

    #[test]
    fn test_weak_password_common() {
        let result = validate_password("password123!");
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("common")));
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
