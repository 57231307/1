//! 行级/字段级权限测试（batch-12 P2-9）
//!
//! 测试 DataPermissionService 的核心功能：
//! - 行级权限：apply_data_scope 根据 data_scope 过滤查询
//! - 字段级权限：filter_fields / filter_fields_batch 根据 allowed_fields/hidden_fields 过滤字段

use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 测试 filter_fields 白名单模式
#[test]
fn test_filter_fields_whitelist() {
    use bingxi_backend::services::data_permission_service::DataPermissionService;
    use serde_json::json;

    let db = Arc::new(DatabaseConnection::default());
    let service = DataPermissionService::new(db);

    let mut data = json!({
        "id": 1,
        "name": "测试产品",
        "code": "P001",
        "cost_price": 100.00,
        "secret_field": "敏感数据"
    });

    let allowed_fields = Some(vec![
        "id".to_string(),
        "name".to_string(),
        "code".to_string(),
    ]);
    let hidden_fields = None;

    service.filter_fields(&mut data, &allowed_fields, &hidden_fields);

    // 白名单模式：只保留允许的字段
    assert!(data.get("id").is_some());
    assert!(data.get("name").is_some());
    assert!(data.get("code").is_some());
    assert!(data.get("cost_price").is_none());
    assert!(data.get("secret_field").is_none());
}

/// 测试 filter_fields 黑名单模式
#[test]
fn test_filter_fields_blacklist() {
    use bingxi_backend::services::data_permission_service::DataPermissionService;
    use serde_json::json;

    let db = Arc::new(DatabaseConnection::default());
    let service = DataPermissionService::new(db);

    let mut data = json!({
        "id": 1,
        "name": "测试产品",
        "code": "P001",
        "cost_price": 100.00,
        "secret_field": "敏感数据"
    });

    let allowed_fields = None;
    let hidden_fields = Some(vec!["cost_price".to_string(), "secret_field".to_string()]);

    service.filter_fields(&mut data, &allowed_fields, &hidden_fields);

    // 黑名单模式：移除隐藏的字段
    assert!(data.get("id").is_some());
    assert!(data.get("name").is_some());
    assert!(data.get("code").is_some());
    assert!(data.get("cost_price").is_none());
    assert!(data.get("secret_field").is_none());
}

/// 测试 filter_fields 无过滤
#[test]
fn test_filter_fields_no_filter() {
    use bingxi_backend::services::data_permission_service::DataPermissionService;
    use serde_json::json;

    let db = Arc::new(DatabaseConnection::default());
    let service = DataPermissionService::new(db);

    let mut data = json!({
        "id": 1,
        "name": "测试产品",
        "cost_price": 100.00
    });

    let allowed_fields = None;
    let hidden_fields = None;

    service.filter_fields(&mut data, &allowed_fields, &hidden_fields);

    // 无过滤：所有字段保留
    assert!(data.get("id").is_some());
    assert!(data.get("name").is_some());
    assert!(data.get("cost_price").is_some());
}

/// 测试 filter_fields_batch 批量过滤
#[test]
fn test_filter_fields_batch() {
    use bingxi_backend::services::data_permission_service::DataPermissionService;
    use serde_json::json;

    let db = Arc::new(DatabaseConnection::default());
    let service = DataPermissionService::new(db);

    let mut data_list = vec![
        json!({"id": 1, "name": "产品A", "cost_price": 100.00}),
        json!({"id": 2, "name": "产品B", "cost_price": 200.00}),
    ];

    let allowed_fields = Some(vec!["id".to_string(), "name".to_string()]);
    let hidden_fields = None;

    service.filter_fields_batch(&mut data_list, &allowed_fields, &hidden_fields);

    // 批量过滤：每个元素都只保留允许的字段
    for data in &data_list {
        assert!(data.get("id").is_some());
        assert!(data.get("name").is_some());
        assert!(data.get("cost_price").is_none());
    }
}

/// 测试 DataScope 枚举解析
#[test]
fn test_data_scope_parsing() {
    use bingxi_backend::utils::data_scope::DataScope;

    assert_eq!(DataScope::parse_scope("all"), DataScope::All);
    assert_eq!(DataScope::parse_scope("dept"), DataScope::Dept);
    assert_eq!(DataScope::parse_scope("self"), DataScope::Self_);
    assert_eq!(DataScope::parse_scope("unknown"), DataScope::Self_); // 默认为 self
}

/// 测试 DataScope as_str
#[test]
fn test_data_scope_as_str() {
    use bingxi_backend::utils::data_scope::DataScope;

    assert_eq!(DataScope::All.as_str(), "all");
    assert_eq!(DataScope::Dept.as_str(), "dept");
    assert_eq!(DataScope::Self_.as_str(), "self");
}

// ===== 数据范围归属门纯函数行为锁 =====
// 本节把 data_permission 侧断言与 `src/utils/data_scope.rs` 真实实现**同源**钉死，
// 覆盖"公海领取/超管跨 owner 写"两条口径在纯函数层的行为。纯函数锁不依赖任何
// 连接/AppState，可确定性执行；handler 级端到端锁由
// `contract_wave6_crm_pool_owner_test.rs` 承担——本节不构成两条口径的端到端验证，
// 不得据本节宣称已端到端。

/// 读门 `check_resource_owner` 三档 + 边界，逐分支对照实现（data_scope.rs:155-177）：
/// - `All` 恒真（可见性语义，与写门分家，:161）；
/// - `Dept` 判据 = 资源部门 ∈ 可见部门集合；**owner=本人但部门列 None 也拒**
///   （:164-167，Dept 读门无"本人行豁免"支，与写门 :203-205 刻意不对称，如实钉死）；
/// - `Self_` 判据 = owner 与本人匹配；owner None 拒（:169-175）。
#[test]
fn check_resource_owner_read_gate_matches_impl() {
    use bingxi_backend::utils::data_scope::{DataScope, DataScopeContext, check_resource_owner};

    let ctx = |scope: DataScope, dept_ids: Vec<i32>| DataScopeContext {
        scope,
        user_id: 50,
        department_id: Some(1),
        dept_ids,
        dept_member_user_ids: vec![50],
    };

    let all = ctx(DataScope::All, vec![1]);
    assert!(check_resource_owner(&all, Some(999), Some(999)), "All 恒真");
    assert!(check_resource_owner(&all, None, None), "All 对空归属也真");

    let dept = ctx(DataScope::Dept, vec![1, 2]);
    assert!(
        check_resource_owner(&dept, Some(999), Some(2)),
        "部门∈可见集放行"
    );
    assert!(
        !check_resource_owner(&dept, Some(999), Some(20)),
        "部门∉可见集拒绝"
    );
    assert!(
        !check_resource_owner(&dept, Some(50), None),
        "Dept 读门无本人豁免：本人行但部门列为 None 也拒绝（实现 :166，与写门不对称）"
    );

    let own = ctx(DataScope::Self_, vec![1]);
    assert!(
        check_resource_owner(&own, Some(50), Some(20)),
        "Self 本人行放行"
    );
    assert!(
        !check_resource_owner(&own, Some(51), Some(1)),
        "Self 他人行拒绝"
    );
    assert!(
        !check_resource_owner(&own, None, Some(1)),
        "Self owner 缺失 fail-closed 拒绝"
    );
}

/// 词表同源锁：`data_permissions.scope_type` 的**存储词表**（服务侧大写常量，
/// `services/data_permission_service.rs:19-31`）必须能被
/// `DataScope::parse_scope`（`utils/data_scope.rs:29-35`，大小写不敏感、未知值
/// fail-closed 回退 Self_）正确解析；`CUSTOM` 现实现回退 Self_，本条**如实钉现状**，
/// 不预先放宽；若该档位将来要改为独立语义，本条变红即为口径变更的显式触发点。
#[test]
fn scope_type_vocabulary_shared_by_service_and_data_scope() {
    use bingxi_backend::services::data_permission_service::data_scope;
    use bingxi_backend::utils::data_scope::DataScope;

    assert_eq!(DataScope::parse_scope(data_scope::ALL), DataScope::All);
    assert_eq!(DataScope::parse_scope(data_scope::DEPT), DataScope::Dept);
    assert_eq!(DataScope::parse_scope(data_scope::SELF), DataScope::Self_);
    assert_eq!(
        DataScope::parse_scope(data_scope::CUSTOM),
        DataScope::Self_,
        "CUSTOM 现实现无独立档位，fail-closed 回退 Self_（data_scope.rs:33）"
    );

    // 三档往返：as_str 的输出必须原样被 parse_scope 解析回同一档
    for s in [DataScope::All, DataScope::Dept, DataScope::Self_] {
        assert_eq!(
            DataScope::parse_scope(s.as_str()),
            s,
            "as_str↔parse_scope 往返"
        );
    }

    // 空串与垃圾值同样 fail-closed 到最严档（实现 :33，权限面禁止默认放行）
    assert_eq!(DataScope::parse_scope(""), DataScope::Self_);
    assert_eq!(DataScope::parse_scope("Everybody"), DataScope::Self_);
}
