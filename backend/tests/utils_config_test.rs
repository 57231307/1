use bingxi_backend::models::status::wage_energy_chemical_business::*;
use std::env;

/// 测试 `APP_ENV=production` 时识别为生产环境
#[test]
fn test_is_production_with_production_value() {
    // 设置为 production
    unsafe {
        env::set_var("APP_ENV", "production");
    }
    assert!(is_production(), "APP_ENV=production 时应判定为生产环境");
    // 清理环境变量，避免污染后续测试
    unsafe {
        env::remove_var("APP_ENV");
    }
}

/// 测试 `APP_ENV=development` 时识别为开发环境
#[test]
fn test_is_production_with_development_value() {
    unsafe {
        env::set_var("APP_ENV", "development");
    }
    assert!(!is_production(), "APP_ENV=development 时应判定为开发环境");
    unsafe {
        env::remove_var("APP_ENV");
    }
}

/// 测试 `APP_ENV` 未设置时识别为开发环境（保守策略）
#[test]
fn test_is_production_with_unset() {
    unsafe {
        env::remove_var("APP_ENV");
    }
    assert!(
        !is_production(),
        "APP_ENV 未设置时应判定为开发环境（保守策略）"
    );
}

/// 测试 `APP_ENV=PRODUCTION` 大小写不敏感识别
#[test]
fn test_is_production_case_insensitive() {
    unsafe {
        env::set_var("APP_ENV", "PRODUCTION");
    }
    assert!(is_production(), "APP_ENV=PRODUCTION 大写也应判定为生产环境");
    unsafe {
        env::remove_var("APP_ENV");
    }
}
