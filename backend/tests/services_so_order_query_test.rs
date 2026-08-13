// 批次 158 v11 修复 clippy：so_status 仅测试代码引用，use 移入测试模块避免 unused import 警告
use bingxi_backend::models::sales_order::so_status;
use bingxi_backend::models::status::sales_order as so_status;

#[test]
fn test_order_query_is_empty() {
    let q = OrderQuery::default();
    assert!(q.is_empty());
    assert_eq!(q.desc(), "无过滤条件");
}

#[test]
fn test_order_query_with_filters() {
    let q = OrderQuery {
        customer_id: Some(100),
        status: Some(so_status::APPROVED.to_string()),
        ..Default::default()
    };
    assert!(!q.is_empty());
    assert!(q.desc().contains("客户ID=100"));
    assert!(q.desc().contains("状态=approved"));
}

#[test]
fn test_query_module_loaded() {
    assert_eq!(P92_QRY_MODULE, "sales_order_query");
}
