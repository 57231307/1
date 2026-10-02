use bingxi_backend::utils::data_scope::DataScope;
use bingxi_backend::utils::data_scope::*;

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
        dept_ids: vec![],
        dept_member_user_ids: vec![],
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
        dept_ids: vec![10],
        dept_member_user_ids: vec![1, 999],
    };
    assert!(check_resource_owner(&ctx, Some(999), Some(10)));
}

#[test]
fn test_check_resource_owner_dept_bmbppfh_false() {
    let ctx = DataScopeContext {
        scope: DataScope::Dept,
        user_id: 1,
        department_id: Some(10),
        dept_ids: vec![10],
        dept_member_user_ids: vec![1, 999],
    };
    assert!(!check_resource_owner(&ctx, Some(1), Some(20)));
}

#[test]
fn test_check_resource_owner_dept_zywbmfh_false() {
    let ctx = DataScopeContext {
        scope: DataScope::Dept,
        user_id: 1,
        department_id: Some(10),
        dept_ids: vec![10],
        dept_member_user_ids: vec![1, 999],
    };
    assert!(!check_resource_owner(&ctx, Some(1), None));
}

#[test]
fn test_check_resource_owner_dept_yhwbmthw_false() {
    // 用户无可见部门集合时，dept 范围无法匹配，返回 false
    let ctx = DataScopeContext {
        scope: DataScope::Dept,
        user_id: 1,
        department_id: None,
        dept_ids: vec![],
        dept_member_user_ids: vec![],
    };
    assert!(!check_resource_owner(&ctx, Some(1), Some(10)));
}

#[test]
fn test_check_resource_owner_self_gsrppfh_true() {
    let ctx = DataScopeContext {
        scope: DataScope::Self_,
        user_id: 1,
        department_id: Some(10),
        dept_ids: vec![],
        dept_member_user_ids: vec![],
    };
    assert!(check_resource_owner(&ctx, Some(1), Some(20)));
}

#[test]
fn test_check_resource_owner_self_gsrbppfh_false() {
    let ctx = DataScopeContext {
        scope: DataScope::Self_,
        user_id: 1,
        department_id: Some(10),
        dept_ids: vec![],
        dept_member_user_ids: vec![],
    };
    assert!(!check_resource_owner(&ctx, Some(999), Some(10)));
}

#[test]
fn test_check_resource_owner_self_zywgsrfh_false() {
    let ctx = DataScopeContext {
        scope: DataScope::Self_,
        user_id: 1,
        department_id: Some(10),
        dept_ids: vec![],
        dept_member_user_ids: vec![],
    };
    assert!(!check_resource_owner(&ctx, None, Some(10)));
}

// ===== build_data_scope_condition Dept 分支（m_rls_dept_domain 成员集合语义）=====

use bingxi_backend::models::{customer, sales_order};
use bingxi_backend::utils::data_scope::{
    build_data_scope_condition, build_department_scope_condition,
};

fn condition_sql(condition: &sea_orm::sea_query::Condition) -> String {
    use sea_orm::sea_query::{Cond, PostgresQueryBuilder, Query};
    // 借 SELECT 空表把 Condition 序列化为 SQL 串，取 WHERE 之后的部分校验谓词形态
    let sql = Query::select()
        .expr(1i32)
        .cond_where(Cond::all().add(condition.clone()))
        .to_string(PostgresQueryBuilder);
    sql.split_once(" WHERE ")
        .map(|(_, after)| after.to_string())
        .unwrap_or_default()
}

#[test]
fn test_dept_condition_filters_by_member_user_ids() {
    let ctx = DataScopeContext {
        scope: DataScope::Dept,
        user_id: 1,
        department_id: Some(10),
        dept_ids: vec![10],
        dept_member_user_ids: vec![1, 7, 9],
    };
    // 非 RLS 表：owner 列（created_by）IN 成员集合
    let cond = build_data_scope_condition(
        &ctx,
        sales_order::Column::CreatedBy,
        sales_order::Column::DepartmentId,
    );
    let sql = condition_sql(&cond);
    assert!(
        sql.contains("IN (1, 7, 9)") || sql.contains("IN (1,7,9)"),
        "Dept 分支应按成员用户集合过滤，实际: {sql}"
    );
}

#[test]
fn test_dept_condition_empty_members_degrades_to_self() {
    let ctx = DataScopeContext {
        scope: DataScope::Dept,
        user_id: 1,
        department_id: None,
        dept_ids: vec![],
        dept_member_user_ids: vec![],
    };
    let cond = build_data_scope_condition(
        &ctx,
        sales_order::Column::CreatedBy,
        sales_order::Column::DepartmentId,
    );
    let sql = condition_sql(&cond);
    assert!(
        sql.contains("= 1"),
        "无可见成员应退化为 self（created_by = 本人），实际: {sql}"
    );
}

#[test]
fn test_department_scope_condition_or_combination() {
    let ctx = DataScopeContext {
        scope: DataScope::Dept,
        user_id: 1,
        department_id: Some(10),
        dept_ids: vec![10, 11],
        dept_member_user_ids: vec![1, 7],
    };
    // RLS 表：本人行 OR department_id IN 可见部门集合（与策略 USING 同形态）
    let cond = build_department_scope_condition(
        &ctx,
        customer::Column::OwnerId,
        customer::Column::DepartmentId,
    );
    let sql = condition_sql(&cond);
    assert!(
        sql.contains("OR") && sql.contains("IN (10, 11)"),
        "RLS 表 Dept 分支应为 self OR department_id IN 集合，实际: {sql}"
    );
}

// ===== apply_department_scope_with_pool 组合形态（拍板 ②：crm_lead=Open，customers=Scoped）=====

use bingxi_backend::models::crm_lead;
use bingxi_backend::utils::data_scope::build_department_scope_with_pool_condition;

fn dept_ctx() -> DataScopeContext {
    DataScopeContext {
        scope: DataScope::Dept,
        user_id: 55,
        department_id: Some(1),
        dept_ids: vec![10, 11],
        dept_member_user_ids: vec![1, 7],
    }
}

fn self_ctx() -> DataScopeContext {
    DataScopeContext {
        scope: DataScope::Self_,
        user_id: 60,
        department_id: Some(1),
        dept_ids: vec![],
        dept_member_user_ids: vec![60],
    }
}

/// Open（crm_lead）：归属条件与公海条件 **OR** 组合——Dept 档形如
/// `(self/成员过滤) OR lead_status='pool'`，公海行可见性与归属条件相互独立
#[test]
fn test_open_pool_visibility_or_combines_dept() {
    let pool = crm_lead::Column::LeadStatus.eq("pool");
    let cond = build_department_scope_with_pool_condition(
        &dept_ctx(),
        crm_lead::Column::OwnerId,
        crm_lead::Column::DepartmentId,
        pool,
        PoolVisibility::Open,
    );
    let sql = condition_sql(&cond);
    assert!(
        sql.contains("lead_status") && sql.contains("'pool'"),
        "Open 组合必须携带公海条件，实际: {sql}"
    );
    // 顶层谓词必须是 OR（公海条件独立成支），绝不允许 AND 遮蔽
    let top_is_or = sql.trim_start_matches('(').contains(" OR ")
        && !sql.contains(" AND lead_status")
        && !sql.contains(" AND (lead_status");
    assert!(
        top_is_or,
        "Open 组合的公海条件必须与归属条件 OR（与 RLS USING 同形态），实际: {sql}"
    );
}

/// Open · Self 档：`owner_id=本人 OR lead_status='pool'`（修复前 Self 分支无
/// 公海放行，公海页 self 用户 0 行——拍板 ② 的纠偏点）
#[test]
fn test_open_pool_visibility_allows_self_branch_pool() {
    let pool = crm_lead::Column::LeadStatus.eq("pool");
    let cond = build_department_scope_with_pool_condition(
        &self_ctx(),
        crm_lead::Column::OwnerId,
        crm_lead::Column::DepartmentId,
        pool,
        PoolVisibility::Open,
    );
    let sql = condition_sql(&cond);
    assert!(
        sql.contains("= 60") && sql.contains("'pool'") && sql.contains(" OR "),
        "Self 档 Open 应为 本人行 OR 公海行，实际: {sql}"
    );
}

/// Open · All 档：行级不过滤（空 Condition），公海放行无从也不需要
#[test]
fn test_open_pool_visibility_all_unfiltered() {
    let ctx = DataScopeContext {
        scope: DataScope::All,
        user_id: 70,
        department_id: None,
        dept_ids: vec![],
        dept_member_user_ids: vec![],
    };
    let pool = crm_lead::Column::LeadStatus.eq("pool");
    let cond = build_department_scope_with_pool_condition(
        &ctx,
        crm_lead::Column::OwnerId,
        crm_lead::Column::DepartmentId,
        pool,
        PoolVisibility::Open,
    );
    let sql = condition_sql(&cond);
    assert!(
        !sql.contains("owner_id") && !sql.contains("lead_status"),
        "All 档不应有任何行级过滤，实际: {sql}"
    );
}

/// Scoped（customers 本轮口径）：**逐字符保持历史组合**——公海条件仅 Dept 档
/// AND 进归属条件；Self/All 无公海分支。客户公海是否改 Open 另待拍板。
#[test]
fn test_scoped_pool_visibility_preserves_historical_shape() {
    let pool = customer::Column::OwnerId.eq(0);
    let dept_sql = condition_sql(&build_department_scope_with_pool_condition(
        &dept_ctx(),
        customer::Column::OwnerId,
        customer::Column::DepartmentId,
        customer::Column::OwnerId.eq(0),
        PoolVisibility::Scoped,
    ));
    assert!(
        dept_sql.contains(" AND ") && dept_sql.contains("owner_id") && dept_sql.contains("= 0"),
        "Scoped·Dept 应维持 归属条件 AND 公海条件 的历史形态，实际: {dept_sql}"
    );
    let self_sql = condition_sql(&build_department_scope_with_pool_condition(
        &self_ctx(),
        customer::Column::OwnerId,
        customer::Column::DepartmentId,
        pool,
        PoolVisibility::Scoped,
    ));
    assert!(
        !self_sql.contains("= 0"),
        "Scoped·Self 不应出现公海放行（可见面零变化），实际: {self_sql}"
    );
}
