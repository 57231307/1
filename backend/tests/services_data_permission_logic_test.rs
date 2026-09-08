//! 数据权限服务集成测试（纯逻辑层：字段级权限过滤 + DataScopeFilter 构造语义）
//!
//! 覆盖缺口：data_permission_service.rs 在 CI 覆盖率报告中 0% 行覆盖。
//! filter_fields 是字段级权限（RBAC 字段矩阵）的执行核心——过滤逻辑回归
//! 意味着敏感字段（成本/利润/手机号）泄露或合法字段被误删。
//!
//! 验证范围：
//! - filter_fields：白名单保留 / 黑名单移除 / 组合 / 非 object 输入的安全行为
//! - filter_fields_batch：批量一致性
//! - DataScopeFilter 五种构造的语义正确性（is_all/is_none 与载荷字段联动）

use bingxi_backend::services::data_permission_service::{DataPermissionService, DataScopeFilter};
use serde_json::json;

fn svc() -> DataPermissionService {
    // filter_fields/filter_fields_batch 是纯 JSON 变换，不触达 db；
    // DatabaseConnection::default() 为 no-op 实例（项目既有测试同模式，
    // 见 services_ai_model_management_service_test.rs），构造无需真实连接
    DataPermissionService::new(std::sync::Arc::new(sea_orm::DatabaseConnection::default()))
}

#[test]
fn test_filter_fields_allowlist_keeps_only_allowed() {
    let mut data = json!({
        "id": 1,
        "customer_name": "客户A",
        "unit_price": "12.5",
        "cost_price": "8.0",
        "profit": "4.5"
    });
    let svc = svc();
    // 白名单：成本/利润不在列 → 移除（字段级权限防成本泄露）
    svc.filter_fields(
        &mut data,
        &Some(vec![
            "id".into(),
            "customer_name".into(),
            "unit_price".into(),
        ]),
        &None,
    );
    assert!(data.get("id").is_some());
    assert!(data.get("unit_price").is_some());
    assert!(data.get("cost_price").is_none());
    assert!(data.get("profit").is_none());
}

#[test]
fn test_filter_fields_hiddenlist_removes_hidden() {
    let mut data = json!({"contact_phone": "13800000001", "customer_name": "客户B", "internal_remark": "内部备注"});
    let svc = svc();
    // 黑名单：仅移除指定字段，其余保留
    svc.filter_fields(
        &mut data,
        &None,
        &Some(vec!["contact_phone".into(), "internal_remark".into()]),
    );
    assert!(data.get("contact_phone").is_none());
    assert!(data.get("internal_remark").is_none());
    assert!(data.get("customer_name").is_some());
}

#[test]
fn test_filter_fields_allow_then_hide_combination() {
    let mut data = json!({"a": 1, "b": 2, "c": 3, "d": 4});
    let svc = svc();
    // 先白名单收敛（a,b），再黑名单移除（b）→ 仅剩 a（实现顺序：retain → remove）
    svc.filter_fields(
        &mut data,
        &Some(vec!["a".into(), "b".into()]),
        &Some(vec!["b".into()]),
    );
    assert!(data.get("a").is_some());
    assert!(data.get("b").is_none());
    assert!(data.get("c").is_none());
    assert!(data.get("d").is_none());
}

#[test]
fn test_filter_fields_non_object_is_safe_noop() {
    // 非 object 输入（数组/标量）必须是安全 no-op：不 panic、不篡改（防越权过滤破坏响应）
    let mut arr = json!([1, 2, 3]);
    let mut scalar = json!(42);
    let svc = svc();
    svc.filter_fields(&mut arr, &Some(vec!["a".into()]), &None);
    svc.filter_fields(&mut scalar, &None, &Some(vec!["x".into()]));
    assert_eq!(arr, json!([1, 2, 3]));
    assert_eq!(scalar, json!(42));
}

#[test]
fn test_filter_fields_batch_consistency() {
    let svc = svc();
    let mut list = vec![
        json!({"name": "A", "secret": 1}),
        json!({"name": "B", "secret": 2}),
        json!({"name": "C", "secret": 3}),
    ];
    svc.filter_fields_batch(&mut list, &None, &Some(vec!["secret".into()]));
    for item in &list {
        assert!(item.get("secret").is_none());
        assert!(item.get("name").is_some());
    }
}

#[test]
fn test_data_scope_filter_semantics() {
    // 五种构造的语义：is_all/is_none 与载荷字段的联动契约
    let all = DataScopeFilter::all();
    assert!(all.is_all());
    assert!(!all.is_none());
    assert!(all.customer_ids.is_empty());
    assert!(all.customer_id.is_none());
    assert!(all.issued_by.is_none());

    assert!(!DataScopeFilter::none().is_all());
    assert!(DataScopeFilter::none().is_none());

    let cs = DataScopeFilter::customer_scope(vec![1, 2, 3]);
    assert!(!cs.is_all() && !cs.is_none());
    assert_eq!(cs.customer_ids, vec![1, 2, 3]);
    assert!(cs.customer_id.is_none());

    let sc = DataScopeFilter::single_customer(42);
    assert_eq!(sc.customer_id, Some(42));
    assert!(sc.customer_ids.is_empty());

    let si = DataScopeFilter::self_issued(7);
    assert_eq!(si.issued_by, Some(7));
    assert!(si.customer_id.is_none());
}
