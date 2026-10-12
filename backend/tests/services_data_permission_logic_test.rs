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

// ===== 写门行为锁（与 utils/data_scope.rs 真实实现同源）=====
// "超管跨 owner 写"（data_scope.rs:179-214）与"公海领取/回收"的写侧边界：
// contract_wave7_crm_read_vs_write_gate:416 只做函数体字符串比对，handler 级活体锁
// 在 AppState::default() 的 Disconnected 哨兵下 sea-orm 直接 panic 而非 Err，
// 两者都覆盖不到行为面。本组用例直接调用纯函数门，不依赖任何连接，把两条口径的
// **行为面**钉死；活体端到端复验依赖真库夹具环境，不据此宣称已验证。

fn ctx(
    scope: bingxi_backend::utils::data_scope::DataScope,
    dept_ids: Vec<i32>,
) -> bingxi_backend::utils::data_scope::DataScopeContext {
    bingxi_backend::utils::data_scope::DataScopeContext {
        scope,
        user_id: 50,
        department_id: Some(1),
        dept_ids,
        dept_member_user_ids: vec![50],
    }
}

/// 本人行恒可写：三种 scope 一律放行，且**先于** behalf 判定（data_scope.rs:203-205）。
/// 特别地：All 用户 + 未持代操作键写自己的行必须 true（"仅非本人行"判据的写侧镜像）。
#[test]
fn write_gate_own_row_always_allowed_regardless_of_behalf() {
    use bingxi_backend::utils::data_scope::{DataScope, check_resource_write_owner};

    for scope in [DataScope::All, DataScope::Dept, DataScope::Self_] {
        let c = ctx(scope, vec![1]);
        assert!(
            check_resource_write_owner(&c, Some(50), None, false),
            "{scope:?} 本人行（owner==user）无 behalf 也必须放行"
        );
        assert!(
            check_resource_write_owner(&c, Some(50), Some(20), true),
            "{scope:?} 本人行有 behalf 同样放行（判据在 behalf 之前短路）"
        );
    }
}

/// 超管跨 owner 写 = **All 档仅凭 behalf_granted**（data_scope.rs:207）：
/// 持 `crm_data/cross_owner_write` 键才放行，未授予一律拒——可见性(All)不等于可变更权。
/// 公海行（owner_id=0，Some(0)）在 All 档同样受该键门控：0 != 本人 ⇒ 走 behalf 判定。
#[test]
fn write_gate_all_scope_requires_explicit_behalf_key() {
    use bingxi_backend::utils::data_scope::{DataScope, check_resource_write_owner};

    let admin = ctx(DataScope::All, vec![1]);
    assert!(
        check_resource_write_owner(&admin, Some(60), Some(1), true),
        "All + behalf_granted=TRUE 跨 owner 写放行（方案 A 授权面）"
    );
    assert!(
        !check_resource_write_owner(&admin, Some(60), Some(1), false),
        "All 未持代操作键跨 owner 写必须拒——把可读升级成可写的旁路正是水平越权根因"
    );
    assert!(
        !check_resource_write_owner(&admin, Some(0), Some(1), false),
        "公海行（owner_id=0）写侧不随读侧放开：All 无 behalf 也不能直改无归属行"
    );
    assert!(
        check_resource_write_owner(&admin, Some(0), Some(1), true),
        "公海行持代操作键放行（领取走 claim 专用入口、直改走本门，两判定同源 behalf 词表）"
    );
    // owner 缺失（None）不是"本人行"：不得被 is_some_and 短路误放（:203）
    assert!(
        !check_resource_write_owner(&admin, None, Some(1), false),
        "owner=None 且无 behalf 必须拒（fail-closed）"
    );
}

/// Dept 跨 owner 写 = 资源部门 ∈ 可见部门集合（部门经理代管本部门是 dept 本职，
/// 不需额外键，data_scope.rs:208-211）；部门缺失/越部门一律拒。
#[test]
fn write_gate_dept_scope_uses_visible_dept_set_not_behalf() {
    use bingxi_backend::utils::data_scope::{DataScope, check_resource_write_owner};

    let mgr = ctx(DataScope::Dept, vec![1, 2]);
    assert!(
        check_resource_write_owner(&mgr, Some(60), Some(2), false),
        "Dept 本部门（∈可见集）他人行放行且不要求 behalf"
    );
    assert!(
        !check_resource_write_owner(&mgr, Some(60), Some(20), false),
        "Dept 越部门他人行拒绝"
    );
    assert!(
        !check_resource_write_owner(&mgr, Some(60), None, true),
        "Dept 资源部门 None 拒绝——即使误传 behalf=true 也不放行（behalf 只作用于 All 档）"
    );
}

/// Self_ 档跨 owner 恒拒（data_scope.rs:212）；behalf/部门命中都不得松动。
/// 公海行（Some(0)）对 Self 档同样恒拒——"领取"必须走 claim 专用入口而非直改。
#[test]
fn write_gate_self_scope_never_crosses_owner() {
    use bingxi_backend::utils::data_scope::{DataScope, check_resource_write_owner};

    let own = ctx(DataScope::Self_, vec![1]);
    assert!(
        !check_resource_write_owner(&own, Some(60), Some(1), true),
        "Self_ 他人行恒拒，behalf 不越档"
    );
    assert!(
        !check_resource_write_owner(&own, Some(0), Some(1), true),
        "Self_ 公海行恒拒（公海直改旁路防线）"
    );
    assert!(
        !check_resource_write_owner(&own, None, Some(1), true),
        "Self_ owner 缺失恒拒（fail-closed）"
    );
}

/// 读写门分家不对称锁：同一输入在 `check_resource_owner`（读门）与
/// `check_resource_write_owner`（写门）结果**必须不同**的两组边界——
/// 防止把读门当写门用（读侧可见 ≠ 写侧可变更，两门判据不同源）。
#[test]
fn read_gate_and_write_gate_diverge_on_all_no_behalf_and_self_pool() {
    use bingxi_backend::utils::data_scope::{
        DataScope, check_resource_owner, check_resource_write_owner,
    };

    // All 无 behalf 读他人行：读门放行（可见）而写门拒（不可变更）
    let admin = ctx(DataScope::All, vec![1]);
    assert!(
        check_resource_owner(&admin, Some(60), Some(9)),
        "读门 All 恒真（:161）"
    );
    assert!(
        !check_resource_write_owner(&admin, Some(60), Some(9), false),
        "写门 All 无 behalf 拒（:207）——读写分家即此不对称"
    );

    // Dept 本人行但资源部门 None：读门拒（Dept 无本人豁免），写门放行（本人短路先于档位）
    let mgr = ctx(DataScope::Dept, vec![1]);
    assert!(
        !check_resource_owner(&mgr, Some(50), None),
        "读门 Dept：本人行部门 None 也拒（:166）"
    );
    assert!(
        check_resource_write_owner(&mgr, Some(50), None, false),
        "写门：本人行恒可写（:203）"
    );
}
