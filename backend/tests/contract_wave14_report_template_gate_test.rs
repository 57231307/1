//! 报表模板可执行性门（ensure_template_runnable）活体行为锁
//!
//! 锁定的判据（`services/report/tpl.rs` 的 `ensure_template_runnable`，被
//! `handlers/report_engine_handler.rs` 的 execute_report / export_report 在真正跑查询之前调用，
//! handler 以 `?` 原样透传返回的 `AppError`，故门函数产出的 status/机器码即 HTTP 出参）：
//! - 随代码发布的预定义模板 id 一律放行；
//! - 库内 is_public = true 的自定义模板对非创建者也放行；
//! - 库内 is_public = false 且 created_by ≠ 会话用户的模板拒绝为权限错误（HTTP 403 / FORBIDDEN）；
//! - 完全未登记的 template_id 拒绝为未找到（HTTP 404 / NOT_FOUND），不得被当成可执行模板或压成 500；
//! - 按 code 回查的 fallback 分支（template_id 未命中时改按 code 查）与主分支共享同一可见性判定。
//!
//! 每条断言都判到具体 HTTP 状态与稳定机器码，而非仅 is_err()：
//! 门一旦被放宽成"恒放行"，403/404 用例会因拿到 Ok 而红；
//! 门一旦被收紧成"未按 is_public/created_by 区分"，owner 私有放行与 public 非 owner 放行用例会因拿到 403 而红；
//! 门若把 code fallback 分支漏判（fallback 命中后直接放行、不校验可见性），forbidden-by-code 用例会因拿到 Ok 而红。

use std::sync::Arc;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bingxi_backend::models::report_template;
use bingxi_backend::services::report::ReportEngineService;
use bingxi_backend::utils::error::AppError;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};

mod test_common;
use test_common::setup_test_db;

/// 与测试会话主体区分的模板归属人 id（无外键约束，仅作可见性判定值）
const OWNER_ID: i32 = 7777;
/// 与 OWNER_ID 不同的测试会话用户 id
const SESSION_USER_ID: i32 = 9999;

/// 取失败错误出参的 HTTP 状态（`IntoResponse` 即生产 handler 透传后的出参链路）
fn http_status_of(err: &AppError) -> StatusCode {
    let resp: Response = err.clone().into_response();
    resp.status()
}

/// 种一行真实合法的 report_templates 记录（走 SeaORM 活动模型、镜像生产写入形态）
///
/// 列约束依据迁移实读：`name/code/report_type/columns/created_by` 为 NOT NULL 无默认，
/// `version` 列虽可空但实体模型为非空 `i32`——回查按模型解码时 NULL 会触发类型错，
/// 故 version 必须显式给非空值；`is_public` 由用例指定，`template_id` 传 None 即置空
/// （让 template_id 等值查询不命中、改由 code fallback 命中）。
async fn seed_template(
    db: &DatabaseConnection,
    name: &str,
    code: &str,
    template_id: Option<&str>,
    is_public: bool,
    created_by: i32,
) {
    report_template::ActiveModel {
        name: Set(name.to_string()),
        code: Set(code.to_string()),
        report_type: Set("custom".to_string()),
        template_id: Set(template_id.map(|s| s.to_string())),
        columns: Set(serde_json::json!([])),
        is_public: Set(is_public),
        status: Set("ACTIVE".to_string()),
        version: Set(1),
        created_by: Set(created_by),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| {
        panic!("真库种 report_templates 失败（应检查 NOT NULL 无默认列与 version 非空约束）: {e}")
    });
}

/// 判据①：随代码发布的预定义模板 id 放行（空业务表下无同名库行，命中必来自预定义分支）
#[tokio::test]
async fn predefined_template_id_is_allowed() {
    let db = Arc::new(setup_test_db().await);
    let svc = ReportEngineService::new(db);

    for id in [
        "sales_summary",
        "sales_detail",
        "top_products",
        "customer_analysis",
        "inventory_status",
        "inventory_turnover",
        "purchase_summary",
        "ar_aging",
        "profit_analysis",
    ] {
        let res = svc.ensure_template_runnable(id, SESSION_USER_ID).await;
        assert!(
            res.is_ok(),
            "预定义模板 {id} 必须放行；红了说明预定义短路分支被破坏（会误查库/误拒随代码发布的模板）"
        );
    }
}

/// 判据②：库内 is_public=true 的自定义模板对非创建者放行
/// （红了说明公开模板被误判为需归属，破坏"公开即可执行"契约）
#[tokio::test]
async fn public_custom_template_is_allowed_for_non_creator() {
    let db = setup_test_db().await;
    seed_template(
        &db,
        "公开自定义模板",
        "pub_custom_t",
        Some("pub_custom_t"),
        true,
        OWNER_ID,
    )
    .await;
    let svc = ReportEngineService::new(Arc::new(db));

    let res = svc
        .ensure_template_runnable("pub_custom_t", SESSION_USER_ID)
        .await;
    assert!(
        res.is_ok(),
        "is_public=true 的模板即使非本人创建也必须放行；红了说明公开判据丢失（会拒绝全体公开模板）"
    );
}

/// 判据③之正向对照：is_public=false 但 created_by==会话用户 → 放行
/// （红了说明归属放行被破坏，私有模板会连本人也无法执行）
#[tokio::test]
async fn own_private_template_is_allowed() {
    let db = setup_test_db().await;
    seed_template(
        &db,
        "本人私有模板",
        "own_priv_t",
        Some("own_priv_t"),
        false,
        SESSION_USER_ID,
    )
    .await;
    let svc = ReportEngineService::new(Arc::new(db));

    let res = svc
        .ensure_template_runnable("own_priv_t", SESSION_USER_ID)
        .await;
    assert!(
        res.is_ok(),
        "created_by==会话用户的私有模板必须放行；红了说明归属放行丢失（本人被拒执行自己建的模板）"
    );
}

/// 判据③之负向：is_public=false 且 created_by!=会话用户 → PermissionDenied，HTTP 403 / FORBIDDEN
/// （红了说明他人私有模板被放行=越权执行数据泄露；或被压成 500=状态码契约漂移）
#[tokio::test]
async fn others_private_template_is_denied_403() {
    let db = setup_test_db().await;
    seed_template(
        &db,
        "他人私有模板",
        "other_priv_t",
        Some("other_priv_t"),
        false,
        OWNER_ID,
    )
    .await;
    let svc = ReportEngineService::new(Arc::new(db));

    let err = svc
        .ensure_template_runnable("other_priv_t", SESSION_USER_ID)
        .await
        .expect_err("他人 is_public=false 的私有模板必须拒绝，不得放行");
    assert!(
        matches!(err, AppError::PermissionDenied(_)),
        "拒绝形态必须是 PermissionDenied（权限族），实际 {err:?}；换成 Business/Internal 即语义错并会改变 HTTP 状态"
    );
    assert_eq!(
        err.error_code(),
        "FORBIDDEN",
        "权限拒绝的机器码必须是 FORBIDDEN"
    );
    assert_eq!(
        http_status_of(&err),
        StatusCode::FORBIDDEN,
        "他人私有模板的拒绝必须是 HTTP 403；红了说明被放宽成放行或被压成 500"
    );
}

/// 判据④：完全未登记的 template_id → NotFound，HTTP 404 / NOT_FOUND
/// （红了说明未知模板被当成可执行=放行，或被压成 500=状态码契约漂移）
#[tokio::test]
async fn unregistered_template_id_is_not_found_404() {
    let db = Arc::new(setup_test_db().await);
    let svc = ReportEngineService::new(db);

    let err = svc
        .ensure_template_runnable("w14-never-registered", SESSION_USER_ID)
        .await
        .expect_err("未登记模板必须拒绝，不得被当成可执行模板");
    assert!(
        matches!(err, AppError::NotFound(_)),
        "未登记模板必须是 NotFound 族，实际 {err:?}；换成 Internal/Database 即把未知资源压成 500"
    );
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "未登记的机器码必须是 NOT_FOUND"
    );
    assert_eq!(
        http_status_of(&err),
        StatusCode::NOT_FOUND,
        "未登记模板必须是 HTTP 404，而非被当成可执行模板或裸 500"
    );
}

/// fallback 分支正向：template_id 未命中、按 code 命中且公开 → 放行
/// （红了说明 code 回查未接进门，或回查命中后被误拒）
#[tokio::test]
async fn code_fallback_public_is_allowed() {
    let db = setup_test_db().await;
    // template_id 置空 → 按 template_id 等值查询不命中，只有 code 查询命中本行
    seed_template(
        &db,
        "按code回查公开模板",
        "code_fallback_pub",
        None,
        true,
        OWNER_ID,
    )
    .await;
    let svc = ReportEngineService::new(Arc::new(db));

    let res = svc
        .ensure_template_runnable("code_fallback_pub", SESSION_USER_ID)
        .await;
    assert!(
        res.is_ok(),
        "code 回查命中且公开必须放行；红了说明 code fallback 分支缺失或对回查行误判可见性"
    );
}

/// fallback 分支负向：template_id 未命中、按 code 命中但为他人私有 → 403 / FORBIDDEN
/// （红了说明 code 回查分支绕过了可见性判定=越权，或被压成 500=契约漂移）
#[tokio::test]
async fn code_fallback_others_private_is_denied_403() {
    let db = setup_test_db().await;
    seed_template(
        &db,
        "按code回查他人私有模板",
        "code_fallback_priv",
        None,
        false,
        OWNER_ID,
    )
    .await;
    let svc = ReportEngineService::new(Arc::new(db));

    let err = svc
        .ensure_template_runnable("code_fallback_priv", SESSION_USER_ID)
        .await
        .expect_err("code 回查命中的他人私有模板必须拒绝");
    assert!(
        matches!(err, AppError::PermissionDenied(_)),
        "fallback 命中行仍须走可见性判定，拒绝形态为 PermissionDenied，实际 {err:?}"
    );
    assert_eq!(err.error_code(), "FORBIDDEN");
    assert_eq!(
        http_status_of(&err),
        StatusCode::FORBIDDEN,
        "code fallback 分支必须与主分支同判据（403），不得绕过可见性判定直接放行"
    );
}
