use bingxi_backend::services::occupational_health_service::*;
use rust_decimal::Decimal;

#[test]
fn test_check_exceedance_normal() {
    let (is_exceeding, ratio) =
        OccupationalHealthService::check_exceedance(Decimal::new(5, 0), Decimal::new(10, 0));
    assert!(!is_exceeding);
    assert!(ratio.is_none());
}

#[test]
fn test_check_exceedance_exceeding() {
    let (is_exceeding, ratio) =
        OccupationalHealthService::check_exceedance(Decimal::new(15, 0), Decimal::new(10, 0));
    assert!(is_exceeding);
    // 超标倍数 = 15/10 - 1 = 0.5
    assert_eq!(ratio, Some(Decimal::new(5, 1)));
}

#[test]
fn test_check_exceedance_equal() {
    let (is_exceeding, ratio) =
        OccupationalHealthService::check_exceedance(Decimal::new(10, 0), Decimal::new(10, 0));
    assert!(!is_exceeding);
    assert!(ratio.is_none());
}

#[test]
fn test_classify_exam_expiry_level_expired() {
    assert_eq!(
        OccupationalHealthService::classify_exam_expiry_level(-1),
        ExamExpiryWarningLevel::Expired
    );
    assert_eq!(
        OccupationalHealthService::classify_exam_expiry_level(-30),
        ExamExpiryWarningLevel::Expired
    );
}

#[test]
fn test_classify_exam_expiry_level_critical() {
    assert_eq!(
        OccupationalHealthService::classify_exam_expiry_level(0),
        ExamExpiryWarningLevel::Critical
    );
    assert_eq!(
        OccupationalHealthService::classify_exam_expiry_level(30),
        ExamExpiryWarningLevel::Critical
    );
}

#[test]
fn test_classify_exam_expiry_level_warning() {
    assert_eq!(
        OccupationalHealthService::classify_exam_expiry_level(31),
        ExamExpiryWarningLevel::Warning
    );
    assert_eq!(
        OccupationalHealthService::classify_exam_expiry_level(60),
        ExamExpiryWarningLevel::Warning
    );
}

#[test]
fn test_classify_exam_expiry_level_notice() {
    assert_eq!(
        OccupationalHealthService::classify_exam_expiry_level(61),
        ExamExpiryWarningLevel::Notice
    );
    assert_eq!(
        OccupationalHealthService::classify_exam_expiry_level(90),
        ExamExpiryWarningLevel::Notice
    );
}

#[test]
fn test_classify_exam_expiry_level_none() {
    assert_eq!(
        OccupationalHealthService::classify_exam_expiry_level(91),
        ExamExpiryWarningLevel::None
    );
    assert_eq!(
        OccupationalHealthService::classify_exam_expiry_level(365),
        ExamExpiryWarningLevel::None
    );
}

#[test]
fn test_validate_hazard_type_valid() {
    assert!(OccupationalHealthService::validate_hazard_type("chemical").is_ok());
    assert!(OccupationalHealthService::validate_hazard_type("physical").is_ok());
    assert!(OccupationalHealthService::validate_hazard_type("dust").is_ok());
    assert!(OccupationalHealthService::validate_hazard_type("biological").is_ok());
}

#[test]
fn test_validate_hazard_type_invalid() {
    assert!(OccupationalHealthService::validate_hazard_type("invalid").is_err());
}

#[test]
fn test_validate_exam_type_valid() {
    assert!(OccupationalHealthService::validate_exam_type("pre_employment").is_ok());
    assert!(OccupationalHealthService::validate_exam_type("in_service").is_ok());
    assert!(OccupationalHealthService::validate_exam_type("resignation").is_ok());
}

#[test]
fn test_validate_exam_type_invalid() {
    assert!(OccupationalHealthService::validate_exam_type("invalid").is_err());
}

#[test]
fn test_validate_exam_result_valid() {
    assert!(OccupationalHealthService::validate_exam_result("normal").is_ok());
    assert!(OccupationalHealthService::validate_exam_result("abnormal").is_ok());
    assert!(OccupationalHealthService::validate_exam_result("contraindication").is_ok());
}

#[test]
fn test_validate_exam_result_invalid() {
    assert!(OccupationalHealthService::validate_exam_result("invalid").is_err());
}

#[test]
fn test_validate_ppe_type_valid() {
    assert!(OccupationalHealthService::validate_ppe_type("mask").is_ok());
    assert!(OccupationalHealthService::validate_ppe_type("gloves").is_ok());
    assert!(OccupationalHealthService::validate_ppe_type("goggles").is_ok());
    assert!(OccupationalHealthService::validate_ppe_type("earplug").is_ok());
    assert!(OccupationalHealthService::validate_ppe_type("respirator").is_ok());
    assert!(OccupationalHealthService::validate_ppe_type("suit").is_ok());
}

#[test]
fn test_validate_ppe_type_invalid() {
    assert!(OccupationalHealthService::validate_ppe_type("invalid").is_err());
}

#[test]
fn test_get_limit_benzene() {
    let limit = OccupationalHazardLimitReference::get_limit("chemical", "苯");
    assert_eq!(limit, Some(Decimal::new(6, 0)));
}

#[test]
fn test_get_limit_noise() {
    let limit = OccupationalHazardLimitReference::get_limit("physical", "噪声");
    assert_eq!(limit, Some(Decimal::new(85, 0)));
}

#[test]
fn test_get_limit_unknown() {
    let limit = OccupationalHazardLimitReference::get_limit("biological", "未知");
    assert_eq!(limit, None);
}

#[test]
fn test_exam_expiry_warning_level_needs_warning() {
    assert!(ExamExpiryWarningLevel::Expired.needs_warning());
    assert!(ExamExpiryWarningLevel::Critical.needs_warning());
    assert!(ExamExpiryWarningLevel::Warning.needs_warning());
    assert!(ExamExpiryWarningLevel::Notice.needs_warning());
    assert!(!ExamExpiryWarningLevel::None.needs_warning());
}
