//! 导出审批令牌生命周期 活体用例（真实 PostgreSQL）
//!
//! 生命周期四条各一条正向 + 关键反向，落库可查、状态回写真实发生：
//!   条件1 一个令牌只放行被批准的资源与数据范围（跨资源与超范围反例）；
//!   条件2 导出成功即作废，重放被拒；
//!   条件3 未导出成功后 5 分钟窗口内重试被拒；
//!   条件4 未成功前提下重试超过上限被拒并落 exhausted 终态。
//!
//! 活体由 CI 兑付，本地不跑（见 LANE_RULES 二A）：连接已迁移的 PostgreSQL（setup_test_db
//! 缺 TEST_DATABASE_URL 直接 panic，禁止 sqlite 兜底），用例之间 TRUNCATE 隔离。
//! 令牌哈希与 service 的 sha256_hex 同源复算，验令牌走 token_hash 命中路径。

mod test_common;

use bingxi_backend::models::export_approval_request::{
    ActiveModel as EaActiveModel, ApprovalStatus, Entity, ExportParams,
};
use bingxi_backend::services::export_approval_service::ExportApprovalService;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, EntityTrait, IntoActiveModel,
};
use std::sync::Arc;

/// 令牌明文（测试用固定串；哈希落库，验令牌走哈希路径）。明文不外发、不入日志。
const PLAINTEXT_TOKEN: &str = "exp_testtoken00000000000000000000abcd";
/// 批准的资源类型与数据范围（export_params 落库形状）
const APPROVED_RESOURCE: &str = "customer";
const APPROVED_PARAMS: &str = r#"{"status":"active"}"#;

/// 插入一行已批准、未消费、未尝试的导出审批（其余列取中性值），返回自增 id。
async fn seed_approved_row(
    db: &sea_orm::DatabaseConnection,
    retry_count: i32,
    max_retries: i32,
) -> i64 {
    let hash = ExportApprovalService::sha256_hex(PLAINTEXT_TOKEN.as_bytes());
    let params: serde_json::Value = serde_json::from_str(APPROVED_PARAMS).unwrap();
    let now = Utc::now().fixed_offset();
    let am = EaActiveModel {
        id: NotSet,
        applicant_user_id: Set(7),
        applicant_username: Set("applicant".to_string()),
        approver_user_id: Set(Some(9)),
        approver_username: Set(Some("approver".to_string())),
        resource_type: Set(APPROVED_RESOURCE.to_string()),
        export_params: Set(Some(ExportParams(params))),
        estimated_rows: Set(Some(10)),
        file_format: Set("xlsx".to_string()),
        status: Set(ApprovalStatus::Approved.as_str().to_string()),
        approval_level: Set(1),
        approver_comments: Set(None),
        approved_at: Set(Some(now)),
        rejected_at: Set(None),
        download_token: Set(None),
        token_expires_at: Set(Some(
            (Utc::now() + chrono::Duration::hours(24)).fixed_offset(),
        )),
        download_count: Set(0),
        max_downloads: Set(1),
        file_path: Set(None),
        file_size_bytes: Set(None),
        file_checksum: Set(None),
        applicant_ip: Set(None),
        approver_ip: Set(None),
        applicant_user_agent: Set(None),
        risk_level: Set("low".to_string()),
        context: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(None),
        token_hash: Set(Some(hash)),
        token_fingerprint: Set(Some("exp_testtok".to_string())),
        token_viewed: Set(false),
        retry_count: Set(retry_count),
        last_attempt_at: Set(None),
        max_retries: Set(max_retries),
        token_consumed: Set(false),
        token_consumed_at: Set(None),
        bpm_instance_id: Set(None),
    };
    let inserted = am.insert(db).await.expect("插入已批准导出审批行应成功");
    inserted.id
}

fn service(db: sea_orm::DatabaseConnection) -> ExportApprovalService {
    ExportApprovalService::new(Arc::new(db))
}

/// 条件1 反向：跨资源令牌被拒（令牌只对被批准的资源与范围有效）
#[tokio::test]
async fn token_is_bound_to_approved_resource_and_scope() {
    let db = test_common::setup_test_db().await;
    let svc = service(db.clone());
    let _id = seed_approved_row(&db, 0, 3).await;

    // 正例：批准资源 + 批准范围 → 放行
    let ok = svc
        .enforce_export_download_secured(
            Some(PLAINTEXT_TOKEN),
            APPROVED_RESOURCE,
            7,
            Some(serde_json::from_str(APPROVED_PARAMS).unwrap()),
        )
        .await;
    assert!(
        ok.is_ok(),
        "批准资源与范围应放行：{:?}",
        ok.err().map(|e| e.to_string())
    );

    // 反例：跨资源使用同一令牌 → 拒绝
    let cross = svc
        .enforce_export_download_secured(
            Some(PLAINTEXT_TOKEN),
            "supplier",
            7,
            Some(serde_json::from_str(APPROVED_PARAMS).unwrap()),
        )
        .await;
    assert!(cross.is_err(), "令牌跨资源复用必须被拒");
    assert!(
        cross.unwrap_err().to_string().contains("资源类型不匹配"),
        "跨资源拒绝应点名资源类型不匹配"
    );
}

/// 条件2：导出成功后令牌作废，重放被拒
#[tokio::test]
async fn token_invalidated_after_successful_export_blocks_replay() {
    let db = test_common::setup_test_db().await;
    let svc = service(db.clone());
    let id = seed_approved_row(&db, 0, 3).await;

    let model = svc
        .verify_download_token(PLAINTEXT_TOKEN)
        .await
        .expect("已批准未消费令牌应校验通过");
    assert_eq!(model.id, id);

    svc.record_download(id, "f.xlsx".to_string(), 128, "checksum".to_string())
        .await
        .expect("登记成功导出应成功");

    // 消费后再校验同一令牌 → 拒绝（一次性终态，禁复用）
    let replay = svc.verify_download_token(PLAINTEXT_TOKEN).await;
    assert!(replay.is_err(), "成功导出后重放同一令牌必须被拒");
    let consumed = Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("申请行应存在");
    assert!(consumed.token_consumed, "消费标记应落库");
    assert_eq!(consumed.status, ApprovalStatus::Consumed.as_str());
    // 哈希已清空（消费终态不再保留可校验凭据）
    assert!(consumed.token_hash.is_none(), "消费后哈希应清空");
}

/// 条件3 反向：上次尝试未满 5 分钟窗口，重试被拒
#[tokio::test]
async fn retry_before_five_minute_window_is_rejected() {
    let db = test_common::setup_test_db().await;
    let svc = service(db.clone());
    let id = seed_approved_row(&db, 0, 3).await;

    // 模拟一次刚发生的尝试：把 last_attempt_at 置为当前、retry_count 置 1
    let row = Entity::find_by_id(id).one(&db).await.unwrap().unwrap();
    let mut am = row.into_active_model();
    am.last_attempt_at = Set(Some(Utc::now().fixed_offset()));
    am.retry_count = Set(1);
    am.update(&db).await.expect("预置尝试时刻应成功");

    let result = svc
        .enforce_export_download(Some(PLAINTEXT_TOKEN), APPROVED_RESOURCE)
        .await;
    assert!(result.is_err(), "窗口未到重试必须被拒");
    assert!(
        result.unwrap_err().to_string().contains("重试窗口"),
        "窗口未到拒绝应点名重试窗口"
    );
}

/// 条件4 反向：未成功重试次数已达上限，再被拒并落 exhausted 终态
#[tokio::test]
async fn retry_beyond_max_attempts_is_rejected_and_exhausts() {
    let db = test_common::setup_test_db().await;
    let svc = service(db.clone());
    // 直接以 retry_count=max 且窗口已过预置：max=3
    let id = seed_approved_row(&db, 3, 3).await;
    let row = Entity::find_by_id(id).one(&db).await.unwrap().unwrap();
    let mut am = row.into_active_model();
    // last_attempt 远早于现在，确保命中「额度用尽」而非「窗口未到」
    am.last_attempt_at = Set(Some(
        (Utc::now() - chrono::Duration::minutes(30)).fixed_offset(),
    ));
    am.update(&db).await.expect("预置窗口应成功");

    let result = svc
        .enforce_export_download(Some(PLAINTEXT_TOKEN), APPROVED_RESOURCE)
        .await;
    assert!(result.is_err(), "超过重试上限必须被拒");
    assert!(
        result.unwrap_err().to_string().contains("用尽"),
        "超限拒绝应点名重试已用尽"
    );
    let after = Entity::find_by_id(id).one(&db).await.unwrap().unwrap();
    assert_eq!(
        after.status,
        ApprovalStatus::Exhausted.as_str(),
        "超限后应落 exhausted 终态"
    );
}
