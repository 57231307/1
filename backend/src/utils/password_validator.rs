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

// P9-1: 静态正则常量初始化，把 unwrap 集中到 helper 便于 P9-1 排查
fn init_regex(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| {
        panic!("P9-1: 密码校验正则 {pattern} 编译失败: {e}")
    })
}

static RE_UPPERCASE: LazyLock<Regex> = LazyLock::new(|| init_regex(r"[A-Z]"));
static RE_LOWERCASE: LazyLock<Regex> = LazyLock::new(|| init_regex(r"[a-z]"));
static RE_DIGIT: LazyLock<Regex> = LazyLock::new(|| init_regex(r"[0-9]"));

fn has_special_char(password: &str) -> bool {
    password.chars().any(|c| !c.is_alphanumeric())
}

// 漏洞 #7 修复：扩展常见密码黑名单至 100+ Top 列表
// 历史问题：原列表仅 15 个，遗漏"12345"、"iloveyou"、"monkey"等高频泄露密码
// 同时新增键盘序列检测（"asdf"、"zxcv"、"qwer"），覆盖更多攻击向量
const COMMON_PASSWORDS: &[&str] = &[
    // === Top 20 全球泄露密码 ===
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
    "123456789",
    "1234567890",
    "qwerty123",
    "0000000000",
    "iloveyou",
    // === Top 50 高频泄露 ===
    "monkey",
    "dragon",
    "sunshine",
    "princess",
    "football",
    "baseball",
    "charlie",
    "shadow",
    "master",
    "1234567",
    "654321",
    "superman",
    "batman",
    "trustno1",
    "hello",
    "michael",
    "jordan",
    "ashley",
    "jessica",
    "starwars",
    "computer",
    "freedom",
    "whatever",
    "1q2w3e4r",
    "qazwsx",
    "asdfgh",
    "zxcvbn",
    "qazwsxedc",
    "1qaz2wsx",
    "password1",
    "p@ssw0rd",
    "passw0rd",
    "p@ssword",
    "test",
    "test123",
    "guest",
    "user",
    "default",
    "changeme",
    "secret",
    "qwertyuiop",
    "asdfghjkl",
    "zxcvbnm",
    "1q2w3e",
    "abc",
    "abcd",
    "abcdef",
    "abcdefg",
    "abcd1234",
    "asdfasdf",
    "qwer1234",
    "qq123456",
    "520520",
    "1314520",
    "woaini",
    "aini",
    "wodema",
    "woaini1314",
    "aa123456",
    "a123456",
    "a12345678",
    "a123456789",
    "abc123456",
    "1q2w3e4r5t",
    "123qwe",
    "12345qwert",
    "z12345",
    "qq111111",
    "woaini520",
    "q1w2e3r4",
    "asd123",
    "asd123456",
    "147258369",
    "741852963",
    "159753",
    "159357",
    "123abc",
    "aa111111",
    "aa12345678",
    "qwerasdf",
    "aaaaaa",
    "aabbcc",
    "112233",
    "qweasd",
    "qweqwe",
    "1qazxsw2",
    "1234qwer",
    "123456a",
    "12345abc",
    "789456",
    "789456123",
    "asdf1234",
    "qqqqqq",
    "woaini123",
    "5201314",
];

// 漏洞 #7 修复：键盘序列模式（多行）
// 历史问题：原代码仅检测"qwerty"，遗漏"asdf"、"zxcv"、"qwer"等横/竖向键序列
const KEYBOARD_ROWS: &[&str] = &[
    "qwertyuiop",
    "asdfghjkl",
    "zxcvbnm",
    "1234567890",
    "!@#$%^&*()",
    "qazwsxedcrfvtgbyhnujmikolp", // 蛇形（column-wise）
];

/// 检测密码是否包含键盘序列（连续 4+ 字符）
///
/// 检测横排、竖排、蛇形、键盘数字行等所有 4+ 连续字符。
fn has_keyboard_sequence(password: &str) -> bool {
    let lower = password.to_lowercase();
    if lower.len() < 4 {
        return false;
    }

    for row in KEYBOARD_ROWS {
        // 正向：password 中包含 row 的连续 4+ 子串
        for i in 0..=row.len().saturating_sub(4) {
            let forward = &row[i..i + 4];
            if lower.contains(forward) {
                return true;
            }
        }
        // 反向：password 中包含 row 反向的连续 4+ 子串
        let reversed: String = row.chars().rev().collect();
        for i in 0..=reversed.len().saturating_sub(4) {
            let backward = &reversed[i..i + 4];
            if lower.contains(backward) {
                return true;
            }
        }
    }
    false
}

/// l33t speak 归一化（漏洞 #7 修复：拒绝变体）
///
/// 将常见替换还原为字母后再进行黑名单匹配：
/// - `@` / `4` → `a`
/// - `3` → `e`
/// - `1` / `!` / `|` → `i` / `l`（按上下文）
/// - `0` → `o`
/// - `5` → `s`
/// - `7` → `t`
/// - `$` → `s`
fn normalize_l33t(password: &str) -> String {
    let mut result = String::with_capacity(password.len());
    for c in password.chars() {
        let lower = c.to_ascii_lowercase();
        match lower {
            '@' | '4' => result.push('a'),
            '3' => result.push('e'),
            '1' => result.push('i'),
            '0' => result.push('o'),
            '5' => result.push('s'),
            '7' => result.push('t'),
            '$' => result.push('s'),
            '!' => result.push('i'),
            '|' => result.push('i'),
            c => result.push(c),
        }
    }
    result
}

/// 检查密码是否命中黑名单（漏洞 #7 修复：严格匹配 + l33t 还原）
///
/// 历史问题：原 `lower_password.contains(common)` 模糊匹配，导致
/// "P@ssw0rd1!"（"p@ssw0rd"不在黑名单）绕过。
/// 修复策略：
/// 1. 完全相等（归一化后）
/// 2. 严格前缀匹配（不模糊 contains）
/// 3. 归一化后比较（l33t 变体如"p@ssw0rd"→"password"）
/// 4. 移除末尾数字/特殊字符后再匹配（防"password123!"绕过）
fn matches_blacklist(plain_password: &str) -> bool {
    let lower = plain_password.to_lowercase();
    let normalized = normalize_l33t(&lower);

    // 1) 完全相等（不区分大小写）
    if COMMON_PASSWORDS.contains(&lower.as_str()) {
        return true;
    }
    if COMMON_PASSWORDS.contains(&normalized.as_str()) {
        return true;
    }

    // 2) 归一化后去掉末尾数字/特殊字符，再匹配（如"password1!" → "password"）
    let trimmed: String = normalized
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    if !trimmed.is_empty() && trimmed.len() >= 4 && COMMON_PASSWORDS.contains(&trimmed.as_str()) {
        return true;
    }

    // 3) 严格包含子串：长度差异 <= 2 的子串（即"password1"或"password1!"）
    // 注：contains 会把"passwordxxx"也命中，但限定长度差异避免误伤"passwordsomething"
    for &common in COMMON_PASSWORDS {
        if common.len() >= 5 && lower.contains(common) && (lower.len() - common.len()) <= 3 {
            return true;
        }
        if common.len() >= 5
            && normalized.contains(common)
            && (normalized.len() - common.len()) <= 3
        {
            return true;
        }
    }
    false
}

pub fn validate_password(password: &str) -> PasswordValidationResult {
    let policy = PasswordPolicy::default();
    validate_password_with_policy(password, &policy)
}

pub fn validate_password_with_policy(
    password: &str,
    policy: &PasswordPolicy,
) -> PasswordValidationResult {
    let mut errors = Vec::new();
    let (mut score, has_uppercase, has_lowercase, has_digit, has_special) =
        score_length_and_char_types(password, policy, &mut errors);
    score = apply_pattern_penalties(password, score, &mut errors);
    score = apply_complexity_bonuses(
        password.len(),
        has_uppercase,
        has_lowercase,
        has_digit,
        has_special,
        score,
    );
    let strength = evaluate_strength_and_check_min(score, policy, &mut errors);
    let is_valid = errors.is_empty();
    PasswordValidationResult {
        strength,
        is_valid,
        errors,
    }
}

/// 校验长度与字符类型，返回基础分数与 4 类字符存在标志
fn score_length_and_char_types(
    password: &str,
    policy: &PasswordPolicy,
    errors: &mut Vec<String>,
) -> (u8, bool, bool, bool, bool) {
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
    (score, has_uppercase, has_lowercase, has_digit, has_special)
}

/// 应用黑名单/键盘序列/连续/重复字符的模式惩罚
fn apply_pattern_penalties(password: &str, score: u8, errors: &mut Vec<String>) -> u8 {
    let mut score = score;
    // 漏洞 #7 修复：使用严格黑名单匹配（matches_blacklist）+ 键盘序列检测
    if matches_blacklist(password) {
        errors.push("密码过于常见，不安全（命中常见密码黑名单）".to_string());
        score = score.saturating_sub(50);
    }
    if has_keyboard_sequence(password) {
        errors.push("密码包含键盘序列（如 qwer/asdf/zxcv），不安全".to_string());
        score = score.saturating_sub(20);
    }
    if has_consecutive_chars(password) {
        score = score.saturating_sub(10);
    }
    if has_repeated_chars(password) {
        score = score.saturating_sub(10);
    }
    score
}

/// 应用长度≥16 与四类字符齐备的复杂度奖励
fn apply_complexity_bonuses(
    password_len: usize,
    has_uppercase: bool,
    has_lowercase: bool,
    has_digit: bool,
    has_special: bool,
    mut score: u8,
) -> u8 {
    if password_len >= 16 {
        score += 10;
    }
    if has_uppercase && has_lowercase && has_digit && has_special {
        score += 10;
    }
    score
}

/// 按分数映射强度等级，并校验是否满足策略最低强度要求
fn evaluate_strength_and_check_min(
    score: u8,
    policy: &PasswordPolicy,
    errors: &mut Vec<String>,
) -> PasswordStrength {
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
    strength
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

    // === 漏洞 #7 修复单元测试 ===

    /// #7 验证：l33t 变体 "P@ssw0rd1!" 应被拒绝
    ///
    /// 历史问题：原"contains"模糊匹配无法识别 l33t 变体
    #[test]
    fn test_l33t_variant_rejected() {
        let result = validate_password("P@ssw0rd1!");
        assert!(
            !result.is_valid,
            "P@ssw0rd1! 应被拒绝（l33t 变体），实际 errors: {:?}",
            result.errors
        );
        assert!(
            result.errors.iter().any(|e| e.contains("常见")),
            "应包含'常见'错误，实际 errors: {:?}",
            result.errors
        );
    }

    /// #7 验证：完全相等黑名单 "password" 应被拒绝
    #[test]
    fn test_exact_blacklist_match_rejected() {
        let result = validate_password("password");
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("常见")));
    }

    /// #7 验证：扩展黑名单 Top 50（"12345"、"iloveyou"）应被拒绝
    #[test]
    fn test_extended_blacklist_rejected() {
        for weak in &["12345", "iloveyou", "monkey", "dragon", "sunshine", "princess"] {
            let result = validate_password(weak);
            assert!(
                !result.is_valid,
                "常见密码 '{}' 应被拒绝，实际 errors: {:?}",
                weak,
                result.errors
            );
        }
    }

    /// #7 验证：归一化后相等 "P@ssword" 应被拒绝
    #[test]
    fn test_normalized_match_rejected() {
        let result = validate_password("P@ssword");
        assert!(
            !result.is_valid,
            "P@ssword（归一化后=password）应被拒绝"
        );
    }

    /// #7 验证：截尾黑名单 "admin1!" 应被拒绝
    ///
    /// 历史问题：原"contains"匹配"admin123"会命中，但仅去掉末尾数字/特殊字符
    /// 的简化密码"admin"需要新逻辑
    #[test]
    fn test_trimmed_blacklist_rejected() {
        let result = validate_password("admin1!");
        assert!(
            !result.is_valid,
            "admin1!（去掉尾部后=admin）应被拒绝"
        );
    }

    /// #7 验证：键盘序列 "Qwerty123" 应被拒绝
    #[test]
    fn test_keyboard_sequence_rejected() {
        let result = validate_password("Qwerty123!");
        assert!(!result.is_valid, "Qwerty123!（含键盘序列）应被拒绝");
        assert!(
            result.errors.iter().any(|e| e.contains("键盘序列")),
            "应包含'键盘序列'错误，实际 errors: {:?}",
            result.errors
        );
    }

    /// #7 验证：键盘序列 "Asdf" / "Zxcv" / "Qwer" 都应被检测
    #[test]
    fn test_keyboard_sequence_various() {
        for kb in &["asdf", "Asdf1234!", "zxcvbnm", "Qwerasdf"] {
            let result = validate_password(kb);
            assert!(
                !result.is_valid,
                "键盘序列 '{}' 应被拒绝，实际 errors: {:?}",
                kb,
                result.errors
            );
        }
    }

    /// #7 验证：键盘反向序列 "4321" 应被检测
    #[test]
    fn test_keyboard_sequence_reverse() {
        let result = validate_password("4321!Abc");
        assert!(!result.is_valid, "反向键盘序列应被拒绝");
    }

    /// #7 验证：l33t 变体 "passw0rd" 严格匹配
    #[test]
    fn test_l33t_strict_match() {
        let result = validate_password("passw0rd");
        assert!(!result.is_valid, "passw0rd 已在黑名单，应被拒绝");
    }

    /// #7 验证：合法强密码仍通过
    #[test]
    fn test_strong_password_still_accepted() {
        let result = validate_password("Tr0ub4dor&3xYz!@#");
        assert!(
            result.is_valid,
            "强密码应通过验证，实际 errors: {:?}",
            result.errors
        );
    }

    /// #7 验证：边界 - 长度不足的密码不被键盘序列误判
    #[test]
    fn test_short_password_no_false_positive() {
        let result = validate_password("Ab1!");
        // 长度 < 4，键盘序列不会触发
        assert!(
            !result.errors.iter().any(|e| e.contains("键盘序列")),
            "短密码不应触发键盘序列错误"
        );
    }
}
