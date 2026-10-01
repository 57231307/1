//! 定制订单状态词表三源合一契约锁（change_pending 收口批次）
//!
//! 锁定三条（对应判责证据的五方对齐表）：
//! ① 大额变更挂起回读 == change_pending：
//!    - sqlite CHECK 同构：按 m0064 重建后的 11 值集合建同构约束，权威模块
//!      `custom_order::ALL` 全部 token 逐一写入成功（旧 10 值 CHECK 下
//!      change_pending 必违反约束冒 500，本用例即"不再违反 CHECK"的证明）；
//!      词表外 token 必须被同构 CHECK 显式拒绝（约束不是摆设）；
//!    - 活库（PG，#[ignore]，ci-test-rust-ignored 执行）：真实 service 链
//!      `submit_change_request`（金额变化>1万 → lock_exclusive 事务写入，
//!      sqlite 方言不支持行锁，不得伪装成 sqlite 用例）挂起后回读，
//!      再 `approve_change` 回 draft 双向闭环。
//! ② 源码扫描锁：`custom_order_crud_service.rs` 状态写入/比较行不得出现
//!    `Set("…")` / `!= "…"` 裸字面量，`change_pending` 字面量整文件禁现
//!    （写入方与比较点必须逐字符取自权威模块常量）。
//! ③ 词表-CHECK 双向锁：`models/status/sales.rs::custom_order::ALL` 与
//!    m0064 迁移 CHECK 取值集合逐项相等（防一边改一边忘）。

mod test_common;

use bingxi_backend::models::custom_order;
use bingxi_backend::models::custom_order_create_dto::UpdateCustomOrderDto;
use bingxi_backend::models::status::custom_order as co_status;
use bingxi_backend::services::custom_order_crud_service::CustomOrderCrudService;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Statement, TransactionTrait,
};
use std::str::FromStr;
use std::sync::Arc;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

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

/// custom_orders 全列最小 DDL（列与 `models/custom_order.rs::Model` 逐列对应，
/// bool→INTEGER / DateTime→TEXT / Decimal→TEXT，照 wave5 先例形态），
/// CHECK 取值集合与迁移 m0064 重建后的 `chk_custom_order_status` 逐字符同构。
const CUSTOM_ORDERS_CHECK_ISO_DDL: &str = r#"CREATE TABLE custom_orders (
    id INTEGER PRIMARY KEY,
    order_no TEXT NOT NULL UNIQUE,
    customer_id INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    color_id INTEGER,
    spec TEXT NOT NULL,
    quantity TEXT NOT NULL,
    unit TEXT NOT NULL,
    custom_requirements TEXT NOT NULL,
    yarn_spec TEXT,
    dye_method TEXT,
    finishing_method TEXT,
    status TEXT NOT NULL,
    expected_delivery_date TEXT,
    actual_delivery_date TEXT,
    sales_order_id INTEGER,
    total_amount TEXT,
    currency TEXT NOT NULL,
    created_by INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    notes TEXT,
    lab_dip_request_id INTEGER,
    quotation_id INTEGER,
    customer_approved_at TEXT,
    customer_approval_comment TEXT,
    quality_standard_id INTEGER,
    approval_instance_id INTEGER,
    approved_by INTEGER,
    approved_at TEXT,
    rejection_reason TEXT,
    CHECK (status IN ('draft', 'lab_dip', 'quotation', 'yarn_purchasing',
        'dyeing', 'finishing', 'delivery', 'after_sales', 'change_pending',
        'completed', 'cancelled'))
)"#;

async fn seed_order(db: &DatabaseConnection, order_no: &str, status: &str) -> custom_order::Model {
    let now = Utc::now();
    custom_order::ActiveModel {
        order_no: sea_orm::Set(order_no.to_string()),
        customer_id: sea_orm::Set(1),
        product_id: sea_orm::Set(1),
        spec: sea_orm::Set("测试规格".to_string()),
        quantity: sea_orm::Set(dec("10.00")),
        unit: sea_orm::Set("m".to_string()),
        custom_requirements: sea_orm::Set(serde_json::json!({})),
        status: sea_orm::Set(status.to_string()),
        currency: sea_orm::Set("CNY".to_string()),
        created_at: sea_orm::Set(now),
        updated_at: sea_orm::Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

// =========================================================
// ①-a sqlite CHECK 同构：权威模块全集逐 token 写入通过 + 词表外拒绝
// =========================================================

#[tokio::test]
async fn change_pending_passes_check_iso_and_all_tokens_are_writable() {
    let db = sqlite_db().await;
    exec(&db, CUSTOM_ORDERS_CHECK_ISO_DDL).await;

    // 权威模块 ALL 的每个 token 都必须能通过与 m0064 同构的 CHECK——
    // 任何一个被拒即"模块与约束漂移"，本用例失败
    for token in co_status::ALL {
        let inserted = custom_order::ActiveModel {
            order_no: sea_orm::Set(format!("CO-W5U-{}", token)),
            customer_id: sea_orm::Set(1),
            product_id: sea_orm::Set(1),
            spec: sea_orm::Set("测试规格".to_string()),
            quantity: sea_orm::Set(dec("10.00")),
            unit: sea_orm::Set("m".to_string()),
            custom_requirements: sea_orm::Set(serde_json::json!({})),
            status: sea_orm::Set(token.to_string()),
            currency: sea_orm::Set("CNY".to_string()),
            created_at: sea_orm::Set(Utc::now()),
            updated_at: sea_orm::Set(Utc::now()),
            ..Default::default()
        }
        .insert(&db)
        .await;
        assert!(
            inserted.is_ok(),
            "权威词表 token「{token}」违反与 m0064 同构的 CHECK——模块与约束漂移"
        );
    }

    // 大额变更挂起回读 == change_pending（同构 CHECK 下写入不再违反约束）
    let seeded = seed_order(&db, "CO-W5U-CHANGE", co_status::CHANGE_PENDING).await;
    let row = custom_order::Entity::find_by_id(seeded.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.status,
        co_status::CHANGE_PENDING,
        "挂起后回读状态必须逐字符等于权威模块 CHANGE_PENDING"
    );

    // 反向锁：词表外 token 必须被同构 CHECK 显式拒绝（约束真实生效，不是摆设）
    let bogus = custom_order::ActiveModel {
        order_no: sea_orm::Set("CO-W5U-BOGUS".to_string()),
        customer_id: sea_orm::Set(1),
        product_id: sea_orm::Set(1),
        spec: sea_orm::Set("测试规格".to_string()),
        quantity: sea_orm::Set(dec("10.00")),
        unit: sea_orm::Set("m".to_string()),
        custom_requirements: sea_orm::Set(serde_json::json!({})),
        status: sea_orm::Set("pending".to_string()),
        currency: sea_orm::Set("CNY".to_string()),
        created_at: sea_orm::Set(Utc::now()),
        updated_at: sea_orm::Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await;
    assert!(
        bogus.is_err(),
        "悬空 token「pending」（历史 custom_order 模块自创、从未进任何 CHECK）必须被拒绝"
    );
}

// =========================================================
// ①-b 活库（PG，#[ignore]）：真实 service 链挂起→回读→审批闭环
//     （submit_change_request 走 lock_exclusive 事务，sqlite 方言不支持行锁）
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（submit_change_request 走 lock_exclusive + 真实 chk_custom_order_status）"]
async fn live_submit_change_request_roundtrip_on_postgres() {
    let db = test_common::setup_test_db().await;
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "本用例必须跑在已迁移的 PostgreSQL（TEST_DATABASE_URL）上；禁止 sqlite 回退假绿"
    );
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();

    // seed：已客户签字确认的订单（submit_change_request 前置门）
    let seeded = custom_order::ActiveModel {
        order_no: sea_orm::Set(format!("CO-W5U-LIVE-{suffix}")),
        customer_id: sea_orm::Set(1),
        product_id: sea_orm::Set(1),
        spec: sea_orm::Set("活库契约订单".to_string()),
        quantity: sea_orm::Set(dec("10.00")),
        unit: sea_orm::Set("m".to_string()),
        custom_requirements: sea_orm::Set(serde_json::json!({})),
        status: sea_orm::Set(co_status::DRAFT.to_string()),
        currency: sea_orm::Set("CNY".to_string()),
        total_amount: sea_orm::Set(Some(dec("100.00"))),
        customer_approved_at: sea_orm::Set(Some(Utc::now())),
        created_at: sea_orm::Set(Utc::now()),
        updated_at: sea_orm::Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("活库 seed custom_orders 行失败（检查 FK 种子数据）");

    let service = CustomOrderCrudService::new(Arc::new(db.clone()));

    // 金额变化 19900 > 10000 ⇒ 触发二级审批挂起。修复前该写入违反
    // chk_custom_order_status（10 值集不含 change_pending）必冒 500；
    // m0064 扩约束后必须成功落库。
    let updated = service
        .submit_change_request(
            seeded.id,
            UpdateCustomOrderDto {
                total_amount: Some(dec("20000.00")),
                ..Default::default()
            },
        )
        .await
        .expect("大额变更挂起不得再违反 CHECK（500）");
    assert_eq!(updated.status, co_status::CHANGE_PENDING);
    let row = custom_order::Entity::find_by_id(seeded.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.status,
        co_status::CHANGE_PENDING,
        "挂起后库内回读必须为 change_pending"
    );

    // 审批通过：change_pending → draft 回写闭环（比较点同源常量）
    service
        .approve_change(seeded.id, 1, true, None)
        .await
        .expect("approve_change 状态门按 CHANGE_PENDING 常量比较，必须放行");
    let row = custom_order::Entity::find_by_id(seeded.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, co_status::DRAFT, "审批通过后必须回到 draft");

    // 清理本测试行（仅删自己插入的行，绝不触碰存量数据）
    let txn = db.begin().await.unwrap();
    custom_order::Entity::delete_many()
        .filter(custom_order::Column::Id.eq(seeded.id))
        .exec(&txn)
        .await
        .unwrap();
    txn.commit().await.unwrap();
}

// =========================================================
// ② 源码扫描锁：crud service 状态写入/比较不得回潮裸字面量
// =========================================================

#[test]
fn crud_service_status_writes_must_use_authority_constants() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/custom_order_crud_service.rs"
    );
    let src = std::fs::read_to_string(path).expect("读取 custom_order_crud_service.rs 失败");

    // change_pending 字面量整文件禁现（写入与比较都必须走 co_status::CHANGE_PENDING）
    assert!(
        !src.contains("change_pending"),
        "custom_order_crud_service.rs 出现裸字面量 change_pending——必须引用权威模块常量"
    );

    for line in src.lines() {
        let trimmed = line.trim();
        // 状态写入行：Set("...") 直写裸字面量一律禁止（常量引用形如 Set(co_status::X…)）
        if trimmed.contains("status = Set(") || trimmed.starts_with("status: Set(") {
            assert!(
                !trimmed.contains("Set(\""),
                "状态写入行禁止 Set(\"…\") 裸字面量: {trimmed}"
            );
            assert!(
                trimmed.contains("co_status::") || trimmed.contains("node_status::"),
                "状态写入必须引用权威模块常量（custom_order 或 process_node）: {trimmed}"
            );
        }
        // 状态比较行：existing.status ==/!= "..." 直写字面量一律禁止
        if trimmed.contains(".status ==") || trimmed.contains(".status !=") {
            assert!(
                !trimmed.contains("== \"") && !trimmed.contains("!= \""),
                "状态比较行禁止与裸字面量比较: {trimmed}"
            );
        }
    }
}

// =========================================================
// ③ 词表-CHECK 双向锁：custom_order::ALL 与 m0064 CHECK 集合逐项相等
// =========================================================

/// 从迁移源文件提取 ALLOWED_STATUS_VALUES 常量内的单引号 token 列表
/// （切片范围锚定在常量声明与结构体声明之间，避开 down 中的 10 值回滚集合）
fn extract_migration_check_tokens() -> Vec<String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migration/src/domain/production/m0064_custom_order_status_add_change_pending.rs"
    );
    let src = std::fs::read_to_string(path).expect("读取 m0064 迁移源文件失败");
    let start = src
        .find("const ALLOWED_STATUS_VALUES")
        .expect("m0064 缺少 ALLOWED_STATUS_VALUES 常量");
    let end = src
        .find("#[derive(DeriveMigrationName)]")
        .expect("m0064 结构体声明锚点缺失");
    let slice = &src[start..end];

    let mut tokens = Vec::new();
    let mut rest = slice;
    while let Some(open) = rest.find('\'') {
        rest = &rest[open + 1..];
        let close = rest.find('\'').expect("单引号未闭合");
        tokens.push(rest[..close].to_string());
        rest = &rest[close + 1..];
    }
    tokens
}

#[test]
fn custom_order_all_equals_m0064_check_token_set_bidirectional() {
    let migration_tokens = extract_migration_check_tokens();
    assert_eq!(
        migration_tokens.len(),
        co_status::ALL.len(),
        "m0064 CHECK token 数量与 custom_order::ALL 不一致（先于集合比较报数，便于定位）"
    );

    // ALL ⊆ CHECK
    for token in co_status::ALL {
        assert!(
            migration_tokens.iter().any(|t| t.as_str() == *token),
            "权威模块 token「{token}」不在 m0064 CHECK 集合内：写入方将违反约束"
        );
    }
    // CHECK ⊆ ALL
    for token in &migration_tokens {
        assert!(
            co_status::ALL.contains(&token.as_str()),
            "m0064 CHECK token「{token}」不在权威模块 ALL 内：约束比词表宽即漂移"
        );
    }
}
