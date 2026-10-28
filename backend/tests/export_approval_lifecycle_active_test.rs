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
use bingxi_backend::utils::error::AppError;
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

/// 条件1 反向：令牌只能由被批准的申请人本人使用（防转发复用）
#[tokio::test]
async fn token_is_rejected_when_caller_is_not_the_approved_applicant() {
    let db = test_common::setup_test_db().await;
    let svc = service(db.clone());
    // 种子行的申请人是 7
    let _id = seed_approved_row(&db, 0, 3).await;

    // 反例：同令牌同资源同范围，但调用者是另一个人（8）→ 点名身份不符
    let other = svc
        .enforce_export_download_secured(
            Some(PLAINTEXT_TOKEN),
            APPROVED_RESOURCE,
            8,
            Some(serde_json::from_str(APPROVED_PARAMS).unwrap()),
        )
        .await;
    assert!(other.is_err(), "非申请人本人使用他人令牌必须被拒");
    assert!(
        other
            .unwrap_err()
            .to_string()
            .contains("只能由被批准的申请人本人使用"),
        "身份不符的拒绝要点名是身份问题，不能只回一句不可下载"
    );

    // 正例：申请人本人调用同令牌同范围 → 放行（证明上一行的拒因确实是身份而非别的前置门）
    let self_call = svc
        .enforce_export_download_secured(
            Some(PLAINTEXT_TOKEN),
            APPROVED_RESOURCE,
            7,
            Some(serde_json::from_str(APPROVED_PARAMS).unwrap()),
        )
        .await;
    assert!(
        self_call.is_ok(),
        "申请人本人按批准范围取数应放行：{:?}",
        self_call.err().map(|e| e.to_string())
    );
}

/// 条件1 反向：请求范围比批准范围宽、缺维度或改取值，三类都要点名拒绝
#[tokio::test]
async fn request_scope_drifting_from_approved_scope_is_rejected_by_name() {
    let db = test_common::setup_test_db().await;
    let svc = service(db.clone());
    let _id = seed_approved_row(&db, 0, 3).await;
    let caller = 7;

    // 反向一：新增了批准外的维度（把筛选条件去掉就是放大范围）
    let wider = svc
        .enforce_export_download_secured(
            Some(PLAINTEXT_TOKEN),
            APPROVED_RESOURCE,
            caller,
            Some(serde_json::json!({"status": "active", "keyword": "x"})),
        )
        .await;
    assert!(wider.is_err(), "批准外新增维度必须被拒");
    let wider_msg = wider.unwrap_err().to_string();
    assert!(
        wider_msg.contains("超出批准范围") && wider_msg.contains("keyword"),
        "超范围拒绝要点名新增了哪个维度，实际文案：{wider_msg}"
    );

    // 反向二：丢了批准时已有的维度（空范围等于全量导出）
    let missing = svc
        .enforce_export_download_secured(
            Some(PLAINTEXT_TOKEN),
            APPROVED_RESOURCE,
            caller,
            Some(serde_json::json!({})),
        )
        .await;
    assert!(missing.is_err(), "丢掉批准维度必须被拒（空范围等于放大到全量）");
    let missing_msg = missing.unwrap_err().to_string();
    assert!(
        missing_msg.contains("丢失了批准时的维度") && missing_msg.contains("status"),
        "缺维度拒绝要点名丢了哪个维度，实际文案：{missing_msg}"
    );

    // 反向三：同名维度改了取值（把已批准的状态换成另一个状态）
    let changed = svc
        .enforce_export_download_secured(
            Some(PLAINTEXT_TOKEN),
            APPROVED_RESOURCE,
            caller,
            Some(serde_json::json!({"status": "inactive"})),
        )
        .await;
    assert!(changed.is_err(), "改变批准时的取值必须被拒");
    let changed_msg = changed.unwrap_err().to_string();
    assert!(
        changed_msg.contains("改变了批准时的取值") && changed_msg.contains("status"),
        "改取值拒绝要点名是哪个维度，实际文案：{changed_msg}"
    );

    // 正例：逐项相等 → 放行
    let exact = svc
        .enforce_export_download_secured(
            Some(PLAINTEXT_TOKEN),
            APPROVED_RESOURCE,
            caller,
            Some(serde_json::from_str(APPROVED_PARAMS).unwrap()),
        )
        .await;
    assert!(
        exact.is_ok(),
        "与批准范围逐项相等应放行：{:?}",
        exact.err().map(|e| e.to_string())
    );
}

/// 导出腿失败要落失败留痕，并且错误原文不被记账吞掉
#[tokio::test]
async fn export_leg_failure_is_recorded_and_error_is_passed_through() {
    let db = test_common::setup_test_db().await;
    let svc = service(db.clone());
    let id = seed_approved_row(&db, 0, 3).await;

    // 受理门放行一次（登记首次尝试），随后导出腿取数失败
    svc.enforce_export_download_secured(
        Some(PLAINTEXT_TOKEN),
        APPROVED_RESOURCE,
        7,
        Some(serde_json::from_str(APPROVED_PARAMS).unwrap()),
    )
    .await
    .expect("本人按批准范围首次受理应放行");

    let failing_leg = async { Err::<(), AppError>(AppError::business("模拟取数失败")) };
    let outcome = svc.run_export_leg(id, failing_leg).await;
    assert!(
        outcome
            .unwrap_err()
            .to_string()
            .contains("模拟取数失败"),
        "记账腿必须把导出错误原样交回，不得换成别的文案"
    );

    let row = Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("申请行应存在");
    let ctx = row.context.expect("失败留痕应写进 context");
    let last_failure = ctx.0.get("last_failure").expect("context 应有 last_failure 留痕");
    assert_eq!(
        last_failure.get("reason").and_then(|v| v.as_str()),
        Some("模拟取数失败"),
        "留痕要记下失败原因原文，实际：{last_failure}"
    );
    assert_eq!(
        row.retry_count, 1,
        "失败登记要把重试计数推进一次（首次尝试本身不计数）"
    );
    assert_eq!(
        row.status,
        ApprovalStatus::Approved.as_str(),
        "额度未尽时失败不得改写审批状态"
    );

    // 失败已计入重试额度：5 分钟窗口内的下一次受理被窗口门挡住
    let retry = svc
        .enforce_export_download_secured(
            Some(PLAINTEXT_TOKEN),
            APPROVED_RESOURCE,
            7,
            Some(serde_json::from_str(APPROVED_PARAMS).unwrap()),
        )
        .await;
    assert!(retry.is_err(), "窗口未到重试必须被拒");
    assert!(
        retry.unwrap_err().to_string().contains("重试窗口"),
        "窗口未到拒绝应点名重试窗口"
    );
}

/// 导出腿成功不落失败留痕，也不动重试计数
#[tokio::test]
async fn export_leg_success_records_no_failure() {
    let db = test_common::setup_test_db().await;
    let svc = service(db.clone());
    let id = seed_approved_row(&db, 0, 3).await;

    svc.enforce_export_download_secured(
        Some(PLAINTEXT_TOKEN),
        APPROVED_RESOURCE,
        7,
        Some(serde_json::from_str(APPROVED_PARAMS).unwrap()),
    )
    .await
    .expect("本人按批准范围首次受理应放行");

    let ok_leg = async { Ok("已生成文件".to_string()) };
    let value = svc
        .run_export_leg(id, ok_leg)
        .await
        .expect("成功腿的原值要原样交回");
    assert_eq!(value, "已生成文件", "成功腿不得改写执行体的产出");

    let row = Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("申请行应存在");
    assert!(row.context.is_none(), "成功路径不应写失败留痕");
    assert_eq!(row.retry_count, 0, "成功路径不应推进重试计数");
}

/// 失败把重试额度用尽时即落 exhausted 终态，后续受理按终态拒绝
#[tokio::test]
async fn export_leg_failure_at_last_budget_lands_exhausted_terminal() {
    let db = test_common::setup_test_db().await;
    let svc = service(db.clone());
    // retry_count 2、max 3：再失败一次就用尽
    let id = seed_approved_row(&db, 2, 3).await;

    let failing_leg = async { Err::<(), AppError>(AppError::business("第三次仍失败")) };
    assert!(
        svc.run_export_leg(id, failing_leg).await.is_err(),
        "失败腿必须把错误交回调用方"
    );

    let row = Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("申请行应存在");
    assert_eq!(
        row.status,
        ApprovalStatus::Exhausted.as_str(),
        "用尽后应落 exhausted 终态"
    );
    assert_eq!(row.retry_count, 3, "用尽时计数应顶到上限");

    // 终态后再走受理门：令牌校验侧按终态拒绝，文案点名该行状态
    let after = svc
        .enforce_export_download_secured(
            Some(PLAINTEXT_TOKEN),
            APPROVED_RESOURCE,
            7,
            Some(serde_json::from_str(APPROVED_PARAMS).unwrap()),
        )
        .await;
    assert!(after.is_err(), "已用尽令牌再受理必须被拒");
    let after_msg = after.unwrap_err().to_string();
    assert!(
        after_msg.contains("exhausted"),
        "终态后的拒绝要点名该行已是 exhausted，实际文案：{after_msg}"
    );
}
