//! 密码验证器单元测试

use bingxi_backend::utils::password_validator::{
    PasswordStrength, PasswordValidationResult, get_password_feedback, validate_password,
};

#[test]
fn test_valid_password() {
    let result = validate_password("StrongP@ssw0rd");
    assert!(result.is_valid);
    assert!(result.strength.score() >= 60);
}

#[test]
fn test_too_short_password() {
    let result = validate_password("Short1!");
    assert!(!result.is_valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("至少") || e.contains("at least"))
    );
}

#[test]
fn test_missing_uppercase() {
    let result = validate_password("lowercase123!");
    assert!(!result.is_valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("大写字母") || e.contains("uppercase"))
    );
}

#[test]
fn test_missing_lowercase() {
    let result = validate_password("UPPERCASE123!");
    assert!(!result.is_valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("小写字母") || e.contains("lowercase"))
    );
}

#[test]
fn test_missing_digit() {
    let result = validate_password("NoDigitsHere!");
    assert!(!result.is_valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("数字") || e.contains("digit"))
    );
}

#[test]
fn test_missing_special_char() {
    let result = validate_password("NoSpecialChars123");
    assert!(!result.is_valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("特殊字符") || e.contains("special"))
    );
}

#[test]
fn test_common_password() {
    let result = validate_password("password");
    assert!(!result.is_valid);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("常见") || e.contains("common"))
    );
}

#[test]
fn test_feedback_generation() {
    // 用生产真实文案（validate_password 推入的即中文规则文案），不再用英文占位串，
    // 否则测不出"中英夹杂进界面"这一族缺陷
    let result = PasswordValidationResult {
        strength: PasswordStrength::Weak,
        is_valid: false,
        errors: vec![
            "密码长度至少为 8 个字符".to_string(),
            "密码必须包含大写字母".to_string(),
        ],
    };
    let feedback = get_password_feedback(&result);
    assert!(
        feedback.contains("密码长度至少为 8 个字符") && feedback.contains("密码必须包含大写字母"),
        "两条规则原因都必须原样带给用户，实际={feedback}"
    );
    assert!(
        !feedback.contains("Password") && !feedback.contains("failed"),
        "出参文案不得夹英文前缀（本仓密码文案一律中文），实际={feedback}"
    );

    let ok = PasswordValidationResult {
        strength: PasswordStrength::Strong,
        is_valid: true,
        errors: Vec::new(),
    };
    let ok_feedback = get_password_feedback(&ok);
    assert!(
        ok_feedback.contains("强") && ok_feedback.contains("符合要求"),
        "通过时应给出中文强度说明，实际={ok_feedback}"
    );
}
