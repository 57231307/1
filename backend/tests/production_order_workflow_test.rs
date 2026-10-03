//! P0-T02 生产订单全流程集成测试（V15 Batch 487）
//!
//! 覆盖：状态常量值 + Service 实例化 + DB 异常路径（各条按被测函数体在**真库化夹具**
//! 下的真实契约逐条校准，见各用例文档注释；#4671 判责 §⑤ W4 / 裁决 R-9）
//! 纯状态机校验函数 validate_status_transition 为私有方法，通过 DB 异常路径间接验证状态门逻辑。
//! 完整业务流程测试（create → submit → approve → complete）需要真实 PostgreSQL，标记 #[ignore]。

mod test_common;

use std::sync::Arc;

use bingxi_backend::models::status::{common, production};
use bingxi_backend::services::production_order_service::{
    CreateProductionOrderRequest, ProductionOrderQuery, ProductionOrderService,
};
use rust_decimal::Decimal;
use sea_orm::Database;
// 批次 490 D10-3b 修复：使用 super:: 限定本地 mod common，避免被 status::common 遮蔽
use bingxi_backend::models::status::common::STATUS_COMPLETED;
use bingxi_backend::models::status::common::STATUS_DRAFT;
use chrono::NaiveDate;
use test_common::setup_test_db;

/// 构造最小 CreateProductionOrderRequest（仅必填字段）
///
/// 无 order_no 字段（任务 #153 缺陷3：单据号禁手输，一律服务端取号）；
/// planned_quantity 为必填 Decimal（NOT NULL 列，无 Option/默认值兜底）。
fn sample_create_request() -> CreateProductionOrderRequest {
    CreateProductionOrderRequest {
        sales_order_id: None,
        product_id: 1,
        planned_quantity: Decimal::new(1000, 0),
        planned_start_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        planned_end_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap()),
        priority: Some(1),
        work_center_id: Some(1),
        remarks: None,
        created_by: 1,
    }
}

// ===== 状态常量值正确性 =====

/// test_scddztcl_zzqx
///
/// 验证生产订单相关的状态常量值符合预期（大写风格）。
#[test]
fn test_scddztcl_zzqx() {
    assert_eq!(common::STATUS_DRAFT, "DRAFT");
    assert_eq!(common::STATUS_APPROVED, "APPROVED");
    assert_eq!(common::STATUS_CANCELLED, "CANCELLED");
    assert_eq!(common::STATUS_COMPLETED, "COMPLETED");
    assert_eq!(production::PRODUCTION_SCHEDULED, "SCHEDULED");
    assert_eq!(production::PRODUCTION_IN_PROGRESS, "IN_PROGRESS");
    assert_eq!(production::PRODUCTION_PENDING_APPROVAL, "PENDING_APPROVAL");
    assert_eq!(production::PRODUCTION_REJECTED, "REJECTED");
}

/// test_scddztcl_dxfgyzx
///
/// 验证所有生产订单状态常量均为大写 + 下划线风格。
#[test]
fn test_scddztcl_dxfgyzx() {
    let statuses = [
        common::STATUS_DRAFT,
        common::STATUS_APPROVED,
        common::STATUS_CANCELLED,
        common::STATUS_COMPLETED,
        production::PRODUCTION_SCHEDULED,
        production::PRODUCTION_IN_PROGRESS,
        production::PRODUCTION_PENDING_APPROVAL,
        production::PRODUCTION_REJECTED,
    ];
    for s in statuses {
        assert!(
            s.chars().all(|c| c.is_uppercase() || c == '_'),
            "状态 {} 应全大写",
            s
        );
    }
}

// ===== Service 实例化与 DB 异常路径 =====

/// test_productionorderservice_slhbcfdb
///
/// 验证 new(db) 仅存储 Arc<DatabaseConnection>，不执行任何 DB 查询。
#[tokio::test]
async fn test_productionorderservice_slhbcfdb() {
    let db = setup_test_db().await;
    let svc = ProductionOrderService::new(Arc::new(db));
    // 仅验证实例化成功，不调用任何方法
    let _ = svc;
}

/// test_productionorderservice_create_kdbfherr —— 真库化夹具前提校准（#4671 判责
/// §⑤ W4；手法照抄 ap_payment_workflow_test 的 R-9 拆分范本）：
/// 钉"已建库空表上 create（引用不存在的产品/工作中心）返回 **VALIDATION_ERROR**
/// 机器码而非 panic"。
///
/// 真实契约依据（读函数体，非读注释）：`production_order_ops/crud.rs:174-185` create
/// 第一步 `validate_create_references`（:188-213）→ `validate_product_exists`（:48-56）
/// 在空 `products` 表（`products` **不在** `test_common.rs:35-55` 种子白名单、会被
/// TRUNCATE）上查不到 product_id=1 ⇒ `AppError::validation`。
/// ⇒ `is_err()` 方向成立，原注释"product 表不存在"的**前提**过期（表存在、只是空），
/// 断言由裸 `is_err()` 收紧为钉机器码——夹具若退化成 schema 缺失（DATABASE_ERROR）
/// 或门控丢失（竟然 Ok）都必须显形，这不是掩盖源码缺陷：引用校验正是该调用的生产契约。
#[tokio::test]
async fn test_productionorderservice_create_kdbfherr() {
    let db = setup_test_db().await;
    let svc = ProductionOrderService::new(Arc::new(db));
    let req = sample_create_request();
    let err = svc
        .create(req)
        .await
        .expect_err("已建库空表上引用不存在产品的 create 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "create 必须命中引用校验族（crud.rs:52），实得 {}",
        err.error_code()
    );
}

/// test_productionorderservice_get_by_id_kdbfherr —— 依据裁决 R-9（同 ap_payment
/// `get_by_id` 先例，判责依据 ci4671-triage.md §2.3 B3 :139）：**必须改绑**
/// `connect_empty_schema_db()`，钉"schema 缺失（production_orders 表根本不存在）时
/// 返回 Err 而非 panic"。
///
/// 为什么不能留在 `setup_test_db()` 上断 Err：读函数体确认
/// `production_order_ops/crud.rs:375-402` get_by_id 返回 `Result<Option<Dto>, _>`，
/// 已建库空表上 `.one()` 得 None ⇒ **Ok(None)** 才是真实契约，`is_err()` 在真库化
/// 夹具下必红——原断言的隐含前提是"SQLite 内存库无 schema"（文件头旧注释自陈），
/// 该前提已随 b09614f7 真库化失效。留 Ok(None) 判据会丢掉本条原本要锁的
/// "缺表必须报错、绝不静默吞成不存在"防线，故按 R-9 拆到空 schema 库上，
/// **断言语义不变**（判责原文 §⑤ W4 对"无 schema 族"的指定正解）。
#[tokio::test]
async fn test_productionorderservice_get_by_id_kdbfherr() {
    let db = test_common::connect_empty_schema_db().await;
    let svc = ProductionOrderService::new(Arc::new(db));
    let result = svc.get_by_id(1, None).await;
    assert!(
        result.is_err(),
        "schema 缺失（无 production_orders 表）时 get_by_id 必须返回 Err 而非 panic"
    );
}

/// test_productionorderservice_list_kdbfherr —— 依据裁决 R-9 拆前提后钉另一件事：
/// 已建库、业务表已清空 ⇒ `list` 不 panic 且返回**空集**（total=0）。
///
/// 真实契约依据（读函数体）：`production_order_ops/crud.rs:423-` list 对空表
/// LEFT JOIN 分页返回 `Ok(([], 0))`；原断 `is_err()` 是把真库化夹具当"空 SQLite"
/// 的过期前提（判责原文 ci4671-triage.md :139/:141，§⑤ W4）。schema 缺失的报错
/// 形态已由上一条用例在 `bingxi_empty` 上钉死，本条不重复、不放宽。
#[tokio::test]
async fn test_productionorderservice_list_kdbfherr() {
    let db = setup_test_db().await;
    let svc = ProductionOrderService::new(Arc::new(db));
    let query = ProductionOrderQuery {
        order_no: None,
        status: None,
        product_id: None,
        page: 1,
        page_size: 20,
    };
    let (items, total) = svc
        .list(query, None)
        .await
        .expect("已建库空表上 list 应返回 Ok 空集，而非 Err/panic");
    assert!(
        items.is_empty(),
        "夹具已 TRUNCATE 业务表，列表必须是空集，实得 {} 行",
        items.len()
    );
    assert_eq!(total, 0, "空表的 total 计数应为 0，实得 {total}");
}

/// test_productionorderservice_submit_for_approval_kdbfherr —— 同族逐条核：读函数体
/// `production_order_ops/approval.rs:34-46`，submit 先 begin 再 find_by_id +
/// lock_exclusive，空表 ⇒ None ⇒ `AppError::not_found` ⇒ 在已建库空表上**确实返回
/// Err**，前提改法为"记录不存在 ⇒ NOT_FOUND"，裸 `is_err()` 收紧为钉机器码。
#[tokio::test]
async fn test_productionorderservice_submit_for_approval_kdbfherr() {
    let db = setup_test_db().await;
    let svc = ProductionOrderService::new(Arc::new(db));
    let err = svc
        .submit_for_approval(1, 1, "测试用户")
        .await
        .expect_err("已建库空表上提交不存在的订单必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "空表 submit 必须命中 not_found（approval.rs:46），实得 {}",
        err.error_code()
    );
}

/// test_productionorderservice_approve_order_kdbfherr —— 同族：approval.rs:100-112
/// → `lock_and_validate_order_for_approval_txn`（:135 `ok_or_else(not_found)`），
/// 空表 ⇒ None ⇒ Err(NOT_FOUND)。收紧为钉机器码。
#[tokio::test]
async fn test_productionorderservice_approve_order_kdbfherr() {
    let db = setup_test_db().await;
    let svc = ProductionOrderService::new(Arc::new(db));
    let err = svc
        .approve_order(1, 1, "测试用户", true, None)
        .await
        .expect_err("已建库空表上审批不存在的订单必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "空表 approve 必须命中 not_found（approval.rs:135），实得 {}",
        err.error_code()
    );
}

// ===== 完整业务流程测试（需要真实 PostgreSQL，标记 ignore）=====

/// 集成测试：生产订单全流程 create → submit → approve → schedule → in_progress → complete
///
/// 需要 PostgreSQL 测试数据库（表结构 + 产品/工作中心/销售订单前置数据）。
/// 设置 TEST_DATABASE_URL=postgres://... 环境变量后运行：cargo test -- --ignored
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库 + 前置产品/工作中心数据"]
async fn test_scddqlc_cjdwc() {
    let db_url = std::env::var("TEST_DATABASE_URL").expect("需设置 TEST_DATABASE_URL 环境变量");
    let db = Database::connect(&db_url).await.expect("DB 连接失败");
    let svc = ProductionOrderService::new(Arc::new(db));

    // 1. 创建（DRAFT）
    let req = sample_create_request();
    let order = svc.create(req).await.expect("创建失败");
    assert_eq!(order.status, common::STATUS_DRAFT);

    // 2. 提交审批（DRAFT → PENDING_APPROVAL）
    let order = svc
        .submit_for_approval(order.id, 1, "测试用户")
        .await
        .expect("提交审批失败");
    assert_eq!(order.status, production::PRODUCTION_PENDING_APPROVAL);

    // 3. 审批通过（PENDING_APPROVAL → APPROVED）
    let order = svc
        .approve_order(order.id, 1, "审批人", true, None)
        .await
        .expect("审批失败");
    assert_eq!(order.status, common::STATUS_APPROVED);

    // 4. 排产（APPROVED → SCHEDULED）
    let order = svc
        .update_status(order.id, production::PRODUCTION_SCHEDULED.to_string(), None)
        .await
        .expect("排产失败");
    assert_eq!(order.status, production::PRODUCTION_SCHEDULED);

    // 5. 开工（SCHEDULED → IN_PROGRESS）
    let order = svc
        .update_status(
            order.id,
            production::PRODUCTION_IN_PROGRESS.to_string(),
            None,
        )
        .await
        .expect("开工失败");
    assert_eq!(order.status, production::PRODUCTION_IN_PROGRESS);

    // 6. 完工（IN_PROGRESS → COMPLETED，需 actual_quantity）
    let order = svc
        .update_status(
            order.id,
            common::STATUS_COMPLETED.to_string(),
            Some(Decimal::new(1000, 0)),
        )
        .await
        .expect("完工失败");
    assert_eq!(order.status, common::STATUS_COMPLETED);
}
