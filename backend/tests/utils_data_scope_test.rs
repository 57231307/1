use bingxi_backend::utils::data_scope::*;
use bingxi_backend::utils::data_scope::DataScope;

// ===== DataScope::parse_scope 测试 =====

#[test]
fn test_data_scope_parse_scope_all() {
    assert_eq!(DataScope::parse_scope("all"), DataScope::All);
    assert_eq!(DataScope::parse_scope("ALL"), DataScope::All);
    assert_eq!(DataScope::parse_scope("All"), DataScope::All);
}

#[test]
fn test_data_scope_parse_scope_dept() {
    assert_eq!(DataScope::parse_scope("dept"), DataScope::Dept);
    assert_eq!(DataScope::parse_scope("DEPT"), DataScope::Dept);
}

#[test]
fn test_data_scope_parse_scope_self() {
    assert_eq!(DataScope::parse_scope("self"), DataScope::Self_);
    assert_eq!(DataScope::parse_scope("SELF"), DataScope::Self_);
}

#[test]
fn test_data_scope_parse_scope_wzzmr_self() {
    // 未知值应回退到 Self_（最小权限原则）
    assert_eq!(DataScope::parse_scope("unknown"), DataScope::Self_);
    assert_eq!(DataScope::parse_scope(""), DataScope::Self_);
    assert_eq!(DataScope::parse_scope("admin"), DataScope::Self_);
}

#[test]
fn test_data_scope_as_str() {
    assert_eq!(DataScope::All.as_str(), "all");
    assert_eq!(DataScope::Dept.as_str(), "dept");
    assert_eq!(DataScope::Self_.as_str(), "self");
}

// ===== check_resource_owner 测试 =====

#[test]
fn test_check_resource_owner_all_szfh_true() {
    let ctx = DataScopeContext {
        scope: DataScope::All,
        user_id: 1,
        department_id: Some(10),
    };
    // 无论资源归属如何，all 范围始终返回 true
    assert!(check_resource_owner(&ctx, Some(999), Some(999)));
    assert!(check_resource_owner(&ctx, None, None));
    assert!(check_resource_owner(&ctx, Some(1), Some(10)));
}

#[test]
fn test_check_resource_owner_dept_bmppfh_true() {
    let ctx = DataScopeContext {
        scope: DataScope::Dept,
        user_id: 1,
        department_id: Some(10),
    };
    assert!(check_resource_owner(&ctx, Some(999), Some(10)));
}

#[test]
fn test_check_resource_owner_dept_bmbppfh_false() {
    let ctx = DataScopeContext {
        scope: DataScope::Dept,
        user_id: 1,
        department_id: Some(10),
    };
    assert!(!check_resource_owner(&ctx, Some(1), Some(20)));
}

#[test]
fn test_check_resource_owner_dept_zywbmfh_false() {
    let ctx = DataScopeContext {
        scope: DataScope::Dept,
        user_id: 1,
        department_id: Some(10),
    };
    assert!(!check_resource_owner(&ctx, Some(1), None));
}

#[test]
fn test_check_resource_owner_dept_yhwbmthw_false() {
    // 用户无部门时，dept 范围无法匹配，返回 false
    let ctx = DataScopeContext {
        scope: DataScope::Dept,
        user_id: 1,
        department_id: None,
    };
    assert!(!check_resource_owner(&ctx, Some(1), Some(10)));
}

#[test]
fn test_check_resource_owner_self_gsrppfh_true() {
    let ctx = DataScopeContext {
        scope: DataScope::Self_,
        user_id: 1,
        department_id: Some(10),
    };
    assert!(check_resource_owner(&ctx, Some(1), Some(20)));
}

#[test]
fn test_check_resource_owner_self_gsrbppfh_false() {
    let ctx = DataScopeContext {
        scope: DataScope::Self_,
        user_id: 1,
        department_id: Some(10),
    };
    assert!(!check_resource_owner(&ctx, Some(999), Some(10)));
}

#[test]
fn test_check_resource_owner_self_zywgsrfh_false() {
    let ctx = DataScopeContext {
        scope: DataScope::Self_,
        user_id: 1,
        department_id: Some(10),
    };
    assert!(!check_resource_owner(&ctx, None, Some(10)));
}
