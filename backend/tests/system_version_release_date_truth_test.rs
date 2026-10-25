//! 版本信息面发布日期与更新说明的真实性锁（钉死不再把请求时刻冒充发布日期、不再把 NULL 折叠成空串）
//!
//! 夹具形态：真 PostgreSQL setup_test_db（已迁移库 + 逐用例 TRUNCATE 业务表）。
//! system_version 由迁移建表但不播种，不在 test_common SEALED_REFERENCE_TABLES 之列，
//! 故每个用例开始时该表为空，由用例自行插入当前版本行后断言，避免依赖外部种子行。
//! 本文件断言依赖真库，CI 侧注入 TEST_DATABASE_URL 执行；缺该环境变量时 setup_test_db 如实 panic。

mod test_common;

use axum::Json;
use axum::extract::State;
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::system_update_handler;
use bingxi_backend::models::system_version;
use sea_orm::{ActiveModelTrait, Set};
use std::sync::Arc;
use test_common::setup_test_db;

/// 构造一个当前版本行：发布日期取固定的远早于今天的时刻，更新说明取一份真实文本。
async fn seed_current_version(
    db: &sea_orm::DatabaseConnection,
    release_date: chrono::DateTime<chrono::Utc>,
    changelog: Option<String>,
) {
    let model = system_version::ActiveModel {
        version: Set("2020.1.1.0000".to_string()),
        release_date: Set(release_date),
        changelog: Set(changelog),
        is_current: Set(true),
        created_at: Set(release_date),
        updated_at: Set(release_date),
        ..Default::default()
    };
    model.insert(db).await.expect("插入当前版本行应成功");
}

/// 固定的历史发布日期（远早于任何可能的请求时刻），用于证明出口不是取 now。
fn historic_release_date() -> chrono::DateTime<chrono::Utc> {
    chrono::NaiveDate::from_ymd_opt(2020, 1, 1)
        .expect("构造历史日期")
        .and_hms_opt(8, 30, 0)
        .expect("构造历史时刻")
        .and_utc()
}

/// 行为锁一：库中有当前版本行时，出口返回的 release_date 与 changelog 来自该行的真实列值，
/// 绝不等于请求当天，也不是空串。
#[tokio::test]
async fn version_info_returns_db_release_date_not_today() {
    let db = setup_test_db().await;
    seed_current_version(
        &db,
        historic_release_date(),
        Some("修复导出乱码".to_string()),
    )
    .await;
    let state = AppState {
        db: Arc::new(db),
        ..AppState::default()
    };

    let Json(api) = system_update_handler::get_version(State(state))
        .await
        .expect("只读版本端点不应报错");
    let resp = api.data.expect("成功信封应带 data");

    assert_eq!(
        resp.release_date.as_deref(),
        Some("2020-01-01"),
        "发布日期应取库中当前行的真实值，而非请求时刻"
    );
    assert_eq!(
        resp.changelog.as_deref(),
        Some("修复导出乱码"),
        "更新说明应取库中当前行的真实值，而非恒 None"
    );

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    assert_ne!(
        resp.release_date.as_deref(),
        Some(today.as_str()),
        "发布日期不得回落为请求当天"
    );
}

/// 行为锁二：库中当前版本行的 changelog 为 NULL 时，出口如实返回 null（未知），
/// 不得被折叠成空串冒充有内容；同时证明 release_date 仍读真实列值。
#[tokio::test]
async fn version_info_keeps_null_changelog_as_unknown_not_empty() {
    let db = setup_test_db().await;
    seed_current_version(&db, historic_release_date(), None).await;
    let state = AppState {
        db: Arc::new(db),
        ..AppState::default()
    };

    let Json(api) = system_update_handler::get_version(State(state))
        .await
        .expect("只读版本端点不应报错");
    let resp = api.data.expect("成功信封应带 data");

    assert_eq!(
        resp.release_date.as_deref(),
        Some("2020-01-01"),
        "发布日期应取库中当前行的真实值"
    );
    assert!(
        resp.changelog.is_none(),
        "库中 changelog 为 NULL 时出口应为未知（None），不得折叠成空串，实际: {:?}",
        resp.changelog
    );
    assert_ne!(
        resp.changelog.as_deref(),
        Some(""),
        "NULL 与空串语义不同：未知不得写成空串冒充有内容"
    );
}

/// 行为锁三：库中无任何当前版本行时，release_date 与 changelog 均为 None（未知），
/// 证明出口未把请求时刻或空串当作缺省值掩盖缺键。
#[tokio::test]
async fn version_info_reports_unknown_when_no_current_row() {
    let db = setup_test_db().await;
    let state = AppState {
        db: Arc::new(db),
        ..AppState::default()
    };

    let Json(api) = system_update_handler::get_version(State(state))
        .await
        .expect("只读版本端点不应报错");
    let resp = api.data.expect("成功信封应带 data");

    assert!(
        resp.release_date.is_none(),
        "查无当前版本行时发布日期应为未知，实际: {:?}",
        resp.release_date
    );
    assert!(
        resp.changelog.is_none(),
        "查无当前版本行时更新说明应为未知，实际: {:?}",
        resp.changelog
    );
}
