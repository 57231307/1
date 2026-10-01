//! 采购质检结论词表强校验 + 完成质检同事务回写入库单检验状态 契约锁（PR #942 wave5）
//!
//! 权威链（本锁的唯一判据，全部经实地取证坐实，路径基准仓库根）：
//! - `purchase_inspection.inspection_result` 列的真实写入方是前端「完成」三连 prompt
//!   的结论采集：`frontend/src/views/purchase-inspection/composables/usePiProc.ts`
//!   （inputPattern 由 `frontend/src/utils/purchase-inspection-result.ts` 的权威常量
//!   构造，原样提交 token）；真实落库 token 全集 = pass / fail / partial。
//! - 本域权威词表 `backend/src/models/status/purchase_inventory.rs::purchase_inspection_result`
//!   与前端常量逐字同源；后端完成校验与「结论→入库单检验状态」映射共用该表的
//!   `to_receipt_inspection_status`（Some ⟺ 词表内），杜绝"校验一套、映射另一套"漂移。
//! - 通用质检记录域 `quality_inspection_records.inspection_result` 的中文词表
//!   `quality_inspection_result`（待检/合格/不合格）服务的是**另一张表**
//!   （见 `backend/src/services/quality_inspection_service.rs::sync_receipt_inspection_status`）。
//!   跨域借用它校验本列会把 pass/fail/partial 这些合法生产数据判成非法——本锁反向钉死
//!   两个方向：中文 token 对本列非法；采购质检服务源码不得再引用通用域词表/映射。
//! - 生产行为佐证（已提交 e2e，不随本锁改动）：`frontend/e2e/purchase/04-inspection.spec.ts`
//!   UI 三连 prompt 送 pass/partial 并断成功；`frontend/e2e/fullflow/22-inspection-to-return.spec.ts`
//!   送 pass/fail 并回读落库原值；`frontend/e2e/purchase/11-return-from-inspection.spec.ts`
//!   送 fail。三处 token 与本域词表逐字符一致。
//!
//! 回写映射裁定（入库词表 purchase_receipt_inspection 只有大写三态）：
//! - pass → PASSED（「质检合格：允许后续入库/结算流转」）；
//! - fail → REJECTED（「质检不合格：走让步接收或退货流程」）；
//! - partial → REJECTED：部分合格≠整批合格，按 PASSED 放行即兜底开门；PENDING 语义为
//!   「待检验」与已完成检验不符；partial 的前端下游门控与 fail 完全同路径
//!   （「生成退货」仅 result∈{fail,partial} 显示，PurchaseInspectionTable.vue），
//!   落入 REJECTED 的「让步接收或退货」处置通道；精确结论 partial 无损保留在本列。
//!   依据原文见 `purchase_inspection_result::to_receipt_inspection_status` 文档注释。
//!
//! 其余根因锁（前批坐实、形态保留）：
//! - 完成链路的回写与质检落库同事务：回写失败或关联入库单缺失一律 `?` 上抛、整体回滚，
//!   禁止"质检显示已完成但入库单状态未回写"的静默半成功。
//! - 建单在任何写库动作（含取号）之前显式校验 receipt_id 指向的入库单存在，
//!   不存在 → not_found（含 ID 的真实原因按 utils/error.rs 口径走脱敏族），不让外键裸 500 兜底。
//!
//! 覆盖策略（无 mock、真实 service 调用）：
//! - sqlite::memory: 自建同构表（列与 models 实体逐列对应）：非法结论拒绝（回查质检行
//!   与 count 无痕）、合法 token 不被词表判非法（词表校验在任何事务/行锁之前，sqlite
//!   可跑到该判定位）、坏引用建单 404（先于取号，不触 PG 专有 advisory lock）；
//! - 完成成功链路含 `lock_exclusive()`（sqlite 方言不支持行锁，先例：
//!   contract_wave1_ap_payment_request_items_test.rs）→ 三 token 完整接受、回写与
//!   回滚行为用 `#[ignore]` 活库用例（TEST_DATABASE_URL→已迁移 PG，ci-test-rust-ignored
//!   执行；本地无变量时 require_postgres 断言**显式失败并说明**，禁止条件跳过假绿）。
//! - 防回潮源码扫描（include_str!）：白名单校验必须先于任何 Set/begin；回写必须在同一
//!   txn 且先于 commit；建单校验必须先于 generate_inspection_no 与 receipt_id 落库；
//!   采购质检服务不得出现通用域词表标识（quality_dyeing / from_inspection_result）。

mod test_common;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bingxi_backend::models::purchase_inspection;
use bingxi_backend::models::purchase_receipt;
use bingxi_backend::models::status::purchase_inventory::{
    purchase_inspection as pis_status, purchase_inspection_result,
    purchase_receipt as receipt_status, purchase_receipt_inspection,
};
use bingxi_backend::models::status::quality_dyeing::quality_inspection_result;
use bingxi_backend::services::purchase_inspection_service::{
    CompleteInspectionRequest, CreatePurchaseInspectionRequest, PurchaseInspectionService,
};
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, PaginatorTrait,
    Set, Statement,
};
use std::str::FromStr;
use std::sync::Arc;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

// =========================================================
// sqlite 同构表（列与 models/purchase_inspection.rs、models/purchase_receipt.rs 逐列对应）
// =========================================================

const PURCHASE_INSPECTION_DDL: &str = r#"CREATE TABLE purchase_inspection (
    id INTEGER PRIMARY KEY,
    inspection_no TEXT NOT NULL,
    receipt_id INTEGER,
    order_id INTEGER,
    supplier_id INTEGER NOT NULL,
    inspection_date TEXT NOT NULL,
    inspector_id INTEGER,
    inspection_type TEXT,
    sample_size TEXT,
    defect_count INTEGER,
    pass_quantity TEXT,
    reject_quantity TEXT,
    inspection_status TEXT,
    inspection_result TEXT,
    quality_score TEXT,
    defect_description TEXT,
    attachment_urls TEXT,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    completed_by INTEGER
)"#;

const PURCHASE_RECEIPT_DDL: &str = r#"CREATE TABLE purchase_receipt (
    id INTEGER PRIMARY KEY,
    receipt_no TEXT NOT NULL UNIQUE,
    order_id INTEGER,
    supplier_id INTEGER NOT NULL,
    receipt_date TEXT NOT NULL,
    warehouse_id INTEGER NOT NULL,
    department_id INTEGER,
    receiver_id INTEGER,
    inspector_id INTEGER,
    inspection_status TEXT NOT NULL,
    receipt_status TEXT NOT NULL,
    total_quantity TEXT NOT NULL,
    total_quantity_alt TEXT NOT NULL,
    total_amount TEXT NOT NULL,
    notes TEXT,
    attachment_urls TEXT,
    created_by INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_by INTEGER,
    updated_at TEXT NOT NULL,
    confirmed_at TEXT,
    confirmed_by INTEGER
)"#;

async fn sqlite_db() -> DatabaseConnection {
    sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败")
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

fn unique_suffix() -> i64 {
    Utc::now().timestamp_nanos_opt().unwrap()
}

async fn seed_pending_inspection(
    db: &DatabaseConnection,
    receipt_id: Option<i32>,
) -> purchase_inspection::Model {
    purchase_inspection::ActiveModel {
        inspection_no: Set(format!("PI-W5-{suffix}", suffix = unique_suffix())),
        receipt_id: Set(receipt_id),
        supplier_id: Set(1),
        inspection_date: Set(Utc
            .with_ymd_and_hms(2026, 9, 1, 0, 0, 0)
            .unwrap()
            .date_naive()),
        inspection_status: Set(Some(pis_status::PENDING.to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种待检质检单失败")
}

async fn seed_receipt(db: &DatabaseConnection) -> purchase_receipt::Model {
    purchase_receipt::ActiveModel {
        receipt_no: Set(format!("GR-W5-{suffix}", suffix = unique_suffix())),
        supplier_id: Set(1),
        receipt_date: Set(Utc
            .with_ymd_and_hms(2026, 9, 1, 0, 0, 0)
            .unwrap()
            .date_naive()),
        warehouse_id: Set(1),
        // 词表同源：入库单检验状态初始值取权威常量（大写 PENDING）
        inspection_status: Set(purchase_receipt_inspection::PENDING.to_string()),
        receipt_status: Set(receipt_status::DRAFT.to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        created_by: Set(1),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种入库单失败")
}

async fn inspection_count(db: &DatabaseConnection) -> u64 {
    purchase_inspection::Entity::find()
        .count(db)
        .await
        .expect("统计质检单行数失败")
}

// =========================================================
// ① 合法 token 必须被词表接受（正向锁，防"判错过严"再犯）——sqlite 段
//
// 白名单校验发生在任何事务/行锁之前（防回潮扫描另锁顺序），因此 sqlite 环境下
// 合法 token 的失败点只可能出现在后续方言受限处（lock_exclusive/审计表），
// 绝不允许是 400 VALIDATION_ERROR——出现即"把合法生产数据判成非法"的再犯实锤。
// 完整成功链路（落库+回写）由 ④ 活库用例承担。
// =========================================================

#[tokio::test]
async fn domain_tokens_are_never_rejected_by_vocabulary_check() {
    let db = sqlite_db().await;
    exec(&db, PURCHASE_INSPECTION_DDL).await;
    exec(&db, PURCHASE_RECEIPT_DDL).await;
    let svc = PurchaseInspectionService::new(Arc::new(db.clone()));

    for token in purchase_inspection_result::ALL {
        let seeded = seed_pending_inspection(&db, None).await;
        let res = svc
            .complete_inspection(
                seeded.id,
                CompleteInspectionRequest {
                    pass_quantity: dec("10.00"),
                    reject_quantity: Decimal::ZERO,
                    inspection_result: token.to_string(),
                },
                1,
            )
            .await;
        if let Err(err) = res {
            let body = err.to_response();
            assert_ne!(
                body.code, "VALIDATION_ERROR",
                "合法结论 {token:?} 被词表校验误判非法（判错过严再犯）: {body:?}",
            );
            assert!(
                !body.message.contains("不在取值域内"),
                "合法结论 {token:?} 不得命中取值域拒绝文案: {body:?}",
            );
        }
        // 清理本 token 的种子行，保持 count 断言环境干净
        purchase_inspection::Entity::delete_by_id(seeded.id)
            .exec(&db)
            .await
            .expect("清理种子行失败");
    }
}

// =========================================================
// ② 非法 inspection_result → 400 VALIDATION_ERROR + 文案含本域真实词表取值，且无痕
//
// 拒绝组刻意包含通用质检域中文表 token（待检/合格/不合格）：对**本列**它们是词表外，
// 跨域借用中文表把它们放行同样是缺陷。
// =========================================================

#[tokio::test]
async fn complete_with_outside_domain_result_rejected_and_leaves_no_trace() {
    let db = sqlite_db().await;
    exec(&db, PURCHASE_INSPECTION_DDL).await;
    exec(&db, PURCHASE_RECEIPT_DDL).await;
    let seeded = seed_pending_inspection(&db, None).await;
    let svc = PurchaseInspectionService::new(Arc::new(db.clone()));

    // 词表外形态全量：跨域中文 token / 大小写变体 / 英文近邻词 / 空串 / 带空白——一律拒绝。
    let illegal = [
        "合格",
        "待检",
        "不合格",
        "PASSED",
        "passed",
        "Passed",
        "qualified",
        "unknown",
        "合格 ",
        "",
    ];
    for bad in illegal {
        let err = svc
            .complete_inspection(
                seeded.id,
                CompleteInspectionRequest {
                    pass_quantity: dec("10.00"),
                    reject_quantity: Decimal::ZERO,
                    inspection_result: bad.to_string(),
                },
                1,
            )
            .await
            .expect_err(&format!("词表外结论 {bad:?} 必须被拒绝"));
        // 真实变体：可外显校验族（不是脱敏 ValidationError、更不是 internal/兜底）
        assert!(
            matches!(&err, bingxi_backend::utils::error::AppError::ValidationErrorDisplayable(m)
                if m.contains(&format!("质检结果只能是{}", purchase_inspection_result::ALL.join("/")))),
            "非法结论必须 ValidationErrorDisplayable 且文案含本域权威词表取值，实际: {err:?}"
        );
        let body = err.to_response();
        assert_eq!(body.code, "VALIDATION_ERROR");
        assert!(
            body.message.contains(purchase_inspection_result::PASS)
                && body.message.contains(purchase_inspection_result::FAIL)
                && body.message.contains(purchase_inspection_result::PARTIAL),
            "外显文案必须含本域词表全部取值（由权威表 join 生成），实际: {body:?}"
        );
        let resp: Response = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "非法结论必须 400");

        // 无痕：结论未落库、状态未推进
        let row = purchase_inspection::Entity::find_by_id(seeded.id)
            .one(&db)
            .await
            .unwrap()
            .expect("质检行应存在");
        assert_eq!(row.inspection_result, None, "被拒绝的结论不得落库");
        assert_eq!(
            row.inspection_status.as_deref(),
            Some(pis_status::PENDING),
            "被拒绝的完成不得推进质检单状态"
        );
    }
    assert_eq!(
        inspection_count(&db).await,
        1,
        "非法结论拒绝路径不得新增任何质检行"
    );
}

// =========================================================
// ③ 不存在的 receipt_id 建单 → 404 NOT_FOUND（非 500），且先于任何写库无脏行
// =========================================================

#[tokio::test]
async fn create_with_missing_receipt_rejected_404_before_any_write() {
    let db = sqlite_db().await;
    exec(&db, PURCHASE_INSPECTION_DDL).await;
    exec(&db, PURCHASE_RECEIPT_DDL).await;
    let svc = PurchaseInspectionService::new(Arc::new(db.clone()));

    // 999999 不存在：必须在应用层显式 404，而不是交给 DB 外键/取号路径出 500
    let err = svc
        .create_inspection(
            CreatePurchaseInspectionRequest {
                receipt_id: Some(999_999),
                order_id: None,
                supplier_id: Some(1),
                inspection_date: None,
                inspector_id: None,
                inspection_type: None,
                sample_size: None,
                notes: None,
            },
            1,
        )
        .await
        .expect_err("指向不存在入库单的建单必须被拒绝");
    assert!(
        matches!(&err, bingxi_backend::utils::error::AppError::NotFound(_)),
        "坏引用必须按真实责任定性为 not_found，实际: {err:?}"
    );
    let body = err.to_response();
    assert_eq!(
        body.code, "NOT_FOUND",
        "含 ID 的真实原因走脱敏 not_found 族"
    );
    let resp: Response = err.into_response();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "坏引用必须 404 而非 500"
    );
    assert_ne!(body.code, "DATABASE_ERROR", "绝不允许以裸 500 形态出参");

    assert_eq!(
        inspection_count(&db).await,
        0,
        "引用校验先于取号/插入：被拒绝的建单不得留下任何脏行"
    );
}

// =========================================================
// 活库夹具：TEST_DATABASE_URL→已迁移 PG；本地无变量时显式失败并说明，不条件跳过
// =========================================================

async fn require_postgres(db: &DatabaseConnection) {
    let url = std::env::var("TEST_DATABASE_URL");
    assert!(
        matches!(&url, Ok(u) if u.starts_with("postgres")),
        "本用例覆盖 lock_exclusive 完成链路（sqlite 方言不支持行锁），\
         必须跑在 TEST_DATABASE_URL 指向的已迁移 PostgreSQL 上（ci-test-rust-ignored 注入）；\
         本地直接跑需显式 export TEST_DATABASE_URL=postgres://...，禁止 sqlite 回退假绿。\
         当前 TEST_DATABASE_URL={url:?}"
    );
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "夹具解析出的后端不是 PostgreSQL，活库锁不可信（setup_test_db 无变量时静默回退 sqlite）"
    );
}

// =========================================================
// ④ 合法结论（pass/fail/partial）完整接受 + 同事务回写入库单检验状态（逐字符=权威常量）
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（complete_inspection 走 lock_exclusive + update_with_audit，sqlite 方言不支持行锁）"]
async fn live_complete_accepts_domain_tokens_and_writes_back_receipt_status() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    let svc = PurchaseInspectionService::new(Arc::new(db.clone()));

    // (结论本域 token, 期望入库单检验状态) —— 两侧均取权威常量，逐字符同源。
    // partial→REJECTED 的裁定依据（入库词表仅三态、partial 与 fail 同走退货/让步通道、
    // PASSED 放行属兜底开门）见 purchase_inspection_result::to_receipt_inspection_status 注释。
    let cases = [
        (
            purchase_inspection_result::PASS,
            purchase_receipt_inspection::PASSED,
        ),
        (
            purchase_inspection_result::FAIL,
            purchase_receipt_inspection::REJECTED,
        ),
        (
            purchase_inspection_result::PARTIAL,
            purchase_receipt_inspection::REJECTED,
        ),
    ];
    for (result_token, expected_status) in cases {
        let receipt = seed_receipt(&db).await;
        let inspection = seed_pending_inspection(&db, Some(receipt.id)).await;

        let done = svc
            .complete_inspection(
                inspection.id,
                CompleteInspectionRequest {
                    pass_quantity: dec("80.00"),
                    reject_quantity: dec("20.00"),
                    inspection_result: result_token.to_string(),
                },
                1,
            )
            .await
            .unwrap_or_else(|e| panic!("合法结论 {result_token:?} 必须被完整接受，实际: {e:?}"));
        assert_eq!(
            done.inspection_status.as_deref(),
            Some(pis_status::COMPLETED)
        );
        assert_eq!(done.inspection_result.as_deref(), Some(result_token));

        // 核心断言：回查收货单检验状态确为映射词表对应值（不是只断质检表）
        let reloaded = purchase_receipt::Entity::find_by_id(receipt.id)
            .one(&db)
            .await
            .unwrap()
            .expect("入库单行应存在");
        assert_eq!(
            reloaded.inspection_status, expected_status,
            "结论「{result_token}」必须回写为入库单检验状态 {expected_status:?}"
        );

        // 清理本用例播种行（质检表对入库单无外键约束，先子后父）
        purchase_inspection::Entity::delete_by_id(inspection.id)
            .exec(&db)
            .await
            .expect("清理播种质检行失败");
        purchase_receipt::Entity::delete_by_id(receipt.id)
            .exec(&db)
            .await
            .expect("清理播种入库单行失败");
    }
}

// =========================================================
// ⑤ 回写失败=整单失败：悬空 receipt_id 完成必须整体回滚（禁静默半成功）
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（complete_inspection 走 lock_exclusive + 事务回滚验证）"]
async fn live_complete_rolls_back_when_receipt_writeback_fails() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    let svc = PurchaseInspectionService::new(Arc::new(db.clone()));

    // purchase_inspection 表无 receipt_id 外键约束（m0009 DDL），坏引用可直接播种，
    // 用于实证"回写目标缺失"路径：完成必须显式失败，且质检单更新一并回滚。
    let dangling = 999_998_i32;
    let inspection = seed_pending_inspection(&db, Some(dangling)).await;

    let err = svc
        .complete_inspection(
            inspection.id,
            CompleteInspectionRequest {
                pass_quantity: dec("10.00"),
                reject_quantity: Decimal::ZERO,
                inspection_result: purchase_inspection_result::PASS.to_string(),
            },
            1,
        )
        .await
        .expect_err("回写目标入库单缺失时必须整单失败，禁止只记日志仍显示质检成功");
    assert!(
        matches!(&err, bingxi_backend::utils::error::AppError::NotFound(_)),
        "回写失败按真实责任定性 not_found（含 ID 走脱敏族），实际: {err:?}"
    );
    let body = err.to_response();
    assert_eq!(body.code, "NOT_FOUND");
    drop(body);
    let resp: Response = err.into_response();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // 同事务回滚实锤：质检单不得停留在"已完成"的半成功形态
    let row = purchase_inspection::Entity::find_by_id(inspection.id)
        .one(&db)
        .await
        .unwrap()
        .expect("质检行应存在");
    assert_eq!(
        row.inspection_status.as_deref(),
        Some(pis_status::PENDING),
        "回写失败必须连带回滚质检单状态推进"
    );
    assert_eq!(row.inspection_result, None, "回写失败必须连带回滚结论落库");

    // 清理播种行（质检表无外键，直接按主键删）
    purchase_inspection::Entity::delete_by_id(inspection.id)
        .exec(&db)
        .await
        .expect("清理播种质检行失败");
}

// =========================================================
// 词表逐字符同源锁（防第二套手写常量/中英互抄回潮）
// =========================================================

#[test]
fn purchase_inspection_result_vocabularies_are_character_identical() {
    // 本域权威表：写入方 prompt 的真实 token 全集，英文小写码，禁止中文化
    assert_eq!(
        purchase_inspection_result::ALL,
        &["pass", "fail", "partial"],
        "采购质检结论词表（写入方 usePiProc prompt 同源常量）token 禁止中文化/改大小写"
    );
    // 白名单判定与 ALL 同源：表内全真、表外（跨域中文/大小写变体/空串/空白）全假
    for token in purchase_inspection_result::ALL {
        assert!(
            purchase_inspection_result::is_valid(token),
            "{token:?} 必须合法"
        );
    }
    for outside in [
        "合格",
        "待检",
        "不合格",
        "PASSED",
        "Passed",
        "pass ",
        "passed",
        "qualified",
        "",
    ] {
        assert!(
            !purchase_inspection_result::is_valid(outside),
            "词表外值 {outside:?} 必须非法（禁止归一/兜底放行）"
        );
    }
    // 映射与白名单同一取值域：Some ⟺ is_valid（杜绝校验一套、映射另一套）
    assert_eq!(
        purchase_inspection_result::to_receipt_inspection_status(purchase_inspection_result::PASS),
        Some(purchase_receipt_inspection::PASSED)
    );
    assert_eq!(
        purchase_inspection_result::to_receipt_inspection_status(purchase_inspection_result::FAIL),
        Some(purchase_receipt_inspection::REJECTED)
    );
    assert_eq!(
        purchase_inspection_result::to_receipt_inspection_status(
            purchase_inspection_result::PARTIAL
        ),
        Some(purchase_receipt_inspection::REJECTED),
        "partial→REJECTED 裁定（依据见函数文档）；若该裁定被推翻，必须同步修订本锁与裁定注释"
    );
    for outside in ["合格", "PASSED", "unknown", ""] {
        assert_eq!(
            purchase_inspection_result::to_receipt_inspection_status(outside),
            None,
            "词表外 {outside:?} 必须 None，由调用方报错而不是默认成某个状态"
        );
    }

    // 入库单检验状态词表（另一列，大写三态）不变
    assert_eq!(
        purchase_receipt_inspection::ALL,
        &["PENDING", "PASSED", "REJECTED"],
        "入库单检验状态词表（写入方 models/status/purchase_inventory.rs）大写 token"
    );

    // 跨域分离锁：通用质检记录域中文词表原样保留（禁止被本域"顺手英文化"），
    // 且两表取值零交集——任何一侧借用另一侧词表都是跨域误用。
    assert_eq!(
        quality_inspection_result::ALL,
        &["待检", "合格", "不合格"],
        "通用质检记录结论词表（quality_inspection_records）中文 token 禁止英文化"
    );
    for token in purchase_inspection_result::ALL {
        assert!(
            !quality_inspection_result::ALL.contains(token),
            "本域 token {token:?} 不得出现在通用域中文表（两表必须互斥）"
        );
    }
    // 通用域映射保持中文表语义，未被本域英文 token 污染
    assert_eq!(
        purchase_receipt_inspection::from_inspection_result(quality_inspection_result::QUALIFIED),
        Some(purchase_receipt_inspection::PASSED)
    );
    assert_eq!(
        purchase_receipt_inspection::from_inspection_result(purchase_inspection_result::PASS),
        None,
        "通用域映射必须不认本域英文 token（反向跨域借用同样是缺陷）"
    );
}

// =========================================================
// ⑥ 防回潮源码扫描（include_str!）
// =========================================================

/// 从源码截取一个 impl 内方法块：anchor 起，到首个以 4 空格缩进的 `}` 行
/// （方法体内部行均 ≥8 空格缩进，首个 `\n    }` 即方法结束）。
/// 先剔除 `\r`：Windows 工作树 CRLF 会使跨行 contains 断言漏检（先例 wave5 测试）。
fn extract_method(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n    }")
        .unwrap_or_else(|| panic!("方法块结束定位失败: {anchor}"));
    src[i..i + j + 6].to_string()
}

#[test]
fn source_scan_complete_validates_against_domain_table_and_never_borrows_cross_domain() {
    let src = include_str!("../src/services/purchase_inspection_service.rs").replace('\r', "");

    // 跨域借用回潮锁：采购质检服务不得再出现通用质检域词表标识——
    // 用中文表（待检/合格/不合格）校验本列会把合法生产数据（pass/fail/partial）判成非法。
    assert!(
        !src.contains("quality_dyeing"),
        "回潮：purchase_inspection_service.rs 重新引用了通用质检记录域模块 quality_dyeing"
    );
    assert!(
        !src.contains("quality_inspection_result::ALL"),
        "回潮：白名单取值域重新借用通用域中文表 quality_inspection_result::ALL"
    );
    assert!(
        !src.contains("from_inspection_result"),
        "回潮：完成链路重新使用通用域映射 from_inspection_result（本域映射是 to_receipt_inspection_status）"
    );

    let block = extract_method(&src, "pub async fn complete_inspection");

    // 白名单校验：与收货状态映射共用本域权威表 to_receipt_inspection_status
    //（同一取值域、单点校验），非法值走可外显 validation_displayable；
    // 禁止"结果字段直接 Set 而无白名单校验"回潮
    assert!(
        block.contains(
            "purchase_inspection_result::to_receipt_inspection_status(&req.inspection_result)"
        ),
        "完成质检必须经本域权威映射做白名单校验:\n{block}"
    );
    assert!(
        block.contains("AppError::validation_displayable"),
        "非法结论必须外显公开取值域（validation_displayable）"
    );
    assert!(
        block.contains("purchase_inspection_result::ALL.join"),
        "外显文案取值必须由本域权威表 ALL join 生成，禁止手写第二套词表"
    );
    let validate_pos = block
        .find("purchase_inspection_result::to_receipt_inspection_status(&req.inspection_result)")
        .unwrap();
    let begin_pos = block.find(".begin()").expect("完成链路必须走事务");
    let result_set_pos = block
        .find("inspection_result = Set(")
        .expect("结论必须落库");
    assert!(
        validate_pos < begin_pos && validate_pos < result_set_pos,
        "白名单校验必须先于事务开启与任何落库 Set（词表外值零副作用）"
    );

    // 同事务回写：find_by_id 在 txn 上、Set 收货状态、且先于 commit；失败一律 `?` 上抛
    assert!(
        block.contains("purchase_receipt::Entity::find_by_id")
            && block.contains("receipt_active.inspection_status = Set(receipt_inspection_status"),
        "必须把映射后的检验状态回写 purchase_receipt:\n{block}"
    );
    let writeback_pos = block
        .find("receipt_active.inspection_status = Set(receipt_inspection_status")
        .unwrap();
    let commit_pos = block.find("txn.commit()").expect("事务必须显式提交");
    assert!(
        writeback_pos < commit_pos,
        "回写必须在同一事务提交之前（失败即整体回滚）"
    );
    // 禁止把回写失败吞成日志/兜底：不得出现 `let _ =`、`.ok();` 形态
    assert!(
        !block.contains("let _ =") && !block.contains(".ok();"),
        "回写失败禁止吞错兜底（let _ / .ok() 形态）:\n{block}"
    );
    // receipt_id 为空的合法情形必须显式记日志（非静默）
    assert!(
        block.contains("tracing::info!"),
        "无关联入库单时必须显式留痕，不许静默跳过回写"
    );
}

#[test]
fn source_scan_create_checks_receipt_before_any_db_action() {
    let src = include_str!("../src/services/purchase_inspection_service.rs").replace('\r', "");
    let block = extract_method(&src, "pub async fn create_inspection");
    assert!(
        block.contains("purchase_receipt::Entity::find_by_id(receipt_id)"),
        "建单必须在应用层显式校验 receipt_id 指向的入库单存在:\n{block}"
    );
    assert!(
        block.contains("AppError::not_found"),
        "引用缺失必须 not_found（含 ID 走脱敏族），不得裸 500/兜底"
    );
    let check_pos = block
        .find("purchase_receipt::Entity::find_by_id(receipt_id)")
        .unwrap();
    let gen_pos = block
        .find("generate_inspection_no()")
        .expect("建单必须走统一取号");
    let set_pos = block
        .find("receipt_id: Set(req.receipt_id)")
        .expect("receipt_id 落库点必须存在");
    assert!(
        check_pos < gen_pos && check_pos < set_pos,
        "引用校验必须先于取号与任何写库动作（拒绝路径零脏行）"
    );
}
