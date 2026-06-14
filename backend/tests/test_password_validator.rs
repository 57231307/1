//! 密码验证器单元测试

use bingxi_backend::utils::password_validator::{
    get_password_feedback, validate_password, PasswordStrength, PasswordValidationResult,
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
    assert!(result.errors.iter().any(|e| e.contains("at least")));
}

#[test]
fn test_missing_uppercase() {
    let result = validate_password("lowercase123!");
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("uppercase")));
}

#[test]
fn test_missing_lowercase() {
    let result = validate_password("UPPERCASE123!");
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("lowercase")));
}

#[test]
fn test_missing_digit() {
    let result = validate_password("NoDigitsHere!");
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("digit")));
}

#[test]
fn test_missing_special_char() {
    let result = validate_password("NoSpecialChars123");
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("special")));
}

#[test]
fn test_common_password() {
    let result = validate_password("password");
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("common")));
}

#[test]
fn test_feedback_generation() {
    let result = PasswordValidationResult {
        strength: PasswordStrength::Weak,
        is_valid: false,
        errors: vec![
            "Password must be at least 8 chars".to_string(),
            "Password must contain uppercase".to_string(),
        ],
    };
    let feedback = get_password_feedback(&result);
    assert!(feedback.contains("failed"));
    assert!(feedback.contains("at least 8 chars"));
}
