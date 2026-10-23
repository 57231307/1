use bingxi_backend::services::labor_contract_service::*;
use bingxi_backend::services::pollution_permit_service::*;
use chrono::NaiveDate;
use rust_decimal::Decimal;

#[test]
fn test_classify_expiry_level_normal() {
    assert_eq!(
        LaborContractService::classify_expiry_level(120),
        ContractExpiryLevel::Normal
    );
    assert_eq!(
        LaborContractService::classify_expiry_level(91),
        ContractExpiryLevel::Normal
    );
}

#[test]
fn test_classify_expiry_level_30_days() {
    assert_eq!(
        LaborContractService::classify_expiry_level(30),
        ContractExpiryLevel::Warning30Days
    );
    assert_eq!(
        LaborContractService::classify_expiry_level(0),
        ContractExpiryLevel::Warning30Days
    );
}

#[test]
fn test_classify_expiry_level_expired() {
    assert_eq!(
        LaborContractService::classify_expiry_level(-1),
        ContractExpiryLevel::Expired
    );
}

#[test]
fn test_validate_probation_short_contract() {
    // 合同期 6 个月，试用期 1 个月（合法）
    let start = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
    let end = NaiveDate::from_ymd_opt(2026, 7, 2).unwrap();
    let probation_end = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
    let result = LaborContractService::validate_probation(
        start,
        Some(end),
        probation_end,
        Decimal::new(4000, 0),
        Decimal::new(5000, 0),
    );
    assert!(result.is_valid);
}

#[test]
fn test_validate_probation_too_long() {
    // 合同期 1 年，试用期 6 个月（违法，应 ≤ 2 个月）
    let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
    let probation_end = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
    let result = LaborContractService::validate_probation(
        start,
        Some(end),
        probation_end,
        Decimal::new(4000, 0),
        Decimal::new(5000, 0),
    );
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("试用期长度")));
}

#[test]
fn test_validate_probation_salary_too_low() {
    // 试用期工资 < 转正工资 80%
    let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
    let probation_end = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
    let result = LaborContractService::validate_probation(
        start,
        Some(end),
        probation_end,
        Decimal::new(3000, 0), // 3000 < 5000 * 0.8 = 4000
        Decimal::new(5000, 0),
    );
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("80%")));
}

#[test]
fn test_validate_contract_type_valid() {
    assert!(LaborContractService::validate_contract_type("fixed_term").is_ok());
    assert!(LaborContractService::validate_contract_type("permanent").is_ok());
    assert!(LaborContractService::validate_contract_type("task_based").is_ok());
}

#[test]
fn test_validate_contract_type_invalid() {
    assert!(LaborContractService::validate_contract_type("invalid").is_err());
}

#[test]
fn test_validate_working_hours_system_valid() {
    assert!(LaborContractService::validate_working_hours_system("standard").is_ok());
    assert!(LaborContractService::validate_working_hours_system("comprehensive").is_ok());
    assert!(LaborContractService::validate_working_hours_system("flexible").is_ok());
}

#[test]
fn test_validate_working_hours_system_invalid() {
    assert!(LaborContractService::validate_working_hours_system("invalid").is_err());
}

/// 契约锁 D2：working_hours_system 与 sign_date 为创建请求非 Option 必填，
/// 各自缺失时反序列化必须失败并点名该键（前端缺键即 422 的事实来源，不许改成 Option 掩盖）。
/// 两键分两次单独缺失，因为 serde 只报首个缺失字段，一次缺两键无法同时断言两个键名。
#[test]
fn create_request_requires_working_hours_system_and_sign_date() {
    let base = serde_json::json!({
        "worker_id": 1,
        "contract_no": "LC-0001",
        "contract_type": "fixed_term",
        "start_date": "2026-01-01",
        "end_date": "2027-01-01",
        "probation_salary": "4000.00",
        "regular_salary": "5000.00"
    });

    let mut missing_working_hours = base.clone();
    missing_working_hours["sign_date"] = serde_json::json!("2025-12-20");
    let err = serde_json::from_value::<CreateLaborContractRequest>(missing_working_hours)
        .expect_err("缺 working_hours_system 必须反序列化失败");
    let msg = err.to_string();
    assert!(
        msg.contains("working_hours_system"),
        "报错应点名缺失键: {msg}"
    );

    let mut missing_sign_date = base;
    missing_sign_date["working_hours_system"] = serde_json::json!("standard");
    let err = serde_json::from_value::<CreateLaborContractRequest>(missing_sign_date)
        .expect_err("缺 sign_date 必须反序列化失败");
    let msg = err.to_string();
    assert!(msg.contains("sign_date"), "报错应点名缺失键: {msg}");

    let complete = serde_json::json!({
        "worker_id": 1,
        "contract_no": "LC-0001",
        "contract_type": "fixed_term",
        "start_date": "2026-01-01",
        "end_date": "2027-01-01",
        "probation_salary": "4000.00",
        "regular_salary": "5000.00",
        "working_hours_system": "standard",
        "sign_date": "2025-12-20"
    });
    let req: CreateLaborContractRequest =
        serde_json::from_value(complete).expect("补齐必填后应反序列化成功");
    assert_eq!(req.working_hours_system, "standard");
    assert_eq!(
        req.sign_date,
        NaiveDate::from_ymd_opt(2025, 12, 20).unwrap()
    );
}

/// 契约锁 D3：更新请求必须承载界面可编辑的日期与工资条款（读写往返的写侧入口），
/// 缺省字段保持 None 语义（不变更），杜绝「编辑保存后刷新即丢」。
#[test]
fn update_request_deserializes_core_terms() {
    let req: UpdateLaborContractRequest = serde_json::from_value(serde_json::json!({
        "contract_type": "permanent",
        "start_date": "2026-01-01",
        "probation_end_date": "2026-03-01",
        "probation_salary": "4000.00",
        "regular_salary": "5000.00",
        "working_hours_system": "comprehensive",
        "remarks": "renewal"
    }))
    .expect("核心条款键必须被更新请求接收");
    assert_eq!(req.contract_type.as_deref(), Some("permanent"));
    assert_eq!(
        req.start_date,
        Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    );
    assert_eq!(
        req.probation_end_date,
        Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap())
    );
    assert_eq!(req.probation_salary, Some(Decimal::new(4000, 0)));
    assert_eq!(req.regular_salary, Some(Decimal::new(5000, 0)));
    assert_eq!(req.working_hours_system.as_deref(), Some("comprehensive"));
    assert_eq!(req.remarks.as_deref(), Some("renewal"));
    assert_eq!(req.end_date, None, "未提交的键缺省即不变更");
    assert_eq!(req.position, None);
}
