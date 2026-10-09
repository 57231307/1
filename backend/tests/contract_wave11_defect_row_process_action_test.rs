//! 不合格品台账「行内处置结果更新」（D1②/D2 后端半）五条契约锁
//!
//! 锁定的契约面（每条对应真 PostgreSQL 落库/回读证明，非源码文本比对）：
//! 1. 正向原地更新：pending 行调用 `process_unqualified_result` 后**本行被更新**
//!    （handling_status 推进至词表常量 approved、handling_method 落用户所选值），
//!    且 `unqualified_products` 行数不变——"不新增行"是本缺陷（原 /process 重复
//!    开同号单）的核心判据；单号逐字符不变；audit_log 前后全行快照可回读
//!    （before=pending / after=approved、操作人=会话用户）；
//! 2. 门控分族（本仓已锁口径：状态门=BUSINESS、字段校验=VALIDATION）：
//!    approved/rejected 前驱 ⇒ BUSINESS_ERROR；报废审批流程中（pending_fin/
//!    pending_gm）⇒ BUSINESS_ERROR；词表内 scrap 走结果端点 ⇒ BUSINESS_ERROR
//!    （终态必须经两级审批，不许旁路）；词表外 handling_method（含大小写漂移）
//!    ⇒ VALIDATION_ERROR 且零半途写入，拒绝文案合法值清单由权威常量拼出；
//!    行不存在 ⇒ NOT_FOUND；纯空白理由 ⇒ VALIDATION_ERROR；
//! 3. 幂等守卫：同 inspection_id 已有非终态行时 `process_unqualified` 二次开单
//!    被 BUSINESS_ERROR 拒绝，行数不增（防 UQ{:08} 同号两行继续累积）；
//! 4. 操作人只认会话：请求体注入伪造身份键（updated_by/user_id/handling_by=B），
//!    DTO 类型层无这些键、serde 直接忽略；断 handling_by == Some(A) 且显式
//!    != Some(B)，审计痕 user_id 同判；
//! 5. D2 筛选下推真实生效：get_defects_list 按 inspection_id 等值筛选，正例命中
//!    行数==预期、负例得空集（改前该筛选不存在，本锁同时防回潮）。
//!
//! 夹具形态：真 PostgreSQL `setup_test_db`（已迁移库 + TRUNCATE 业务表，suppliers
//! 等种子参照表不清空），照抄 contract_wave11_concession_receiving_flow_test 范式；
//! 同测试二进制由 CI 以 `--test-threads=1` 串行执行，逐用例 setup 不互踩。

mod test_common;

use bingxi_backend::models::audit_log;
use bingxi_backend::models::status::quality_dyeing::{
    quality_handling, quality_inspection_result, quality_inspection_type,
};
use bingxi_backend::models::{quality_inspection_record, unqualified_product};
use bingxi_backend::services::quality_inspection_service::{
    HANDLING_DOWNGRADE_SALE, HANDLING_REWORK, HANDLING_SCRAP, ProcessResultRequest,
    ProcessUnqualifiedRequest, QualityInspectionQueryParams, QualityInspectionService,
};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    DbBackend, EntityTrait, PaginatorTrait, QueryFilter, Statement,
};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use test_common::setup_test_db;

/// 会话注入用户（服务端身份唯一来源）
const SESSION_A: i32 = 4101;
/// 攻击者在请求体里伪造的身份键值（必须被忽略）
const FORGED_B: i32 = 999;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn unique_suffix() -> i64 {
    Utc::now().timestamp_nanos_opt().unwrap()
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

/// 已迁移真库 + users 会话行（照抄让步接收通道夹具口径）
async fn seeded_db() -> DatabaseConnection {
    let db = setup_test_db().await;
    exec(
        &db,
        r#"INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
             (4101,'w11_drp_operator','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             (999,'w11_forged_identity','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#,
    )
    .await;
    db
}

fn service(db: &DatabaseConnection) -> QualityInspectionService {
    QualityInspectionService::new(Arc::new(db.clone()))
}

/// 播种一行不合格品台账（词表常量取权威值，不手写第二套 token）
async fn seed_unqualified(
    db: &DatabaseConnection,
    inspection_id: Option<i32>,
    handling_status: &str,
    handling_method: &str,
    scrap_approval_status: &str,
) -> unqualified_product::Model {
    unqualified_product::ActiveModel {
        unqualified_no: Set(format!("W11DRP-{}", unique_suffix())),
        inspection_id: Set(inspection_id),
        product_id: Set(1),
        unqualified_qty: Set(dec("10.00")),
        unqualified_reason: Set("染色缸差超标".to_string()),
        handling_method: Set(handling_method.to_string()),
        handling_status: Set(handling_status.to_string()),
        scrap_approval_status: Set(scrap_approval_status.to_string()),
        grade: Set(Some("C".to_string())),
        stock_grade_synced: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种不合格品台账行失败")
}

async fn seed_pending(db: &DatabaseConnection) -> unqualified_product::Model {
    seed_unqualified(
        db,
        Some(77001),
        quality_handling::PENDING,
        HANDLING_REWORK,
        unqualified_product::SCRAP_NOT_REQUIRED,
    )
    .await
}

async fn reread(db: &DatabaseConnection, id: i32) -> unqualified_product::Model {
    unqualified_product::Entity::find_by_id(id)
        .one(db)
        .await
        .expect("回读不合格品行失败")
        .expect("不合格品行必须存在")
}

async fn row_count(db: &DatabaseConnection) -> u64 {
    unqualified_product::Entity::find()
        .count(db)
        .await
        .expect("统计台账行数失败")
}

fn result_req(method: &str, reason: Option<&str>) -> ProcessResultRequest {
    ProcessResultRequest {
        handling_method: method.to_string(),
        reason: reason.map(|r| Some(r.to_string())),
    }
}

// =========================================================
// 锁1：pending 行原地更新成功，行数不变，审计前后快照可回读
// =========================================================

#[tokio::test]
async fn lock1_pending_row_updated_in_place_without_new_row() {
    let db = seeded_db().await;
    let row = seed_pending(&db).await;
    let count_before = row_count(&db).await;
    let original_no = row.unqualified_no.clone();

    service(&db)
        .process_unqualified_result(row.id, result_req(HANDLING_DOWNGRADE_SALE, None), SESSION_A)
        .await
        .expect("pending 行原地更新处置结果必须成功");

    let count_after = row_count(&db).await;
    assert_eq!(
        count_after, count_before,
        "本缺陷核心判据：结果端点绝不允许新增行（前={count_before} 后={count_after}）"
    );

    let updated = reread(&db, row.id).await;
    assert_eq!(
        updated.handling_status,
        quality_handling::APPROVED,
        "处置完成后 handling_status 必须推进为词表常量 approved（逐字符）"
    );
    assert_eq!(
        updated.handling_method, HANDLING_DOWNGRADE_SALE,
        "处理方式必须落用户所选词表值"
    );
    assert_eq!(
        updated.unqualified_no, original_no,
        "原地更新不得改写单号（单号只由开单流程派生）"
    );
    assert_eq!(
        updated.handling_by,
        Some(SESSION_A),
        "处置操作人必须落真实列"
    );
    assert!(updated.handling_at.is_some(), "处置时间必须落真实列");
    assert!(
        updated.remark.is_none(),
        "理由/操作人严禁被挪用写入 remark（S-2 语义挪列红线）"
    );
    assert!(
        updated.handling_result.is_none(),
        "handling_result 既有语义是结果数据（降级单价/工时/损失），禁挪用承载理由"
    );

    let logs = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceId.eq(row.id.to_string()))
        .all(&db)
        .await
        .expect("查询审计日志失败");
    let snap = logs
        .iter()
        .find(|log| {
            let before = log
                .before_snapshot
                .as_ref()
                .map(|v| {
                    v.0.get("handling_status")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            let after = log
                .after_snapshot
                .as_ref()
                .map(|v| {
                    v.0.get("handling_status")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            before == quality_handling::PENDING && after == quality_handling::APPROVED
        })
        .expect("必须存在一条 audit_log：before=pending、after=approved 的全行快照");
    assert_eq!(
        snap.user_id,
        Some(SESSION_A),
        "审计痕操作人必须可回读且等于会话用户"
    );
}

// =========================================================
// 锁2：门控分族——非法前驱/旁路报废=BUSINESS；词表外/空白理由=VALIDATION；
//       行不存在=NOT_FOUND；被拒时零半途写入
// =========================================================

#[tokio::test]
async fn lock2_status_gate_is_business_and_field_gate_is_validation() {
    let db = seeded_db().await;

    // 终态前驱 approved / rejected ⇒ BUSINESS_ERROR，displayable 文案禁含记录 ID
    for terminal in [quality_handling::APPROVED, quality_handling::REJECTED] {
        let row = seed_unqualified(
            &db,
            Some(77002),
            terminal,
            HANDLING_REWORK,
            unqualified_product::SCRAP_NOT_REQUIRED,
        )
        .await;
        let err = service(&db)
            .process_unqualified_result(row.id, result_req(HANDLING_REWORK, None), SESSION_A)
            .await
            .expect_err(&format!("终态 {terminal} 行再次处置必须被拒"));
        assert_eq!(
            err.error_code(),
            "BUSINESS_ERROR",
            "状态门必须归 BUSINESS_ERROR，实得: {}",
            err.error_code()
        );
        let body = err.to_response();
        assert!(
            !body.message.contains(&row.id.to_string()),
            "displayable 拒绝文案禁含记录 ID，实得: {}",
            body.message
        );
    }

    // 词表外处理方式（含大小写漂移）⇒ VALIDATION_ERROR 且零半途写入
    let row = seed_pending(&db).await;
    let err = service(&db)
        .process_unqualified_result(row.id, result_req("REWORK", None), SESSION_A)
        .await
        .expect_err("大小写漂移的「REWORK」不在词表，必须被拒");
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "字段校验必须归 VALIDATION_ERROR，实得: {}",
        err.error_code()
    );
    let expected_allowed = [HANDLING_REWORK, HANDLING_DOWNGRADE_SALE, HANDLING_SCRAP].join("/");
    assert!(
        err.to_response().message.contains(&expected_allowed),
        "合法值清单必须由词表常量拼出（逐字符等于 {expected_allowed}），禁手写第二套字面量"
    );
    let untouched = reread(&db, row.id).await;
    assert_eq!(
        untouched.handling_status,
        quality_handling::PENDING,
        "被拒后状态不得被半途改写"
    );
    assert_eq!(untouched.handling_by, None, "被拒后操作人列必须仍为 NULL");

    // 词表内 scrap 走结果端点 ⇒ BUSINESS（终态必须经两级审批，不许旁路）
    let err = service(&db)
        .process_unqualified_result(row.id, result_req(HANDLING_SCRAP, None), SESSION_A)
        .await
        .expect_err("结果端点直进报废必须被拒");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "实得: {}",
        err.error_code()
    );

    // 报废审批流程中（pending_fin）的行 ⇒ BUSINESS（出口只有审批端点）
    let in_flow = seed_unqualified(
        &db,
        Some(77003),
        quality_handling::PENDING,
        HANDLING_SCRAP,
        unqualified_product::SCRAP_PENDING_FIN,
    )
    .await;
    let err = service(&db)
        .process_unqualified_result(in_flow.id, result_req(HANDLING_REWORK, None), SESSION_A)
        .await
        .expect_err("报废审批流程中的行必须禁止原地改判");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "实得: {}",
        err.error_code()
    );
    let still = reread(&db, in_flow.id).await;
    assert_eq!(
        still.scrap_approval_status,
        unqualified_product::SCRAP_PENDING_FIN,
        "被拒后审批流状态不得漂移"
    );

    // 行不存在 ⇒ NOT_FOUND
    let err = service(&db)
        .process_unqualified_result(999_999_999, result_req(HANDLING_REWORK, None), SESSION_A)
        .await
        .expect_err("不存在的行必须 404 语义被拒");
    assert_eq!(err.error_code(), "NOT_FOUND", "实得: {}", err.error_code());

    // 纯空白理由 ⇒ VALIDATION_ERROR（理由可选，但提交即不得为空白）
    let err = service(&db)
        .process_unqualified_result(row.id, result_req(HANDLING_REWORK, Some("   ")), SESSION_A)
        .await
        .expect_err("纯空白理由必须被拒");
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "实得: {}",
        err.error_code()
    );
}

// =========================================================
// 锁3：幂等守卫——同 inspection_id 非终态行存在时二次开单被拒且不增行
// =========================================================

#[tokio::test]
async fn lock3_second_issue_for_same_inspection_is_rejected_without_new_row() {
    let db = seeded_db().await;
    let record = quality_inspection_record::ActiveModel {
        inspection_no: Set(format!("QC-W11DRP-{}", unique_suffix())),
        // 词表同源：状态/类型/结论全部取权威常量，不手写第二套 token
        inspection_type: Set(quality_inspection_type::INCOMING.to_string()),
        product_id: Set(1),
        inspection_date: Set(date(2026, 10, 1)),
        total_qty: Set(dec("100")),
        inspected_qty: Set(dec("100")),
        inspection_result: Set(quality_inspection_result::UNQUALIFIED.to_string()),
        grade: Set(Some("C".to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("播种质检记录失败");

    let issue_req = || ProcessUnqualifiedRequest {
        unqualified_qty: dec("5"),
        unqualified_reason: "染色缸差超标".to_string(),
        handling_method: HANDLING_REWORK.to_string(),
        remark: None,
        handling_result: None,
    };
    let count_before = row_count(&db).await;

    let first = service(&db)
        .process_unqualified(record.id, issue_req(), SESSION_A)
        .await
        .expect("首次开单必须成功");

    let err = service(&db)
        .process_unqualified(record.id, issue_req(), SESSION_A)
        .await
        .expect_err("同质检记录存在非终态行时二次开单必须被拒");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "幂等拒绝必须归 BUSINESS_ERROR，实得: {}",
        err.error_code()
    );
    assert!(
        !err.to_response().message.contains(&record.id.to_string()),
        "displayable 拒绝文案禁含记录 ID，实得: {}",
        err.to_response().message
    );
    assert_eq!(
        row_count(&db).await,
        count_before + 1,
        "被拒的二次开单不得落任何行（同号两行累积是本缺陷根因）"
    );

    let same_no = unqualified_product::Entity::find()
        .filter(unqualified_product::Column::UnqualifiedNo.eq(first.unqualified_no.clone()))
        .all(&db)
        .await
        .expect("按单号回查失败");
    assert_eq!(
        same_no.len(),
        1,
        "单号 UQ{:08} 只允许一行——重复调用落同号两行即判红",
        record.id
    );

    // 终态化后允许再次开单（守卫判据是"非终态"，终态用词表 approved 判，不自造）
    service(&db)
        .process_unqualified_result(
            first.id,
            result_req(HANDLING_REWORK, Some("返工完成复检合格")),
            SESSION_A,
        )
        .await
        .expect("pending 行结果更新必须成功");
    service(&db)
        .process_unqualified(record.id, issue_req(), SESSION_A)
        .await
        .expect("既有行已终态（approved）时再次开单应被放行");
}

// =========================================================
// 锁4：操作人只认会话——body 伪造身份键在 DTO 类型层不存在、serde 忽略
// =========================================================

#[tokio::test]
async fn lock4_operator_derived_from_session_never_from_body() {
    let db = seeded_db().await;
    let row = seed_pending(&db).await;

    let payload = json!({
        "handling_method": HANDLING_REWORK,
        "reason": "缸差复检后确认可返工",
        "updated_by": FORGED_B,
        "user_id": FORGED_B,
        "handling_by": FORGED_B,
        "operator_id": FORGED_B,
    });
    let req: ProcessResultRequest =
        serde_json::from_value(payload).expect("结果请求反序列化必须成功（身份键被忽略）");
    service(&db)
        .process_unqualified_result(row.id, req, SESSION_A)
        .await
        .expect("会话用户更新 pending 行必须成功");

    let updated = reread(&db, row.id).await;
    assert_eq!(
        updated.handling_by,
        Some(SESSION_A),
        "处置操作人必须等于会话用户 A"
    );
    assert_ne!(
        updated.handling_by,
        Some(FORGED_B),
        "伪造身份 B 落库即红（body 身份键必须在类型层不存在）"
    );

    let logs = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceId.eq(row.id.to_string()))
        .all(&db)
        .await
        .expect("查询审计日志失败");
    let snap = logs
        .iter()
        .find(|log| {
            log.before_snapshot.as_ref().map(|v| {
                v.0.get("handling_status")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            }) == Some(quality_handling::PENDING)
        })
        .expect("必须存在处置前快照的审计行");
    assert_eq!(snap.user_id, Some(SESSION_A), "审计痕操作人只认会话");
    assert_ne!(snap.user_id, Some(FORGED_B), "审计痕不得为伪造身份");
}

// =========================================================
// 锁5：D2 筛选下推——record_id→inspection_id 等值筛选真实生效（正反例）
// =========================================================

#[tokio::test]
async fn lock5_record_id_filter_pushes_down_to_inspection_id() {
    let db = seeded_db().await;
    seed_unqualified(
        &db,
        Some(88001),
        quality_handling::PENDING,
        HANDLING_REWORK,
        unqualified_product::SCRAP_NOT_REQUIRED,
    )
    .await;
    seed_unqualified(
        &db,
        Some(88001),
        quality_handling::APPROVED,
        HANDLING_DOWNGRADE_SALE,
        unqualified_product::SCRAP_NOT_REQUIRED,
    )
    .await;
    seed_unqualified(
        &db,
        Some(88002),
        quality_handling::PENDING,
        HANDLING_REWORK,
        unqualified_product::SCRAP_NOT_REQUIRED,
    )
    .await;

    let params = QualityInspectionQueryParams {
        inspection_type: None,
        status: None,
        inspection_id: Some(88001),
        page: 1,
        page_size: 10,
    };
    let (rows, total) = service(&db)
        .get_defects_list(params)
        .await
        .expect("inspection_id 筛选查询必须成功");
    assert_eq!(
        total, 2,
        "正例：inspection_id=88001 必须命中 2 行，实得 {total}"
    );
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter().all(|r| r.inspection_id == Some(88001)),
        "命中行的 inspection_id 必须逐行等于筛选值（筛选失效即回潮判红）"
    );

    let params = QualityInspectionQueryParams {
        inspection_type: None,
        status: None,
        inspection_id: Some(88999),
        page: 1,
        page_size: 10,
    };
    let (rows, total) = service(&db)
        .get_defects_list(params)
        .await
        .expect("负例查询必须成功");
    assert_eq!(total, 0, "负例：不存在的 inspection_id 必须得空集");
    assert!(rows.is_empty());

    // 组合筛选：inspection_id + status 两条件同时生效（AND 语义）
    let params = QualityInspectionQueryParams {
        inspection_type: None,
        status: Some(quality_handling::PENDING.to_string()),
        inspection_id: Some(88001),
        page: 1,
        page_size: 10,
    };
    let (rows, total) = service(&db)
        .get_defects_list(params)
        .await
        .expect("组合筛选查询必须成功");
    assert_eq!(total, 1, "88001+pending 组合必须只剩 1 行");
    assert_eq!(rows[0].handling_status, quality_handling::PENDING);
}

// =========================================================
// 锁6：处置理由三态落库真库回读——有值=覆盖、缺席=保持原值、显式null=清空
// =========================================================

#[tokio::test]
async fn lock6_reason_present_persists_trimmed_value() {
    let db = seeded_db().await;
    let row = seed_pending(&db).await;

    let payload = json!({
        "handling_method": HANDLING_REWORK,
        "reason": "  缸差返工后复检合格  "
    });
    let req: ProcessResultRequest = serde_json::from_value(payload).expect("反序列化必须成功");
    service(&db)
        .process_unqualified_result(row.id, req, SESSION_A)
        .await
        .expect("带理由处置必须成功");

    let updated = reread(&db, row.id).await;
    assert_eq!(
        updated.handling_reason.as_deref(),
        Some("缸差返工后复检合格"),
        "理由 trim 后必须落库"
    );
}

#[tokio::test]
async fn lock6_reason_absent_preserves_original_value() {
    let db = seeded_db().await;
    let mut row = seed_pending(&db).await;

    // 先手动写入一个 reason 模拟"原值已存在"
    let mut am: unqualified_product::ActiveModel = row.clone().into();
    am.handling_reason = Set(Some("原始理由".to_string()));
    row = am.update(&db).await.expect("预置原值必须成功");
    assert_eq!(row.handling_reason.as_deref(), Some("原始理由"));

    // 构造不含 reason 键的 JSON（缺席态）
    let payload = json!({
        "handling_method": HANDLING_DOWNGRADE_SALE
    });
    let req: ProcessResultRequest =
        serde_json::from_value(payload).expect("缺席 reason 反序列化必须成功");
    service(&db)
        .process_unqualified_result(row.id, req, SESSION_A)
        .await
        .expect("reason 缺席时处置仍应成功");

    let updated = reread(&db, row.id).await;
    assert_eq!(
        updated.handling_reason.as_deref(),
        Some("原始理由"),
        "键缺席时原值不得被覆盖或清空"
    );
}

#[tokio::test]
async fn lock6_reason_explicit_null_clears_to_none() {
    let db = seeded_db().await;
    let mut row = seed_pending(&db).await;

    // 预置 reason 非 NULL
    let mut am: unqualified_product::ActiveModel = row.clone().into();
    am.handling_reason = Set(Some("待清空".to_string()));
    row = am.update(&db).await.expect("预置原值必须成功");
    assert_eq!(row.handling_reason.as_deref(), Some("待清空"));

    // 构造 reason=null 的 JSON（显式 null 态）
    let payload = json!({
        "handling_method": HANDLING_REWORK,
        "reason": null
    });
    let req: ProcessResultRequest =
        serde_json::from_value(payload).expect("显式 null reason 反序列化必须成功");
    service(&db)
        .process_unqualified_result(row.id, req, SESSION_A)
        .await
        .expect("显式 null 清空理由处置必须成功");

    let updated = reread(&db, row.id).await;
    assert_eq!(
        updated.handling_reason, None,
        "显式 null 必须落库为 SQL NULL"
    );
}
