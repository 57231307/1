use bingxi_backend::services::pollution_permit_service::*;


#[test]
fn test_classify_expiry_level_normal() {
    assert_eq!(
        PollutionPermitService::classify_expiry_level(120),
        ExpiryWarningLevel::Normal
    );
    assert_eq!(
        PollutionPermitService::classify_expiry_level(91),
        ExpiryWarningLevel::Normal
    );
}

#[test]
fn test_classify_expiry_level_90_days() {
    assert_eq!(
        PollutionPermitService::classify_expiry_level(90),
        ExpiryWarningLevel::Warning90Days
    );
    assert_eq!(
        PollutionPermitService::classify_expiry_level(61),
        ExpiryWarningLevel::Warning90Days
    );
}

#[test]
fn test_classify_expiry_level_60_days() {
    assert_eq!(
        PollutionPermitService::classify_expiry_level(60),
        ExpiryWarningLevel::Warning60Days
    );
    assert_eq!(
        PollutionPermitService::classify_expiry_level(31),
        ExpiryWarningLevel::Warning60Days
    );
}

#[test]
fn test_classify_expiry_level_30_days() {
    assert_eq!(
        PollutionPermitService::classify_expiry_level(30),
        ExpiryWarningLevel::Warning30Days
    );
    assert_eq!(
        PollutionPermitService::classify_expiry_level(1),
        ExpiryWarningLevel::Warning30Days
    );
    assert_eq!(
        PollutionPermitService::classify_expiry_level(0),
        ExpiryWarningLevel::Warning30Days
    );
}

#[test]
fn test_classify_expiry_level_expired() {
    assert_eq!(
        PollutionPermitService::classify_expiry_level(-1),
        ExpiryWarningLevel::Expired
    );
    assert_eq!(
        PollutionPermitService::classify_expiry_level(-30),
        ExpiryWarningLevel::Expired
    );
}

#[test]
fn test_validate_permit_type_valid() {
    assert!(PollutionPermitService::validate_permit_type("wastewater").is_ok());
    assert!(PollutionPermitService::validate_permit_type("exhaust").is_ok());
    assert!(PollutionPermitService::validate_permit_type("solid_waste").is_ok());
}

#[test]
fn test_validate_permit_type_invalid() {
    assert!(PollutionPermitService::validate_permit_type("invalid").is_err());
}