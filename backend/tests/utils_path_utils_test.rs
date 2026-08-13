use bingxi_backend::utils::path_utils::*;


// ===== is_module_prefix 测试 =====

#[test]
fn test_is_module_prefix_xsy() {
    assert!(is_module_prefix("sales"));
    assert!(is_module_prefix("quotations"));
    assert!(is_module_prefix("custom-orders"));
    assert!(is_module_prefix("color-cards"));
    assert!(is_module_prefix("color-prices"));
    assert!(is_module_prefix("trading"));
}

#[test]
fn test_is_module_prefix_cgyxzpx() {
    // V15 修正：purchases → purchase
    assert!(is_module_prefix("purchase"));
    assert!(!is_module_prefix("purchases"));
}

#[test]
fn test_is_module_prefix_scyxz() {
    // V15 新增：production 原缺失
    assert!(is_module_prefix("production"));
    assert!(is_module_prefix("material-shortage"));
    assert!(is_module_prefix("scheduling"));
}

#[test]
fn test_is_module_prefix_cwy() {
    assert!(is_module_prefix("finance"));
    assert!(is_module_prefix("ap"));
    assert!(is_module_prefix("ar"));
    assert!(is_module_prefix("assist-accounting"));
}

#[test]
fn test_is_module_prefix_rzyxty() {
    assert!(is_module_prefix("auth"));
    assert!(is_module_prefix("ws"));
    assert!(is_module_prefix("init"));
    assert!(is_module_prefix("system-update"));
    assert!(is_module_prefix("dashboard"));
    assert!(is_module_prefix("audit-logs"));
    assert!(is_module_prefix("slow-queries"));
    assert!(is_module_prefix("user"));
    assert!(is_module_prefix("data-import"));
}

#[test]
fn test_is_module_prefix_fxybby() {
    assert!(is_module_prefix("reports"));
    assert!(is_module_prefix("bi"));
    assert!(is_module_prefix("advanced"));
    assert!(is_module_prefix("search"));
}

#[test]
fn test_is_module_prefix_yqlzsj() {
    // V15 清理：以下脏数据应已移除
    assert!(!is_module_prefix("purchases")); // 拼写错误
    assert!(!is_module_prefix("api-keys")); // 路径不存在
    assert!(!is_module_prefix("gl")); // 不是路径段
    assert!(!is_module_prefix("supplier-evaluation")); // 位置错误
    assert!(!is_module_prefix("customer-credits")); // 位置错误
    assert!(!is_module_prefix("quality-inspection")); // 位置错误
    assert!(!is_module_prefix("cost-collections")); // 位置错误
    assert!(!is_module_prefix("sales-analysis")); // 位置错误
    assert!(!is_module_prefix("sales-prices")); // 位置错误
    assert!(!is_module_prefix("purchase-prices")); // 位置错误
    assert!(!is_module_prefix("sales-returns")); // 位置错误
    assert!(!is_module_prefix("financial-analysis")); // 非模块前缀
    assert!(!is_module_prefix("fund-management")); // 非模块前缀
    assert!(!is_module_prefix("ar-reconciliations")); // 非模块前缀
    assert!(!is_module_prefix("exchange-rates")); // 非模块前缀
}

#[test]
fn test_is_module_prefix_wzdfh_false() {
    assert!(!is_module_prefix("unknown-module"));
    assert!(!is_module_prefix(""));
    assert!(!is_module_prefix("fake"));
}

// ===== is_known_resource_segment 测试 =====

#[test]
fn test_is_known_resource_segment_bhsymkqz() {
    assert!(is_known_resource_segment("sales"));
    assert!(is_known_resource_segment("purchase"));
    assert!(is_known_resource_segment("production"));
    assert!(is_known_resource_segment("finance"));
}

#[test]
fn test_is_known_resource_segment_bhzjzy() {
    assert!(is_known_resource_segment("users"));
    assert!(is_known_resource_segment("roles"));
    assert!(is_known_resource_segment("departments"));
    assert!(is_known_resource_segment("products"));
    assert!(is_known_resource_segment("vouchers"));
    assert!(is_known_resource_segment("suppliers"));
    assert!(is_known_resource_segment("system-config"));
    assert!(is_known_resource_segment("health"));
}

#[test]
fn test_is_known_resource_segment_wzdfh_false() {
    assert!(!is_known_resource_segment("unknown-resource"));
    assert!(!is_known_resource_segment(""));
    assert!(!is_known_resource_segment("hack"));
}