use bingxi_backend::services::customer_credit_service::*;

// ========== clamp_page 纯函数测试 ==========

/// test_ymys_lzbxzw1（验证 page=0（页码小于下界）被 clamp 到 1。）
#[test]
fn test_ymys_lzbxzw1() {
    assert_eq!(CustomerCreditService::clamp_page(0), 1);
}

/// test_ymys_ymw1_bjfh1（验证 page=1（下界边界）原值返回 1。）
#[test]
fn test_ymys_ymw1_bjfh1() {
    assert_eq!(CustomerCreditService::clamp_page(1), 1);
}

/// test_ymys_ymw500_zcfwyzfh（验证 page=500（在 [1, 1000] 正常范围内）原值返回 500。）
#[test]
fn test_ymys_ymw500_zcfwyzfh() {
    assert_eq!(CustomerCreditService::clamp_page(500), 500);
}

/// test_ymys_ymw1000_bjfh1000（验证 page=1000（上界边界）原值返回 1000。）
#[test]
fn test_ymys_ymw1000_bjfh1000() {
    assert_eq!(CustomerCreditService::clamp_page(1000), 1000);
}

/// test_ymys_ymw1001_ccsjbxzw1000（验证 page=1001（超出上界）被 clamp 到 1000，防止超大偏移量 DoS。）
#[test]
fn test_ymys_ymw1001_ccsjbxzw1000() {
    assert_eq!(CustomerCreditService::clamp_page(1001), 1000);
}

/// test_ymys_fzbxzw1（验证 page=-5（负数）被 clamp 到 1，防止负偏移量。）
#[test]
fn test_ymys_fzbxzw1() {
    assert_eq!(CustomerCreditService::clamp_page(-5), 1);
}

/// test_ymys_i64zdzbxzw1000（验证 page=i64::MAX（极端大值）被 clamp 到 1000，防止溢出与 DoS。）
#[test]
fn test_ymys_i64zdzbxzw1000() {
    assert_eq!(CustomerCreditService::clamp_page(i64::MAX), 1000);
}

/// test_ymys_i64zxzbxzw1（验证 page=i64::MIN（极端小值）被 clamp 到 1，防止溢出与负偏移。）
#[test]
fn test_ymys_i64zxzbxzw1() {
    assert_eq!(CustomerCreditService::clamp_page(i64::MIN), 1);
}

// ========== CreditQueryParams Default 实现测试 ==========

/// test_cxcs_defaultsx_qbzdwkhl
/// 验证 CreditQueryParams::default() 返回的结构体：customer_id=None（未指定客户筛选）；credit_level=None（未指定信用等级筛选）；status=None（未指定状态筛选）；page=0（零页码，需配合 clamp_page 使用）；page_size=0（零页大小）
#[test]
fn test_cxcs_defaultsx_qbzdwkhl() {
    let params = CreditQueryParams::default();
    assert!(params.customer_id.is_none(), "默认 customer_id 应为 None");
    assert!(params.credit_level.is_none(), "默认 credit_level 应为 None");
    assert!(params.status.is_none(), "默认 status 应为 None");
    assert_eq!(params.page, 0, "默认 page 应为 0");
    assert_eq!(params.page_size, 0, "默认 page_size 应为 0");
}
