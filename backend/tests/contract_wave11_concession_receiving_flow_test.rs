//! 采购收货「让步接收 / 复检改判」通道五条契约锁（用户终裁落地）
//!
//! 锁定的契约面（每条对应一条真实落库/回读证明，非源码文本比对）：
//! 1. 合法让步转移成功且理由入库可读回——PENDING（收货待检）与 REJECTED（质检判
//!    不合格后特采）两个真实前驱都能进 CONCESSION_ACCEPTED；理由落专用真实列
//!    （concession_reason，禁挪用 notes），操作人取会话身份（请求体伪造的同名身份键
//!    在 DTO 类型层不存在、serde 直接忽略）；
//! 2. 缺理由 ⇒ VALIDATION_ERROR（null/空串/纯空白三形全覆盖），且零落库——目标行
//!    检验状态与理由列不得被半途改写；改判结论词表外值同样归 VALIDATION_ERROR
//!    （字段校验归 VALIDATION，本仓裁定）；
//! 3. 非法前驱 ⇒ BUSINESS_ERROR（状态门归 BUSINESS）：PASSED 不可让步、非让步态
//!    不可改判、已处让步态不可重复让步、已确认入库（receipt_status≠DRAFT）不可
//!    再进通道；displayable 拒绝文案禁含记录 ID；
//! 4. 复检改判成功且留痕——改判前状态（CONCESSION_ACCEPTED）与改判后状态（PASSED/
//!    REJECTED）、操作人都可回读：实体行走让步+改判后终态逐字段核对，历史前后态经
//!    既有审计范式 audit_log（update_with_audit 前后全行快照）取回；改判次数落
//!    真实列 rejudge_count 累加；
//! 5. 应用层词表 == DB CHECK 允许值集合（三重同源：运行时词表 ALL、活库
//!    pg_constraint 定义解析集、迁移原文 ALLOWED_SQL 锚点解析集，两两集合相等，
//!    不等必须判红）。
//!
//! 夹具形态：真 PostgreSQL `setup_test_db`（已迁移库 + TRUNCATE 业务表，suppliers
//! 等种子参照表不清空），FK 父行自种子口径照抄 contract_wave6_receipt_gate_test；
//! 同测试二进制由 CI 以 `--test-threads=1` 串行执行，逐用例 setup 不互踩。

mod test_common;

use bingxi_backend::models::audit_log;
use bingxi_backend::models::status::purchase_inventory::{
    purchase_inspection_result, purchase_receipt as pr_status, purchase_receipt_inspection,
};
use bingxi_backend::models::{purchase_receipt, user};
use bingxi_backend::services::purchase_receipt_dto::{
    ConcedeReceiptRequest, RejudgeReceiptRequest,
};
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    DbBackend, EntityTrait, QueryFilter, Statement,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
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

/// 已迁移真库 + 本域 FK 父行（suppliers 1 为种子参照表不清空；warehouses/users 自种）
async fn seeded_db() -> DatabaseConnection {
    let db = setup_test_db().await;
    exec(
        &db,
        r#"INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
             (4101,'w11_concede_operator','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             (999,'w11_forged_identity','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#,
    )
    .await;
    exec(
        &db,
        "INSERT INTO warehouses (id, name, warehouse_code, is_active) VALUES (1, '波11让步通道主仓', 'W11CON-W1', true)",
    )
    .await;
    db
}

async fn seed_receipt(
    db: &DatabaseConnection,
    inspection_status: &str,
    receipt_status: &str,
) -> purchase_receipt::Model {
    purchase_receipt::ActiveModel {
        receipt_no: Set(format!("GR-W11CON-{}", unique_suffix())),
        supplier_id: Set(1),
        receipt_date: Set(date(2026, 10, 1)),
        warehouse_id: Set(1),
        // 词表同源：状态取权威常量，不手写第二套 token
        inspection_status: Set(inspection_status.to_string()),
        receipt_status: Set(receipt_status.to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        total_amount: Set(dec("0.00")),
        created_by: Set(SESSION_A),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种入库单失败")
}

fn service(db: &DatabaseConnection) -> PurchaseReceiptService {
    PurchaseReceiptService::new(Arc::new(db.clone()))
}

async fn reread(db: &DatabaseConnection, id: i32) -> purchase_receipt::Model {
    purchase_receipt::Entity::find_by_id(id)
        .one(db)
        .await
        .expect("回读入库单失败")
        .expect("入库单必须存在")
}

/// 单引号 token 解析（口径同 contract_wave11_export_inspection_result_check_test
/// 的 ALLOWED_SQL 扫描器：逐对单引号取值，不做转义扩展——CHECK 清单与迁移清单均
/// 不含转义引号）。
fn parse_quoted_tokens(text: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    let mut cursor = text;
    while let Some(open) = cursor.find('\'') {
        cursor = &cursor[open + 1..];
        let close = cursor.find('\'').expect("单引号未闭合");
        set.insert(cursor[..close].to_string());
        cursor = &cursor[close + 1..];
    }
    set
}

// =========================================================
// 锁1：合法让步转移成功，理由入库可读回；身份只认会话
// =========================================================

#[tokio::test]
async fn lock1_concession_from_legal_predecessor_writes_reason_and_session_identity() {
    let db = seeded_db().await;
    // 前驱一：PENDING（收货即待检，用户终裁「收货时可选让步接收」）
    let pending = seed_receipt(&db, purchase_receipt_inspection::PENDING, pr_status::DRAFT).await;
    // 请求体故意夹带伪造身份键（concession_by/user_id），DTO 类型层无这些字段 ⇒ 忽略
    let payload = json!({
        "reason": " 缸差超出允许范围，双方协商降级用于二等品产线 ",
        "concession_by": FORGED_B,
        "user_id": FORGED_B,
    });
    let req: ConcedeReceiptRequest =
        serde_json::from_value(payload).expect("让步请求反序列化必须成功（身份键被忽略）");
    service(&db)
        .concede_receipt(pending.id, req, SESSION_A)
        .await
        .expect("PENDING→CONCESSION_ACCEPTED 合法转移必须成功");

    let row = reread(&db, pending.id).await;
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::CONCESSION_ACCEPTED,
        "让步后检验状态必须为 CONCESSION_ACCEPTED（词表常量逐字符）"
    );
    assert_eq!(
        row.concession_reason.as_deref(),
        Some("缸差超出允许范围，双方协商降级用于二等品产线"),
        "理由必须落专用真实列且为 trim 后原文（首尾空白不得入库）"
    );
    assert_ne!(
        row.notes.as_deref(),
        Some("缸差超出允许范围，双方协商降级用于二等品产线"),
        "理由严禁被挪用写入 notes"
    );
    assert_eq!(
        row.concession_by,
        Some(SESSION_A),
        "让步操作人必须等于会话用户 A、绝不等于请求体伪造的 B"
    );
    assert_ne!(row.concession_by, Some(FORGED_B), "伪造身份落库即红");
    assert!(row.concession_at.is_some(), "让步时间必须落真实列");

    // 前驱二：REJECTED（质检判不合格后特采）
    let rejected = seed_receipt(&db, purchase_receipt_inspection::REJECTED, pr_status::DRAFT).await;
    service(&db)
        .concede_receipt(
            rejected.id,
            ConcedeReceiptRequest {
                reason: Some("外观轻微瑕疵，客户书面确认可收".to_string()),
            },
            SESSION_A,
        )
        .await
        .expect("REJECTED→CONCESSION_ACCEPTED 合法转移必须成功");
    let row = reread(&db, rejected.id).await;
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::CONCESSION_ACCEPTED,
        "不合格特采后检验状态必须为 CONCESSION_ACCEPTED"
    );
}

// =========================================================
// 锁2：缺理由 / 词表外结论 ⇒ VALIDATION_ERROR 且零落库
// =========================================================

#[tokio::test]
async fn lock2_missing_reason_and_out_of_vocab_result_are_validation_with_zero_write() {
    let db = seeded_db().await;

    // 让步：null / 空串 / 纯空白 三形都必须被拒且归 VALIDATION_ERROR
    for missing in [None, Some(""), Some("   ")] {
        let receipt =
            seed_receipt(&db, purchase_receipt_inspection::PENDING, pr_status::DRAFT).await;
        let err = service(&db)
            .concede_receipt(
                receipt.id,
                ConcedeReceiptRequest {
                    reason: missing.map(str::to_string),
                },
                SESSION_A,
            )
            .await
            .expect_err("缺理由的让步请求必须被拒");
        assert_eq!(
            err.error_code(),
            "VALIDATION_ERROR",
            "缺理由必须归 VALIDATION_ERROR 族（字段校验归 VALIDATION），实得: {}",
            err.error_code()
        );
        let row = reread(&db, receipt.id).await;
        assert_eq!(
            row.inspection_status,
            purchase_receipt_inspection::PENDING,
            "被拒后检验状态必须保持 PENDING（零半途写入）"
        );
        assert_eq!(
            row.concession_reason, None,
            "被拒后理由列必须仍为 NULL（零半途写入）"
        );
        assert_eq!(row.concession_by, None, "被拒后操作人列必须仍为 NULL");
    }

    // 改判：结论词表外（大小写漂移即非法，逐字符强校验）⇒ VALIDATION_ERROR
    let conceded = seed_receipt(
        &db,
        purchase_receipt_inspection::CONCESSION_ACCEPTED,
        pr_status::DRAFT,
    )
    .await;
    let err = service(&db)
        .rejudge_receipt(
            conceded.id,
            RejudgeReceiptRequest {
                inspection_result: Some("PASS".to_string()),
                reason: Some("复检确认整批合格".to_string()),
            },
            SESSION_A,
        )
        .await
        .expect_err("词表外结论「PASS」必须被拒");
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "实得: {}",
        err.error_code()
    );
    let row = reread(&db, conceded.id).await;
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::CONCESSION_ACCEPTED,
        "词表外结论被拒后不得改写状态"
    );

    // 改判缺理由 ⇒ VALIDATION_ERROR（先于任何状态判定）
    let err = service(&db)
        .rejudge_receipt(
            conceded.id,
            RejudgeReceiptRequest {
                inspection_result: Some(purchase_inspection_result::PASS.to_string()),
                reason: Some("  ".to_string()),
            },
            SESSION_A,
        )
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
// 锁3：非法前驱 ⇒ BUSINESS_ERROR，文案禁含记录 ID
// =========================================================

#[tokio::test]
async fn lock3_illegal_predecessor_is_business_error_without_record_id() {
    let db = seeded_db().await;

    // PASSED 不可让步（已合格无需特采）
    let passed = seed_receipt(&db, purchase_receipt_inspection::PASSED, pr_status::DRAFT).await;
    let err = service(&db)
        .concede_receipt(
            passed.id,
            ConcedeReceiptRequest {
                reason: Some("试图对合格单让步".to_string()),
            },
            SESSION_A,
        )
        .await
        .expect_err("PASSED→CONCESSION 非法转移必须被拒");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "状态门必须归 BUSINESS_ERROR，实得: {}",
        err.error_code()
    );
    let body = err.to_response();
    assert!(
        !body.message.contains(&passed.id.to_string()),
        "displayable 拒绝文案禁含记录 ID，实得: {}",
        body.message
    );

    // 已处让步态不可重复让步（须先改判离开）
    let conceded = seed_receipt(
        &db,
        purchase_receipt_inspection::CONCESSION_ACCEPTED,
        pr_status::DRAFT,
    )
    .await;
    let err = service(&db)
        .concede_receipt(
            conceded.id,
            ConcedeReceiptRequest {
                reason: Some("重复让步".to_string()),
            },
            SESSION_A,
        )
        .await
        .expect_err("CONCESSION→CONCESSION 非法转移必须被拒");
    assert_eq!(err.error_code(), "BUSINESS_ERROR");

    // PENDING 直接改判非法（改判只从让步态出发）
    let pending = seed_receipt(&db, purchase_receipt_inspection::PENDING, pr_status::DRAFT).await;
    let err = service(&db)
        .rejudge_receipt(
            pending.id,
            RejudgeReceiptRequest {
                inspection_result: Some(purchase_inspection_result::PASS.to_string()),
                reason: Some("跳过让步直接改判".to_string()),
            },
            SESSION_A,
        )
        .await
        .expect_err("PENDING→改判 非法前驱必须被拒");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "实得: {}",
        err.error_code()
    );
    let body = err.to_response();
    assert!(
        !body.message.contains(&pending.id.to_string()),
        "改判非法前驱文案禁含记录 ID，实得: {}",
        body.message
    );

    // 已确认入库（COMPLETED）不可再进通道（越权状态流转防线）
    let completed = seed_receipt(
        &db,
        purchase_receipt_inspection::PENDING,
        pr_status::COMPLETED,
    )
    .await;
    let err = service(&db)
        .concede_receipt(
            completed.id,
            ConcedeReceiptRequest {
                reason: Some("对已完成单让步".to_string()),
            },
            SESSION_A,
        )
        .await
        .expect_err("已完成单据让步必须被拒");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "实得: {}",
        err.error_code()
    );
}

// =========================================================
// 锁4：复检改判成功且留痕（前后状态与操作人可回读）
// =========================================================

#[tokio::test]
async fn lock4_rejudge_succeeds_with_readback_before_after_state_and_operator() {
    let db = seeded_db().await;
    let receipt = seed_receipt(&db, purchase_receipt_inspection::PENDING, pr_status::DRAFT).await;

    // 前置：让步接收（改判的前驱态）
    service(&db)
        .concede_receipt(
            receipt.id,
            ConcedeReceiptRequest {
                reason: Some("让步接收用于改判链路".to_string()),
            },
            SESSION_A,
        )
        .await
        .expect("让步必须成功");

    // 改判：结论 pass 经同源映射 → PASSED；partial → REJECTED 的出边由词表映射钉住
    service(&db)
        .rejudge_receipt(
            receipt.id,
            RejudgeReceiptRequest {
                inspection_result: Some(purchase_inspection_result::PASS.to_string()),
                reason: Some("复检实测缸差在允收范围内".to_string()),
            },
            SESSION_A,
        )
        .await
        .expect("CONCESSION_ACCEPTED→PASSED 改判必须成功");

    let row = reread(&db, receipt.id).await;
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::PASSED,
        "改判后终态必须为 PASSED（pass→PASSED 同源映射）"
    );
    assert_eq!(
        row.rejudge_reason.as_deref(),
        Some("复检实测缸差在允收范围内"),
        "改判理由必须落专用真实列"
    );
    assert_eq!(
        row.rejudge_by,
        Some(SESSION_A),
        "改判操作人必须为会话用户且可回读"
    );
    assert!(row.rejudge_at.is_some(), "改判时间必须落真实列");
    assert_eq!(row.rejudge_count, 1, "改判次数必须累加落真实列");
    // 让步列保持历史让步事实（改判不得抹掉让步痕迹）
    assert_eq!(
        row.concession_reason.as_deref(),
        Some("让步接收用于改判链路"),
        "改判后最近一次让步理由仍可回读"
    );

    // 审计痕：既有范式 update_with_audit 写 audit_log 前后全行快照——
    // 改判行的 before 必须是被改判前的让步态、after 必须是 PASSED、user_id 为会话用户
    let logs = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceId.eq(receipt.id.to_string()))
        .all(&db)
        .await
        .expect("查询审计日志失败");
    let rejudge_log = logs
        .iter()
        .find(|log| {
            let before = log
                .before_snapshot
                .as_ref()
                .map(|v| {
                    v.0.get("inspection_status")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            let after = log
                .after_snapshot
                .as_ref()
                .map(|v| {
                    v.0.get("inspection_status")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            before == purchase_receipt_inspection::CONCESSION_ACCEPTED
                && after == purchase_receipt_inspection::PASSED
        })
        .expect("必须存在一条 audit_log：改判前 CONCESSION_ACCEPTED、改判后 PASSED");
    assert_eq!(
        rejudge_log.user_id,
        Some(SESSION_A),
        "改判审计痕的操作人必须可回读且等于会话用户"
    );

    // fail 出边：另一张让步单改判为 REJECTED（不合格→走退货出口）
    let other = seed_receipt(
        &db,
        purchase_receipt_inspection::CONCESSION_ACCEPTED,
        pr_status::DRAFT,
    )
    .await;
    service(&db)
        .rejudge_receipt(
            other.id,
            RejudgeReceiptRequest {
                inspection_result: Some(purchase_inspection_result::FAIL.to_string()),
                reason: Some("复检确认整批不合格，转退货".to_string()),
            },
            SESSION_A,
        )
        .await
        .expect("CONCESSION_ACCEPTED→REJECTED 改判必须成功");
    let row = reread(&db, other.id).await;
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::REJECTED,
        "fail→REJECTED 同源映射改判必须成立"
    );

    // 改判离开让步态后再次改判 ⇒ BUSINESS_ERROR（非法前驱闭环）
    let err = service(&db)
        .rejudge_receipt(
            other.id,
            RejudgeReceiptRequest {
                inspection_result: Some(purchase_inspection_result::PASS.to_string()),
                reason: Some("再次改判".to_string()),
            },
            SESSION_A,
        )
        .await
        .expect_err("非让步态再次改判必须被拒");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "实得: {}",
        err.error_code()
    );
}

// =========================================================
// 锁5：应用层词表 == DB CHECK 允许值集合（三重同源，不等判红）
// =========================================================

#[tokio::test]
async fn lock5_db_check_equals_app_vocab_exactly() {
    let db = seeded_db().await;

    // 1) 活库真值：从 pg_constraint 读取 CHECK 定义并解析允许值集合
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"SELECT pg_get_constraintdef(con.oid) AS def
                 FROM pg_constraint con
                 JOIN pg_class rel ON rel.oid = con.conrelid
                WHERE con.conname = 'chk_purchase_receipt_inspection_status'
                  AND rel.relname = 'purchase_receipt'"#,
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .expect("查询 CHECK 定义失败")
        .expect("迁移已施加 CHECK chk_purchase_receipt_inspection_status，活库必须可查");
    let def: Option<String> = row.try_get("", "def").expect("读取 def 列失败");
    let def = def.expect("CHECK 定义不得为 NULL");
    let db_check_set = parse_quoted_tokens(&def);
    assert!(
        !db_check_set.is_empty(),
        "CHECK 定义解析出空集——解析失守，拒绝继续比对（定义原文: {def}）"
    );

    // 2) 应用层权威词表
    let app_set: BTreeSet<String> = purchase_receipt_inspection::ALL
        .iter()
        .map(|t| t.to_string())
        .collect();
    assert_eq!(
        db_check_set, app_set,
        "DB CHECK 允许值集合与应用层词表 ALL 必须集合相等（逐元素，不是包含）：\nCHECK={db_check_set:?}\n词表={app_set:?}\n\
         任一侧单独增删值都是契约漂移（词表扩 CHECK 未跟⇒写入撞 23514；CHECK 宽⇒旁路可写业务永不产生的脏 token）"
    );

    // 3) 迁移原文 ALLOWED_SQL 锚点解析（防止「迁移改了但活库没跑到」与「两侧手写清单漂移」）
    let mig_src = std::fs::read_to_string(format!(
        "{}/migration/src/domain/business/m0084_concession_receiving_channel.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("读取让步接收通道迁移文件失败");
    let anchor = "const ALLOWED_SQL: &str = \"";
    let start = mig_src
        .find(anchor)
        .expect("迁移文件缺少 ALLOWED_SQL 锚点（CHECK 取值种子形态已改动）");
    let rest = &mig_src[start + anchor.len()..];
    let end = rest.find("\";").expect("ALLOWED_SQL 字面量未闭合");
    let migration_set = parse_quoted_tokens(&rest[..end]);
    assert_eq!(
        migration_set, app_set,
        "迁移 ALLOWED_SQL 与词表必须集合相等：\n迁移={migration_set:?}\n词表={app_set:?}"
    );

    // 4) 活体正向钉：让步 token 真写进库不撞 CHECK（词表==CHECK 不只是纸面相等）
    let receipt = seed_receipt(&db, purchase_receipt_inspection::PENDING, pr_status::DRAFT).await;
    service(&db)
        .concede_receipt(
            receipt.id,
            ConcedeReceiptRequest {
                reason: Some("活体写入验证让步 token 不违反 CHECK".to_string()),
            },
            SESSION_A,
        )
        .await
        .expect("CONCESSION_ACCEPTED 必须同时被词表与 DB CHECK 接受");
    let users = user::Entity::find_by_id(SESSION_A)
        .one(&db)
        .await
        .expect("查询会话用户失败");
    assert!(users.is_some(), "会话用户夹具必须存在");
}
