//! 采购质检「复检改判覆写」契约锁（PR #942 wave6）
//!
//! 钉死的现状事实（与 `models/status/purchase_inventory.rs::purchase_receipt_inspection`
//! REJECTED 注释同源；注释撒谎在本仓是明令禁止的，本锁是其代码级实证）：
//! `purchase_receipt.inspection_status` 的「复检改判」能力**事实上存在，但不是独立通道**——
//! 它只是 `complete_inspection` 回写段（services/purchase_inspection_service.rs:253-273）
//! 对目标列的**无条件 Set 覆写(:265)**：对同一收货单再建一张质检单（建单仅校收货单存在性
//! :91-96，无"同 receipt 唯一/已有质检结论则拒"的前置；表约束层 receipt_id 也仅普通索引
//! 无 UNIQUE，migration m0009:145-172）并 complete(pass)，即可把已 REJECTED 的列翻成
//! PASSED。**无专门端点、无审批、无历史留痕**（本列只存最终 token，历次结论只存在于
//! purchase_inspection 各行自身的 inspection_result）。
//! 「让步接收（不合格特采/降级接收）」在本列**没有 token**——词表恒为
//! PENDING/PASSED/REJECTED 三态（本文件 S4 锁防第四态悄悄加入）。
//!
//! 门控侧（判定唯一实现 PurchaseReceiptService::ensure_receipt_inspection_allows_flow，
//! services/purchase_receipt_service.rs:83-107）：只放行 PASSED；接线点为确认入库
//! （purchase_receipt_ops/state.rs:48）与应付 auto-generate
//! （ap_invoice_ops/receipt.rs:113）。改判覆写成 PASSED 后两入口随之放行——这是现状
//! 契约（风险：任何人对 REJECTED 收货单再建一单质检并 pass 即可完成改判，绕过审批）。
//!
//! 覆盖策略（无 mock、真实 service 调用，禁止硬编码 JSON 假装断言；
//! 路线一 #4669 判责：全部用例经 `test_common::setup_test_db()` 跑已迁移
//! PostgreSQL，表结构唯一来源 = backend/migration，不再自建同构 DDL；
//! FK 父行 users/warehouses/products 按裁定 R1 自种子，suppliers id=1 为
//! 迁移种子参照表恒在）：
//!   T1 结算入口对 REJECTED 列值的真实拒绝（BUSINESS_ERROR+逐字符文案+信封四键）与
//!     被拒零副作用回读；再把列翻成 PASSED（**列写入本身由 C1 活链路与 S3 无条件覆写锁
//!     负责，此处只钉"门控判定完全由列终值驱动"**）——真库 ap_invoice 表存在，
//!     放行证明从 sqlite 时代"缺表 DATABASE_ERROR 旁证"升级为**应付单真实生成**
//!     （更强断言；sqlite 可验性假设已随路线一废弃）；
//!   T2 建单入口对"已有 COMPLETED 质检单的同收货单"不拒（真实调用 create_inspection；
//!     真库下取号 pg_advisory_xact_lock 是正常路径，Ok 分支即当前契约主证据；
//!     绝不允许出现 BUSINESS/VALIDATION 形态的"重复质检"拒绝，出现即改判通道被关闭
//!     的实锤，需同步修订本锁与词表注释）；
//!   S1-S4 纯源码/词表防回潮锁（任何环境可跑）。
//!   C1 完整业务链路（不再 #[ignore]：路线一后所有用例同跑已迁移 PG——complete/create/
//!   confirm 走 lock_exclusive 与 pg_advisory_xact_lock 取号在 PG 上是原生路径，
//!   缺 TEST_DATABASE_URL 由夹具显式 panic，禁止条件跳过假绿）——
//!   真实 create_inspection + complete_inspection(fail) 回写 REJECTED
//!   → confirm_receipt 被拒（400/BUSINESS_ERROR/逐字符文案/零漂移：列、状态、确认时间、
//!   库存、应付全不变）→ 旧单重复 complete 被状态门拒（BUSINESS_ERROR，改判只能走新单）
//!   → 真实 create_inspection 第二单成功（无唯一性前置的服务级实证）→ complete(pass)
//!   把列从 REJECTED **覆写**为 PASSED（改判核心断言）→ 此后 auto_generate_from_receipt
//!   真实生成应付成功、confirm_receipt 真实推进 COMPLETED+库存回读；旧质检行结论无损
//!   （留痕在质检行、不在收货列）。

mod test_common;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bingxi_backend::models::status::common::STATUS_DRAFT;
use bingxi_backend::models::status::purchase_inventory::{
    purchase_inspection as pis_status, purchase_inspection_result,
    purchase_receipt as receipt_status, purchase_receipt_inspection,
};
use bingxi_backend::models::{
    ap_invoice, inventory_stock, inventory_transaction, purchase_inspection, purchase_receipt,
    purchase_receipt_item,
};
use bingxi_backend::services::ap_invoice_service::ApInvoiceService;
use bingxi_backend::services::purchase_inspection_service::{
    CompleteInspectionRequest, CreatePurchaseInspectionRequest, PurchaseInspectionService,
};
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, Set, Statement,
};
use std::str::FromStr;
use std::sync::Arc;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn unique_suffix() -> i64 {
    Utc::now().timestamp_nanos_opt().unwrap()
}

// =========================================================
// 真库夹具：表结构唯一来源 = backend/migration。本域 FK 父行自种子（裁定 R1）：
// purchase_receipt.supplier_id → suppliers（迁移种子参照表 id=1 恒在）、
// warehouse_id → warehouses（自建 id=1）；purchase_receipt_item.product_id →
// products（自建 id=1）；操作人/审计引用 users(id)（自建 id=1）。
// =========================================================

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

async fn seeded_db() -> DatabaseConnection {
    let db = test_common::setup_test_db().await;
    exec(
        &db,
        r#"INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
             (1,'w6_rejudge_op','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#,
    )
    .await;
    exec(
        &db,
        "INSERT INTO warehouses (id, name, warehouse_code, is_active) VALUES (1, '波6改判主仓', 'W6RJ-W1', true)",
    )
    .await;
    exec(
        &db,
        "INSERT INTO products (id, code, name) VALUES (1, 'W6RJ-P1', '复检改判契约测试坯布面料')",
    )
    .await;
    db
}

async fn seed_receipt(
    db: &DatabaseConnection,
    inspection_status: &str,
    total_amount: Decimal,
) -> purchase_receipt::Model {
    purchase_receipt::ActiveModel {
        receipt_no: Set(format!("GR-W6RJ-{suffix}", suffix = unique_suffix())),
        supplier_id: Set(1),
        receipt_date: Set(date(2026, 9, 1)),
        warehouse_id: Set(1),
        // 词表同源：检验状态取权威常量，不手写第二套 token
        inspection_status: Set(inspection_status.to_string()),
        receipt_status: Set(receipt_status::DRAFT.to_string()),
        total_quantity: Set(dec("10.0000")),
        total_quantity_alt: Set(Decimal::ZERO),
        total_amount: Set(total_amount),
        created_by: Set(1),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种入库单失败")
}

/// 播种一张已完成、结论 fail 的质检单（fixture 形态；真实"complete(fail) 回写 REJECTED"
/// 链路只在 C1 活库用例承载，这里不为成功路径招魂）
async fn seed_completed_fail_inspection(
    db: &DatabaseConnection,
    receipt_id: i32,
) -> purchase_inspection::Model {
    purchase_inspection::ActiveModel {
        inspection_no: Set(format!("PI-W6RJ-{suffix}", suffix = unique_suffix())),
        receipt_id: Set(Some(receipt_id)),
        supplier_id: Set(1),
        inspection_date: Set(date(2026, 9, 1)),
        inspection_status: Set(Some(pis_status::COMPLETED.to_string())),
        inspection_result: Set(Some(purchase_inspection_result::FAIL.to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种已完成质检单失败")
}

// =========================================================
// T1（真库）：结算入口门控完全由列终值驱动——REJECTED 真拒+零副作用；
// 列被翻成 PASSED 后同一入口立即放行并真实生成应付（改判覆写的下游效果）
// =========================================================

#[tokio::test]
async fn ap_entry_rejects_rejected_column_then_passes_after_column_flipped_to_passed() {
    // 真库 ap_invoice 表存在：放行证明＝应付单真实生成（sqlite 时代"缺表旁证"
    // 随路线一废弃，见文件头覆盖策略）
    let db = seeded_db().await;
    let svc = ApInvoiceService::new(Arc::new(db.clone()));

    let receipt = seed_receipt(&db, purchase_receipt_inspection::REJECTED, dec("100.00")).await;
    let before = purchase_receipt::Entity::find_by_id(receipt.id)
        .one(&db)
        .await
        .unwrap()
        .expect("收货单应存在");

    // ① REJECTED → 400 + BUSINESS_ERROR + 族口径逐字符文案（动作=生成应付结算）
    let err = svc
        .auto_generate_from_receipt(receipt.id, 1)
        .await
        .expect_err("REJECTED 收货单必须被结算门控拒绝");
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(_)),
        "REJECTED 拒绝必须外显族，实际: {err:?}"
    );
    let body = err.to_response();
    assert_eq!(body.code, "BUSINESS_ERROR");
    assert_eq!(
        body.message, "质检不合格的收货单不能生成应付结算，请先处理不合格品",
        "结算入口拒绝文案必须与裁定句式逐字符一致"
    );
    // 失败信封四键齐全（族口径 code/message/trace_id/timestamp）
    assert!(!body.trace_id.is_empty(), "失败信封必须含 trace_id");
    assert!(body.timestamp > 0, "失败信封必须含 timestamp");
    let resp: Response = err.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // ② 被拒零副作用：收货单全部业务列原样（门控只读判定，不得改列）
    let after = purchase_receipt::Entity::find_by_id(receipt.id)
        .one(&db)
        .await
        .unwrap()
        .expect("收货单必须原样存在");
    assert_eq!(
        after.inspection_status, before.inspection_status,
        "被拒路径不得改动检验状态列（REJECTED 仍是事实）"
    );
    assert_eq!(after.receipt_status, receipt_status::DRAFT.to_string());
    assert_eq!(after.confirmed_at, None, "被拒路径不得写确认时间");
    assert_eq!(after.total_amount, before.total_amount);

    // ③ 列被翻成 PASSED（改判覆写的**列写入动作**本身由 C1 活链路 + S3 无条件覆写
    //    源码锁钉死；此处直接更新列是夹具手段，钉的是"门控判定唯一由列终值驱动，
    //    收货单没有其他缓存/联表推导资格"——PASSED 即刻放行）
    let mut flip: purchase_receipt::ActiveModel = after.into();
    flip.inspection_status = Set(purchase_receipt_inspection::PASSED.to_string());
    flip.update(&db).await.expect("夹具翻转列失败");

    let invoice = svc
        .auto_generate_from_receipt(receipt.id, 1)
        .await
        .expect("列已 PASSED 的收货单门控必须放行并真实生成应付（真库全表，无缺表旁证）");
    assert_eq!(invoice.source_type.as_deref(), Some("PURCHASE_RECEIPT"));
    assert_eq!(invoice.source_id, Some(receipt.id));
    assert_eq!(invoice.amount, dec("100.00"), "应付金额取收货单总额");

    let final_row = purchase_receipt::Entity::find_by_id(receipt.id)
        .one(&db)
        .await
        .unwrap()
        .expect("收货单应存在");
    assert_eq!(
        final_row.inspection_status,
        purchase_receipt_inspection::PASSED.to_string(),
        "门控放行路径不得再翻动本列"
    );
    assert_eq!(final_row.confirmed_at, None);
}

// =========================================================
// T2（真库）：改判通道的入口实证——对"已有 COMPLETED 质检单"的同一收货单再建质检单，
// 建单入口不以任何业务理由拒绝（真库下取号是正常路径，Ok 分支即主证据）
// =========================================================

#[tokio::test]
async fn create_second_inspection_for_same_receipt_is_not_business_rejected() {
    let db = seeded_db().await;
    let svc = PurchaseInspectionService::new(Arc::new(db.clone()));

    let receipt = seed_receipt(&db, purchase_receipt_inspection::REJECTED, dec("100.00")).await;
    let old = seed_completed_fail_inspection(&db, receipt.id).await;

    let req = CreatePurchaseInspectionRequest {
        receipt_id: Some(receipt.id),
        order_id: None,
        supplier_id: Some(1),
        inspection_date: None,
        inspector_id: None,
        inspection_type: Some("复检".to_string()),
        sample_size: None,
        notes: None,
    };
    match svc.create_inspection(req, 1).await {
        // 真库下的正常路径：第二单真实建成、恒 pending、挂同一收货单——
        // "无同 receipt 唯一/已有质检则拒"的前置（改判通道入口敞开的主证据）
        Ok(created) => {
            assert_eq!(created.receipt_id, Some(receipt.id));
            assert_ne!(created.inspection_no, old.inspection_no, "两单号必须不同");
            assert_eq!(
                created.inspection_status.as_deref(),
                Some(pis_status::PENDING),
                "新单恒 pending（service :118）"
            );
        }
        Err(err) => {
            let body = err.to_response();
            assert_ne!(
                body.code, "BUSINESS_ERROR",
                "同收货单再建质检单不得被业务理由拒绝（改判通道被悄悄关闭需同步修订本锁与词表注释），实际: {body:?}"
            );
            assert_ne!(
                body.code, "VALIDATION_ERROR",
                "同收货单再建质检单不得被取值域拒绝，实际: {body:?}"
            );
            assert_ne!(
                body.code, "NOT_FOUND",
                "receipt 存在性前置已满足，不得再以引用缺失拒绝，实际: {body:?}"
            );
            // 真库下若仍失败，只可能是取号路径触真数据库故障（DATABASE_ERROR 族）——
            // 业务/校验/引用缺失三类拒绝任何一种出现都意味着改判通道被关闭
            assert_eq!(
                body.code, "DATABASE_ERROR",
                "真库下建单失败只允许是真数据库故障（改判通道入口必须敞开），实际: {body:?}"
            );
        }
    }

    // 建单尝试（无论成败）零副作用：收货列不被触碰、旧 COMPLETED 质检单不被改写
    let row = purchase_receipt::Entity::find_by_id(receipt.id)
        .one(&db)
        .await
        .unwrap()
        .expect("收货单应存在");
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::REJECTED.to_string(),
        "建单路径绝不回写收货列（回写只发生在 complete）"
    );
    let old_row = purchase_inspection::Entity::find_by_id(old.id)
        .one(&db)
        .await
        .unwrap()
        .expect("旧质检单应存在");
    assert_eq!(
        old_row.inspection_status.as_deref(),
        Some(pis_status::COMPLETED),
        "新单建立不得改写旧单状态（旧单仍是不可改判的 COMPLETED 事实）"
    );
    assert_eq!(
        old_row.inspection_result.as_deref(),
        Some(purchase_inspection_result::FAIL),
        "旧结论 token 原样保留（留痕在质检行自身，不在收货列）"
    );
}

// =========================================================
// PostgreSQL 夹具守卫：路线一下所有用例都跑已迁移 PG（setup_test_db 缺
// TEST_DATABASE_URL 即显式 panic）；本守卫再钉一遍后端形状，防夹具回退。
// =========================================================

async fn require_postgres(db: &DatabaseConnection) {
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "夹具解析出的后端不是 PostgreSQL，改判全链路锁不可信（路线一要求已迁移 PG）"
    );
}

// =========================================================
// C1（真库）：改判覆写全链路契约（本文件的根因主证据）
// =========================================================

#[tokio::test]
async fn live_rejudge_override_full_chain() {
    let db = seeded_db().await;
    require_postgres(&db).await;
    let pi_svc = PurchaseInspectionService::new(Arc::new(db.clone()));
    let rc_svc = PurchaseReceiptService::new(Arc::new(db.clone()));
    let ap_svc = ApInvoiceService::new(Arc::new(db.clone()));

    let receipt = seed_receipt(&db, purchase_receipt_inspection::PENDING, dec("100.00")).await;
    let receipt_no = receipt.receipt_no.clone();
    let batch = format!("W6RJLB{}", unique_suffix());
    purchase_receipt_item::ActiveModel {
        receipt_id: Set(receipt.id),
        order_item_id: Set(None),
        line_no: Set(1),
        product_id: Set(1),
        material_code: Set("FAB-W6RJ".to_string()),
        material_name: Set("复检改判契约测试坯布".to_string()),
        batch_no: Set(Some(batch.clone())),
        quantity: Set(dec("10.0000")),
        quantity_alt: Set(Some(dec("5.0000"))),
        unit_master: Set("米".to_string()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("播种入库明细失败");

    // ① 首单真实建单+complete(fail)：同事务回写把列置 REJECTED（映射 pass/fail 由 wave5 钉）
    let ins1 = pi_svc
        .create_inspection(
            CreatePurchaseInspectionRequest {
                receipt_id: Some(receipt.id),
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
        .expect("首张质检单建单必须成功");
    let done1 = pi_svc
        .complete_inspection(
            ins1.id,
            CompleteInspectionRequest {
                pass_quantity: Decimal::ZERO,
                reject_quantity: dec("10.0000"),
                inspection_result: purchase_inspection_result::FAIL.to_string(),
            },
            1,
        )
        .await
        .expect("complete(fail) 必须成功");
    assert_eq!(
        done1.inspection_status.as_deref(),
        Some(pis_status::COMPLETED)
    );
    let row = purchase_receipt::Entity::find_by_id(receipt.id)
        .one(&db)
        .await
        .unwrap()
        .expect("收货单应存在");
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::REJECTED.to_string(),
        "fail 结论必须经真实回写把列置 REJECTED"
    );

    // ② REJECTED 下确认入库被拒：400/BUSINESS_ERROR/逐字符文案，且零漂移（列不被触碰）
    let before_confirm = row.updated_at;
    let err = rc_svc
        .confirm_receipt(receipt.id, 1)
        .await
        .expect_err("质检不合格的收货单绝对不能确认入库");
    let body = err.to_response();
    assert_eq!(body.code, "BUSINESS_ERROR");
    assert_eq!(
        body.message, "质检不合格的收货单不能确认入库，请先处理不合格品",
        "确认入库入口 REJECTED 文案必须与裁定原文逐字符一致"
    );
    assert!(
        !body.trace_id.is_empty() && body.timestamp > 0,
        "信封四键齐全"
    );
    let resp: Response = err.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let row = purchase_receipt::Entity::find_by_id(receipt.id)
        .one(&db)
        .await
        .unwrap()
        .expect("收货单必须原样存在");
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::REJECTED.to_string(),
        "被拒的确认入库不得改动检验状态列（被拒零副作用）"
    );
    assert_eq!(row.receipt_status, receipt_status::DRAFT.to_string());
    assert_eq!(row.confirmed_at, None);
    assert_eq!(row.updated_at, before_confirm, "被拒路径不得触碰收货行");
    assert_eq!(
        inventory_stock::Entity::find()
            .filter(inventory_stock::Column::BatchNo.eq(&batch))
            .count(&db)
            .await
            .unwrap(),
        0,
        "被拒确认不得新增库存行"
    );
    assert_eq!(
        ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SourceType.eq("PURCHASE_RECEIPT"))
            .filter(ap_invoice::Column::SourceId.eq(receipt.id))
            .count(&db)
            .await
            .unwrap(),
        0,
        "被拒确认不得生成应付"
    );

    // ③ 边界：旧单（已 COMPLETED）再 complete 试图改判 → 状态门拒（门只看质检单自身
    //    service :215，不看收货列）；收货列保持 REJECTED，一步都不被触碰
    let err = pi_svc
        .complete_inspection(
            ins1.id,
            CompleteInspectionRequest {
                pass_quantity: dec("10.0000"),
                reject_quantity: Decimal::ZERO,
                inspection_result: purchase_inspection_result::PASS.to_string(),
            },
            1,
        )
        .await
        .expect_err("已完成的老质检单绝不能再次 complete");
    // 该门用 AppError::business（脱敏族，service :216）：真实文案只进变体内部与日志，
    // 出参 message 是固定公开文案——所以内容断言 match 变体，出参只断族 code（口径同源）
    assert!(
        matches!(&err, AppError::BusinessError(m) if m.contains("质检单状态不允许完成")),
        "拒绝原因必须是质检单自身状态门（不看收货列），实际: {err:?}"
    );
    let body = err.to_response();
    assert_eq!(
        body.code, "BUSINESS_ERROR",
        "状态门拒绝族口径，实际: {body:?}"
    );
    let row = purchase_receipt::Entity::find_by_id(receipt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::REJECTED.to_string(),
        "旧单重复完成被拒不得改动收货列（被拒零副作用）"
    );

    // ④ 改判通道实证：对同一收货单真实再建第二张质检单——**成功**（无同 receipt
    //    唯一性/已有结论则拒的前置；表约束 receipt_id 也仅普通索引，S1/S2 源码锁钉死）
    let ins2 = pi_svc
        .create_inspection(
            CreatePurchaseInspectionRequest {
                receipt_id: Some(receipt.id),
                order_id: None,
                supplier_id: Some(1),
                inspection_date: None,
                inspector_id: None,
                inspection_type: Some("复检".to_string()),
                sample_size: None,
                notes: None,
            },
            1,
        )
        .await
        .expect("REJECTED 收货单必须可以再建质检单（改判通道事实上打开）");
    assert_eq!(ins2.receipt_id, Some(receipt.id));
    assert_ne!(ins2.inspection_no, done1.inspection_no);
    assert_eq!(ins2.inspection_status.as_deref(), Some(pis_status::PENDING));

    // ⑤ 核心契约：第二单 complete(pass) → 回写段**无条件覆写**本列 REJECTED→PASSED
    pi_svc
        .complete_inspection(
            ins2.id,
            CompleteInspectionRequest {
                pass_quantity: dec("10.0000"),
                reject_quantity: Decimal::ZERO,
                inspection_result: purchase_inspection_result::PASS.to_string(),
            },
            1,
        )
        .await
        .expect("复检 complete(pass) 必须成功");
    let row = purchase_receipt::Entity::find_by_id(receipt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::PASSED.to_string(),
        "改判覆写契约：本列只存最终 token，REJECTED 被无条件 Set 翻成 PASSED"
    );
    // 无历史留痕的对照：fail 结论只无损保存在质检行自身，收货列不留改判痕迹
    let ins1_row = purchase_inspection::Entity::find_by_id(ins1.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        ins1_row.inspection_result.as_deref(),
        Some(purchase_inspection_result::FAIL),
        "旧质检行结论原样保留（留痕在质检行、不在收货列）"
    );

    // ⑥ 改判后放行：应付 auto-generate **真实生成成功**（不是 wave6 的缺表旁证——
    //    活库全表，这里要真单据；重复调用被防重检查拒绝，证明后续拒绝原因已是防重而非门控）
    let invoice = ap_svc
        .auto_generate_from_receipt(receipt.id, 1)
        .await
        .expect("列被覆写为 PASSED 后应付 auto-generate 必须放行并真实生成");
    assert_eq!(invoice.source_type.as_deref(), Some("PURCHASE_RECEIPT"));
    assert_eq!(invoice.source_id, Some(receipt.id));
    assert_eq!(invoice.amount, dec("100.00"), "应付金额取收货单总额");
    assert_eq!(
        invoice.invoice_status,
        STATUS_DRAFT.to_string(),
        "自动生成应付初始 DRAFT（P0 3-1 口径）"
    );
    let err = ap_svc
        .auto_generate_from_receipt(receipt.id, 1)
        .await
        .expect_err("同一收货单不得重复生成应付");
    // 防重检查用 AppError::business（脱敏族，ap_invoice_ops/receipt.rs:126）
    assert!(
        matches!(&err, AppError::BusinessError(m) if m.as_str() == "该入库单已生成应付单"),
        "第二次拒绝原因必须是防重检查而非质检门控，实际: {err:?}"
    );
    let body = err.to_response();
    assert_eq!(body.code, "BUSINESS_ERROR");

    // ⑦ 改判后确认入库放行：真实推进 COMPLETED + 库存回读（confirm 内部对已存在应付
    //    只 warn 不报错，见 state.rs publish_events_and_generate_ap 容错口径）
    let done = rc_svc
        .confirm_receipt(receipt.id, 1)
        .await
        .expect("列已 PASSED 的收货单必须可确认入库");
    assert_eq!(done.receipt_status, receipt_status::COMPLETED.to_string());
    assert!(done.confirmed_at.is_some());
    assert_eq!(
        done.inspection_status,
        purchase_receipt_inspection::PASSED.to_string(),
        "确认入库不再翻动本列（PASSED 是改判后终值）"
    );
    let stock = inventory_stock::Entity::find()
        .filter(inventory_stock::Column::BatchNo.eq(&batch))
        .one(&db)
        .await
        .unwrap()
        .expect("PASSED 确认后库存行必须真实落库");
    assert_eq!(stock.quantity_meters, dec("10.0000"));

    // 清理（应付凭证为 best-effort，可能不存在；按来源定位一并清）
    for invoice in ap_invoice::Entity::find()
        .filter(ap_invoice::Column::SourceType.eq("PURCHASE_RECEIPT"))
        .filter(ap_invoice::Column::SourceId.eq(receipt.id))
        .all(&db)
        .await
        .unwrap()
    {
        ap_invoice::Entity::delete_by_id(invoice.id)
            .exec(&db)
            .await
            .expect("清理应付单失败");
    }
    inventory_transaction::Entity::delete_many()
        .filter(inventory_transaction::Column::SourceBillNo.eq(&receipt_no))
        .exec(&db)
        .await
        .expect("清理库存流水失败");
    inventory_stock::Entity::delete_many()
        .filter(inventory_stock::Column::BatchNo.eq(&batch))
        .exec(&db)
        .await
        .expect("清理库存行失败");
    purchase_inspection::Entity::delete_by_id(ins2.id)
        .exec(&db)
        .await
        .expect("清理第二质检单失败");
    purchase_inspection::Entity::delete_by_id(ins1.id)
        .exec(&db)
        .await
        .expect("清理第一质检单失败");
    purchase_receipt_item::Entity::delete_many()
        .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt.id))
        .exec(&db)
        .await
        .expect("清理入库明细失败");
    purchase_receipt::Entity::delete_by_id(receipt.id)
        .exec(&db)
        .await
        .expect("清理入库单失败");
}

// =========================================================
// 防回潮源码/约束扫描锁（纯静态，任何环境可跑）
// =========================================================

/// 从源码截取一个 impl 内方法块：anchor 起，到首个以 4 空格缩进的 `}` 行
/// （先例 wave5/wave6 同式；先剔 \r 防 Windows 工作树 CRLF 漏检）
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

/// S1：表约束层不阻止改判——purchase_inspection.receipt_id 无 UNIQUE（建表 m0009
/// 仅普通索引；全迁移目录不存在 receipt_id 相关 UNIQUE；m0063 补的唯一约束是
/// inspection_no 单号列，不是 receipt_id）。改判通道在 schema 层就是打开的。
#[test]
fn s1_receipt_id_has_no_unique_constraint_anywhere_in_migrations() {
    let mig = include_str!("../migration/src/domain/business/m0009_add_purchase_extensions.rs")
        .replace('\r', "");
    let start = mig
        .find("CREATE TABLE IF NOT EXISTS \"purchase_inspection\"")
        .expect("m0009 质检建表语句必须存在");
    let end = mig[start..].find(");").expect("质检建表语句必须闭合");
    let table_block = &mig[start..start + end];
    let receipt_id_line = table_block
        .lines()
        .find(|l| l.contains("\"receipt_id\""))
        .expect("receipt_id 列必须存在于建表块");
    assert!(
        !receipt_id_line.to_uppercase().contains("UNIQUE"),
        "receipt_id 列定义不得出现 UNIQUE（出现即改判通道被 schema 层关闭，需同步修订本锁与词表注释），实际: {receipt_id_line}"
    );
    assert!(
        mig.contains(
            "CREATE INDEX IF NOT EXISTS \"idx_purchase_inspection_receipt\" ON \"purchase_inspection\" (\"receipt_id\")"
        ),
        "receipt_id 现状只有普通索引（非唯一）——这就是\"同 receipt 可有多张质检单\"的表级证据"
    );

    // 全迁移目录扫描：任何同时含 unique 与 receipt_id 的行、或含
    // purchase_inspection 且含 receipt_id 的 create unique index，都视为回潮。
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migration/src");
    let mut stack = vec![root];
    let mut scanned = 0usize;
    while let Some(dir) = stack.pop() {
        for entry in
            std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("迁移目录不可读 {dir:?}: {e}"))
        {
            let path = entry.expect("目录项").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("迁移文件读取失败");
            scanned += 1;
            for line in text.lines() {
                let l = line.to_lowercase();
                assert!(
                    !(l.contains("unique") && l.contains("receipt_id")),
                    "回潮：{path:?} 出现 receipt_id 的 UNIQUE 约束行: {line}"
                );
            }
        }
    }
    assert!(
        scanned > 50,
        "迁移目录扫描异常（文件数 {scanned}），本锁失效"
    );

    // m0063 给 purchase_inspection 补的 UNIQUE 是 inspection_no 单号列，不是 receipt_id
    let m63 = include_str!(
        "../migration/src/domain/production/m0063_add_document_no_unique_constraints.rs"
    );
    assert!(
        m63.contains("uq_purchase_inspection_inspection_no"),
        "m0063 单据号唯一约束锚点丢失（本锁前提：唯一约束加在单号列上）"
    );
}

/// S2：建单入口没有"已有质检单则拒"的前置——create_inspection 对 receipt 只做
/// 存在性校验（缺失 not_found），块内不出现任何按 receipt_id 查既有质检单的判定；
/// 新单恒 PENDING（service :118）。
#[test]
fn s2_create_inspection_has_no_duplicate_precheck_and_new_is_pending() {
    let src = include_str!("../src/services/purchase_inspection_service.rs").replace('\r', "");
    let block = extract_method(&src, "pub async fn create_inspection");
    assert!(
        block.contains("purchase_receipt::Entity::find_by_id(receipt_id)"),
        "建单唯一前置是收货单存在性校验:\n{block}"
    );
    assert!(
        !block.contains("purchase_inspection::Entity::find"),
        "回潮：create_inspection 出现了按质检单表自查的\"已有质检则拒\"前置——改判通道被\
         入口层关闭。若这是有意修订，必须同步修订本锁与词表注释，禁止只改一处:\n{block}"
    );
    for forbidden in ["已存在", "已有质检", "重复质检"] {
        assert!(
            !block.contains(forbidden),
            "回潮：建单出现 {forbidden:?} 形态的重复质检拒绝文案"
        );
    }
    assert!(
        block.contains("inspection_status: Set(Some(pis_status::PENDING.to_string()))"),
        "新单必须恒 PENDING（复检改判必须走 complete 才能翻收货列，建单本身不翻）:\n{block}"
    );
}

/// S3：改判覆写是无条件 Set——complete_inspection 状态门只看质检单自身(:215)，
/// 回写段从"定位收货单"到"提交"之间不存在任何对本列当前值的条件判定
/// （REJECTED 不豁免、不累加、不比对）。若有人加了"当前已 REJECTED 则拒"的护栏，
/// 本锁显式失败，要求同步修订词表注释与本契约测试，禁止注释与代码漂移。
#[test]
fn s3_writeback_overwrites_inspection_status_unconditionally() {
    let src = include_str!("../src/services/purchase_inspection_service.rs").replace('\r', "");
    let block = extract_method(&src, "pub async fn complete_inspection");

    // 状态门只引用质检单自身的 inspection_status（旧单不可再完成）
    assert!(
        block.contains("inspection.inspection_status.as_deref() != Some(pis_status::PENDING)"),
        "完成门必须只按质检单自身状态判定（service :215）:\n{block}"
    );
    assert!(
        !block.contains("receipt.inspection_status ==")
            && !block.contains("receipt.inspection_status !="),
        "回潮：complete 引入了对收货列当前值的等值/不等值前置（如\"已 REJECTED 拒改判\"）——\
         改判覆写语义已变更，必须同步修订本锁与词表注释"
    );

    // 回写段无条件：从 match receipt_id 到 commit 只有 Set，没有 if/!=/REJECTED 判定
    let seg_start = block
        .find("match receipt_id {")
        .expect("回写段锚点 match receipt_id 必须存在");
    let seg_end = block.find("txn.commit()").expect("commit 锚点必须存在");
    let seg = &block[seg_start..seg_end];
    assert!(
        seg.contains(
            "receipt_active.inspection_status = Set(receipt_inspection_status.to_string())"
        ),
        "回写必须是无条件 Set 映射值（:265）:\n{seg}"
    );
    assert!(
        !seg.contains("if ") && !seg.contains("!=") && !seg.contains("REJECTED"),
        "回潮：回写段出现条件判定/REJECTED 特判——无条件覆写契约被改动而未同步本锁:\n{seg}"
    );
}

/// S4：词表边界锁——本列合法 token 恒为三态；「让步接收（特采/降级接收）」没有
/// token（未实现）。第四态一旦出现（无论叫 CONCESSION 还是别的），本锁先炸。
#[test]
fn s4_receipt_inspection_vocabulary_has_no_concession_token() {
    assert_eq!(
        purchase_receipt_inspection::ALL,
        &["PENDING", "PASSED", "REJECTED"],
        "purchase_receipt.inspection_status 词表恒三态；让步接收无 token（未实现），\
         复检改判复用 PASSED/REJECTED 覆写而非独立状态"
    );
    // 映射取值域同样只有两个目标态，不存在第四种落点
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
        Some(purchase_receipt_inspection::REJECTED)
    );
    // 非法值（含大小写变体/中文跨域/空串）在映射与白名单两处都进不去——
    // "特采"之类的私改判绝不能从结论 token 混进来
    for outside in ["PASS", "concession", "特采", "让步接收", "让步", ""] {
        assert_eq!(
            purchase_inspection_result::to_receipt_inspection_status(outside),
            None,
            "词表外 {outside:?} 必须 None（改判只能走 pass/fail/partial 三 token 的覆写）"
        );
        assert!(!purchase_inspection_result::is_valid(outside));
    }
}
