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
    let hidden_fields = Some(vec![
        "cost_price".to_string(),
        "secret_field".to_string(),
    ]);

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
