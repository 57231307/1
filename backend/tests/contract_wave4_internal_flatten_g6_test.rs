//! G6（PR #942 清理波）internal 重包拍平缺陷契约测
//!
//! 缺陷本体：`AppError::internal(format!("{e}"))` / `AppError::internal(e.to_string())`
//! 包裹**已经返回 AppError 或已可归类**的调用，把服务层的 400/403/404 拍平成 500，
//! 并丢失真实 code 与文案（用户侧"提交后出现请求错误"的头号根因）。
//!
//! 本测三层锁：
//! 1. shrink-only ratchet（源码扫描）：G6 清单内每文件 `AppError::internal(` 出现次数
//!    只许减不许增，上限即本轮修复后的基线；清单外文件不在此锁范围
//!    （`services/ar_ops/report.rs` 由 G3 并行撰写、`services/so/delivery_ops/inventory.rs`
//!    由库存接管专家撰写，均不属于本组，故不进表）。
//! 2. 拍平指纹零命中：`AppError::internal(e.to_string())`（把未知/已带族的错误强行
//!    降级成 500 的写法）在本组任何文件不得出现；多行 `map_err(|e| { … AppError::internal }`
//!    块同样禁止（voucher 批量查询旧写法形态）。
//! 3. 真实断言（非纯源码锁）：
//!    - sqlite 自建空表，真实调用业务追溯 handler：不存在的 five_dimension_id →
//!      404 NOT_FOUND 且真实原因「未找到追溯链」随 Display/日志外达，不得再是被拍平的 500；
//!      不存在的 trace_chain_id 建快照 → 服务层 not_found「追溯链不存在」原样透传出 404
//!      （修复前该 404 被 handler `map_err(internal)` 压成 500——本测即该站点的回归锁）。
//!    - 活库（#[ignore]，CI 的 nextest --run-ignored 作业以已迁移 PostgreSQL 执行）：
//!      付款申请非草稿状态的修改状态门 → 400 BUSINESS_ERROR 而非 500。
//!
//! 保留 internal 的站点与理由见 PR 描述（IO/reqwest/env 配置/serde 序列化/聚合"无结果"
//! 不变量等不可归类场景，按判据 (b)(c)(e) 保留，出参仍走脱敏）。

mod test_common;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::business_trace_handler;
use bingxi_backend::utils::error::AppError;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use std::sync::Arc;

// =========================================================
// 1/2. 源码 ratchet + 拍平指纹扫描
// =========================================================

/// (文件标识, `AppError::internal(` 出现次数上限=本轮修复后基线, 源码)
/// shrink-only：后续修复只会让计数继续下降，任何回升即判红。
const G6_FILES: &[(&str, usize, &str)] = &[
    (
        "src/services/voucher_ops/crud.rs",
        0,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/services/voucher_ops/crud.rs"
        )),
    ),
    (
        "src/services/ap_report_service.rs",
        5,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/services/ap_report_service.rs"
        )),
    ),
    (
        "src/handlers/supplier_handler.rs",
        9,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/handlers/supplier_handler.rs"
        )),
    ),
    (
        "src/services/business_trace_service.rs",
        5,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/services/business_trace_service.rs"
        )),
    ),
    (
        "src/handlers/business_trace_handler.rs",
        0,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/handlers/business_trace_handler.rs"
        )),
    ),
    (
        "src/handlers/field_permission_handler.rs",
        0,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/handlers/field_permission_handler.rs"
        )),
    ),
    (
        "src/handlers/fund_management_handler.rs",
        4,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/handlers/fund_management_handler.rs"
        )),
    ),
    (
        "src/services/finance_report_service.rs",
        2,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/services/finance_report_service.rs"
        )),
    ),
    (
        "src/handlers/webhook_integration_handler.rs",
        2,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/handlers/webhook_integration_handler.rs"
        )),
    ),
    (
        "src/services/report/job.rs",
        2,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/services/report/job.rs"
        )),
    ),
    (
        "src/services/purchase_delivery_calculator.rs",
        0,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/services/purchase_delivery_calculator.rs"
        )),
    ),
    (
        "src/handlers/purchase_return_handler.rs",
        0,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/handlers/purchase_return_handler.rs"
        )),
    ),
    (
        "src/handlers/color_card/issue.rs",
        0,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/handlers/color_card/issue.rs"
        )),
    ),
];

/// 判据 (a) 的回归锁：`AppError::internal(e.to_string())` 是"把已带族/已可归类的错误
/// 统一拍平成 500"的指纹，G6 清单内必须零命中，且不得回潮。
#[test]
fn g6_no_internal_rewrap_fingerprint() {
    for (file, _cap, src) in G6_FILES {
        assert!(
            !src.contains("AppError::internal(e.to_string())"),
            "{file}: 仍存在 AppError::internal(e.to_string()) 重包——把 ?/From 透传改回来的回潮写法"
        );
        // 多行 map_err(|e| { … AppError::internal( … ) }) 块（voucher 批量查询旧形态）：
        // 该形态要么包 AppError（必须 ?），要么包 DbErr（必须 From<DbErr> 归类 + ERROR 日志）
        let lines: Vec<&str> = src.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains(".map_err(|e| {") {
                let window = lines[i..(i + 4).min(lines.len())].join("\n");
                assert!(
                    !window.contains("AppError::internal("),
                    "{file}:{} map_err 块内重新出现 AppError::internal 重包",
                    i + 1
                );
            }
        }
    }
}

/// shrink-only ratchet：每文件 `AppError::internal(` 计数只许 ≤ 基线。
#[test]
fn g6_internal_count_is_shrink_only_ratchet() {
    for (file, cap, src) in G6_FILES {
        let n = src.matches("AppError::internal(").count();
        assert!(
            n <= *cap,
            "{file}: AppError::internal( 出现 {n} 次，超过本组修复后基线 {cap}——禁止新增重包"
        );
    }
}

/// 定向坐实 1：业务追溯 4 端点必须 `?` 透传服务层 AppError（含 404），
/// handler 文件计数锁 0 的同时，再锁服务层"不存在即 not_found"的语义字面量。
#[test]
fn g6_trace_handler_passthrough_and_service_not_found_semantics() {
    let handler = G6_FILES
        .iter()
        .find(|(f, _, _)| *f == "src/handlers/business_trace_handler.rs")
        .unwrap()
        .2;
    assert!(
        handler.contains("AppError::not_found(\"未找到追溯链\")"),
        "five_dimension 空链必须显式 404「未找到追溯链」，不得改回列表空 200 或被拍平"
    );
    let service = G6_FILES
        .iter()
        .find(|(f, _, _)| *f == "src/services/business_trace_service.rs")
        .unwrap()
        .2;
    assert!(
        service.contains("AppError::not_found(\"追溯链不存在\")"),
        "create_snapshot 对不存在链必须 not_found（404），这是 handler 透传后的真实语义源"
    );
}

// =========================================================
// 3. 真实断言（sqlite 自建空表，直连真实 handler）
// =========================================================

async fn sqlite_db() -> sea_orm::DatabaseConnection {
    sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败")
}

/// 与 models/business_trace_chain.rs::Model 列逐一对应的 sqlite 建表（空表即"单据不存在"场景）
async fn create_business_trace_chain_table(db: &sea_orm::DatabaseConnection) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"CREATE TABLE business_trace_chain (
            id INTEGER PRIMARY KEY,
            trace_chain_id TEXT, five_dimension_id TEXT,
            product_id INTEGER, batch_no TEXT, color_no TEXT,
            dye_lot_no TEXT, grade TEXT,
            current_stage TEXT, current_bill_type TEXT, current_bill_no TEXT,
            current_bill_id INTEGER, previous_trace_id INTEGER, next_trace_id INTEGER,
            quantity_meters TEXT, quantity_kg TEXT,
            warehouse_id INTEGER, supplier_id INTEGER, customer_id INTEGER, workshop_id INTEGER,
            trace_status TEXT, remarks TEXT,
            created_at TEXT, created_by INTEGER
        )"#,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("business_trace_chain 建表失败: {e}"));
}

fn trace_state(db: sea_orm::DatabaseConnection) -> AppState {
    let mut state = AppState::default();
    state.db = Arc::new(db);
    state
}

/// 真实断言 A：不存在的 five_dimension_id 调真实 handler → 404 + NOT_FOUND + 真实原因
/// （修复前：服务层 Ok(空)→handler 自身 not_found 尚可；但链路上任何 Err——含同表
/// create_snapshot 的 404——都会被 `.map_err(|e| AppError::internal(e.to_string()))` 拍成 500）
#[tokio::test]
async fn missing_five_dimension_trace_returns_404_not_flattened_500() {
    let db = sqlite_db().await;
    create_business_trace_chain_table(&db).await;
    let state = trace_state(db);

    let err = business_trace_handler::get_trace_by_five_dimension(
        State(state),
        Path("FD-不存在的五维".to_string()),
    )
    .await
    .expect_err("追溯链不存在必须拒绝，不得空 200");

    assert!(
        matches!(err, AppError::NotFound(_)),
        "必须原样透传服务/handler 的 404 族，实际={err:?}"
    );
    assert_eq!(err.error_code(), "NOT_FOUND");
    assert!(
        err.to_string().contains("未找到追溯链"),
        "真实原因必须留在错误对象（Display/日志侧），实际={err}"
    );

    let resp = err.clone().into_response();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "修复前该链路 Err 被拍平成 500——404 即本测回归锁"
    );
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["code"], "NOT_FOUND");
}

/// 真实断言 B：不存在的 trace_chain_id 建快照 → 服务层 not_found「追溯链不存在」
/// 经 handler `?` 原样透传出 404（修复前 handler `map_err(internal)` 把这条 404 压成 500，
/// 正是普查点名的"单据不存在被拍平"站点）。
#[tokio::test]
async fn missing_trace_chain_snapshot_returns_404_with_real_reason() {
    let db = sqlite_db().await;
    create_business_trace_chain_table(&db).await;
    let state = trace_state(db);

    let err = business_trace_handler::create_trace_snapshot(
        State(state),
        Path("TC-不存在的链".to_string()),
    )
    .await
    .expect_err("链不存在时快照必须被拒");

    assert_eq!(err.error_code(), "NOT_FOUND", "实际={err:?}");
    assert!(
        err.to_string().contains("追溯链不存在"),
        "服务层真实原因必须透传，实际={err}"
    );
    let resp = err.into_response();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "此站点修复前恒 500——404 为回归锁"
    );
}

// =========================================================
// 3'. 真实断言 C（活库 #[ignore]，CI nextest --run-ignored 作业执行）
// =========================================================

/// 付款申请状态门：非草稿（APPROVING）修改必须 400 BUSINESS_ERROR 而非 500。
/// update 走事务内 lock_exclusive，sqlite 方言不支持，需 TEST_DATABASE_URL 指向已迁移 PG。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL：update 状态门在事务内 lock_exclusive，sqlite 不支持"]
async fn ap_payment_request_state_gate_is_business_400_not_flattened_500() {
    use bingxi_backend::models::ap_payment_request;
    use bingxi_backend::models::status::{ap_payment_request as approval, common};
    use bingxi_backend::services::ap_payment_request_service::{
        ApPaymentRequestService, UpdateApPaymentRequest,
    };
    use rust_decimal::Decimal;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let db = test_common::setup_test_db().await;
    let seeded = ap_payment_request::ActiveModel {
        request_no: Set(format!(
            "PRG6-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )),
        request_date: Set(chrono::Utc::now().naive_utc().date()),
        supplier_id: Set(1),
        payment_type: Set("PREPAYMENT".to_string()),
        payment_method: Set("TT".to_string()),
        request_amount: Set(Decimal::new(1000, 0)),
        approval_status: Set(approval::APPROVAL_APPROVING.to_string()),
        created_by: Set(100),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("插入 APPROVING 付款申请失败（需已迁移 PG 且 supplier_id=1/user 100 存在，同 wave1 活库夹具）");
    assert_ne!(seeded.approval_status, common::STATUS_DRAFT);

    let db = Arc::new(db);
    let svc = ApPaymentRequestService::new(db.clone());
    let err = svc
        .update(
            seeded.id,
            UpdateApPaymentRequest {
                request_date: None,
                payment_type: None,
                payment_method: None,
                request_amount: Some(Decimal::new(2000, 0)),
                expected_payment_date: None,
                bank_name: None,
                bank_account: None,
                bank_account_name: None,
                notes: None,
                attachment_urls: None,
            },
            100,
        )
        .await
        .expect_err("APPROVING 状态修改必须被状态门拒绝");

    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "状态门归业务族且不得被拍平成 INTERNAL_ERROR，实际={err:?}"
    );
    assert!(
        err.to_string().contains("不可修改"),
        "真实拒绝原因必须留在错误对象，实际={err}"
    );

    let resp = err.into_response();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "付款申请状态门必须 400，而非被 internal 重包后的 500"
    );

    // 夹具自清：删除本轮插入的申请，避免污染同库后续用例
    ap_payment_request::Entity::delete_by_id(seeded.id)
        .exec(&*db)
        .await
        .expect("夹具清理失败：ap_payment_request 行残留将污染同库其他用例");
}
