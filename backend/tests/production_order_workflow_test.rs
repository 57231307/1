//! 生产订单全流程集成测试
//!
//! 覆盖：状态常量值 + Service 实例化 + DB 异常路径（各条按被测函数体在**真库化夹具**
//! 下的真实契约逐条校准，见各用例文档注释）
//! 纯状态机校验函数 validate_status_transition 为私有方法，通过 DB 异常路径间接验证状态门逻辑。
//! 完整业务流程测试（create → submit → approve → complete）需要真实 PostgreSQL，标记 #[ignore]。

mod test_common;

use std::sync::Arc;

use bingxi_backend::models::status::{common, production};
use bingxi_backend::services::production_order_service::{
    CreateProductionOrderRequest, ProductionOrderQuery, ProductionOrderService,
};
use rust_decimal::Decimal;
use bingxi_backend::models::status::common::STATUS_COMPLETED;
use bingxi_backend::models::status::common::STATUS_DRAFT;
use chrono::NaiveDate;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use test_common::setup_test_db;

/// 构造最小 CreateProductionOrderRequest（仅必填字段）
///
/// 无 order_no 字段（单据号禁手输，一律服务端取号）；
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
    }
}

// ===== 状态常量值正确性 =====

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

/// 验证 new(db) 仅存储 Arc<DatabaseConnection>，不执行任何 DB 查询。
#[tokio::test]
async fn test_productionorderservice_slhbcfdb() {
    let db = setup_test_db().await;
    let svc = ProductionOrderService::new(Arc::new(db));
    // 仅验证实例化成功，不调用任何方法
    let _ = svc;
}

/// 钉"已建库空表上 create（引用不存在的产品/工作中心）返回 **VALIDATION_ERROR**
/// 机器码而非 panic"。
///
/// 契约依据（读函数体，非读注释）：`production_order_ops/crud.rs:174-186` create
/// 第一步 `validate_create_references`（:189-214）→ `validate_product_exists`（:48-58）
/// 在空 `products` 表（`products` **不在** `test_common.rs:40-60` 种子白名单、会被
/// TRUNCATE）上查不到 product_id=1 ⇒ `AppError::validation`。
/// 断言钉机器码而非裸 `is_err()`：夹具若退化成 schema 缺失（DATABASE_ERROR）
/// 或门控丢失（竟然 Ok）都必须显形——引用校验正是该调用的生产契约。
#[tokio::test]
async fn test_productionorderservice_create_kdbfherr() {
    let db = setup_test_db().await;
    let svc = ProductionOrderService::new(Arc::new(db));
    let req = sample_create_request();
    let err = svc
        .create(req, 1)
        .await
        .expect_err("已建库空表上引用不存在产品的 create 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "create 必须命中引用校验族（crud.rs:52），实得 {}",
        err.error_code()
    );
}

/// test_productionorderservice_get_by_id_kdbfherr —— 同 ap_payment_workflow_test.rs
/// 的 get_by_id 拆分先例：**必须改绑**
/// `connect_empty_schema_db()`，钉"schema 缺失（production_orders 表根本不存在）时
/// 返回 Err 而非 panic"。
///
/// 为什么不能留在 `setup_test_db()` 上断 Err：读函数体确认
/// `production_order_ops/crud.rs:375-402` get_by_id 返回 `Result<Option<Dto>, _>`，
/// 已建库空表上 `.one()` 得 None ⇒ **Ok(None)** 才是真实契约，`is_err()` 在真库化
/// 夹具下必红——原断言的隐含前提是"SQLite 内存库无 schema"（文件头旧注释自陈），
/// 该前提已随夹具真库化失效。留 Ok(None) 判据会丢掉本条原本要锁的
/// "缺表必须报错、绝不静默吞成不存在"防线，故拆到空 schema 库上，
/// **断言语义不变**（"无 schema 族"的指定正解）。
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
/// 的过期前提。schema 缺失的报错
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
/// 真库化前提补齐（CI §A.2 种子族，只补剩下这条前提，600e5640 已提交的
/// 判据用例不动）：create 的第一步是引用校验（读函数体
/// `production_order_ops/crud.rs`：`validate_create_references` →
/// `validate_product_exists`（:48-56，products 空表 ⇒ ValidationError("产品ID 1 不存在")
/// = 本轮 CI 红因原文）→ `validate_work_center_exists`（:76-88，work_centers 空表
/// ⇒ ValidationError("工作中心ID 1 不存在")），故必须种 **products(1) 与
/// work_centers(1)** 两父行。列形态按真表 DDL：products(code/name NOT NULL，
/// 范式同 contract_wave5_receipt_return_three_state_test.rs:154）、
/// work_centers(code UNIQUE NOT NULL / name NOT NULL，
/// migration/src/domain/business/m0007_add_mrp_production_bom.rs:56-66）。
/// 走 `setup_test_db()`（已迁移 PG + TRUNCATE + RESTART IDENTITY）使显式 id=1 与
/// `sample_create_request()` 的引用恒对齐；ignored lane `--test-threads=1` 串行，
/// 无同库竞态。
/// products 种子列形态按 **实体整 Model 解码需求** 而非仅 DDL NOT NULL：
/// create 引用校验 `validate_product_exists` 走 `ProductEntity::find_by_id(...).one()`
/// 全行解码（production_order_ops/crud.rs:49），product::Model 声明
/// unit/status/product_type 为非 Option String（models/product.rs:26/35/45），
/// 三列的生效 DDL 却是可空无默认的后补列（system/mod.rs:325/328 及该表后续补充列的迁移建列项）；
/// 写入侧三者恒非空（handlers/product_handler.rs:383-390 默认 "个"/master_data::ACTIVE
/// ("active")/"成品"，services/product_ops/crud.rs:210 Set(unit)），故种子必须补齐，
/// 否则解码报 `Missing value for column 'unit'`。取面料域真实词值：米 / active / 成品布。
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库 + 前置产品/工作中心数据"]
async fn test_scddqlc_cjdwc() {
    let db = setup_test_db().await;
    for (sql, what) in [
        (
            "INSERT INTO products (id, code, name, unit, status, product_type) VALUES \
             (1, 'PT-WF-P1', '生产全流程测试面料', '米', 'active', '成品布')",
            "products 父行",
        ),
        (
            "INSERT INTO work_centers (id, code, name) VALUES \
             (1, 'WC-WF-1', '生产全流程测试工作中心')",
            "work_centers 父行",
        ),
    ] {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .unwrap_or_else(|e| panic!("种子 {what} 写入失败: {e}\nSQL: {sql}"));
    }
    let svc = ProductionOrderService::new(Arc::new(db));

    // 1. 创建（DRAFT）
    let req = sample_create_request();
    let order = svc.create(req, 1).await.expect("创建失败");
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
